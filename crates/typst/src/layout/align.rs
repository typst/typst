use std::ops::Add;

use ecow::{eco_format, EcoString};

use crate::diag::{bail, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, func, scope, ty, Content, Fold, Packed, Repr, Resolve, Show, StyleChain,
};
use crate::layout::{Abs, Axes, Axis, Dir, Side};
use crate::text::TextElem;

/// Aligns content horizontally and vertically.
///
/// # Example
/// ```example
/// #set align(center)
///
/// Centered text, a sight to see \
/// In perfect balance, visually \
/// Not left nor right, it stands alone \
/// A work of art, a visual throne
/// ```
#[elem(Show)]
pub struct AlignElem {
    /// The [alignment]($alignment) along both axes.
    ///
    /// ```example
    /// #set page(height: 6cm)
    /// #set text(lang: "ar")
    ///
    /// مثال
    /// #align(
    ///   end + horizon,
    ///   rect(inset: 12pt)[ركن]
    /// )
    /// ```
    #[positional]
    #[fold]
    #[default]
    pub alignment: Alignment,

    /// The content to align.
    #[required]
    pub body: Content,
}

impl Show for Packed<AlignElem> {
    #[typst_macros::time(name = "align", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        Ok(self
            .body()
            .clone()
            .styled(AlignElem::set_alignment(self.alignment(styles))))
    }
}

/// Where to [align]($align) something along an axis.
///
/// Possible values are:
/// - `start`: Aligns at the [start]($direction.start) of the [text
///   direction]($text.dir).
/// - `end`: Aligns at the [end]($direction.end) of the [text
///   direction]($text.dir).
/// - `left`: Align at the left.
/// - `center`: Aligns in the middle, horizontally.
/// - `right`: Aligns at the right.
/// - `top`: Aligns at the top.
/// - `horizon`: Aligns in the middle, vertically.
/// - `bottom`: Align at the bottom.
///
/// These values are available globally and also in the alignment type's scope,
/// so you can write either of the following two:
///
/// ```example
/// #align(center)[Hi]
/// #align(alignment.center)[Hi]
/// ```
///
/// # 2D alignments
/// To align along both axes at the same time, add the two alignments using the
/// `+` operator. For example, `top + right` aligns the content to the top right
/// corner.
///
/// ```example
/// #set page(height: 3cm)
/// #align(center + bottom)[Hi]
/// ```
///
/// # Fields
/// The `x` and `y` fields hold the alignment's horizontal and vertical
/// components, respectively (as yet another `alignment`). They may be `{none}`.
///
/// ```example
/// #(top + right).x \
/// #left.x \
/// #left.y (none)
/// ```
#[ty(scope)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Alignment {
    H(HAlignment),
    V(VAlignment),
    Both(HAlignment, VAlignment),
}

impl Alignment {
    /// The horizontal component.
    pub const fn x(self) -> Option<HAlignment> {
        match self {
            Self::H(x) | Self::Both(x, _) => Some(x),
            Self::V(_) => None,
        }
    }

    /// The vertical component.
    pub const fn y(self) -> Option<VAlignment> {
        match self {
            Self::V(y) | Self::Both(_, y) => Some(y),
            Self::H(_) => None,
        }
    }

    /// Normalize the alignment to a LTR-TTB space.
    pub fn fix(self, text_dir: Dir) -> Axes<FixedAlignment> {
        Axes::new(
            self.x().unwrap_or_default().fix(text_dir),
            self.y().unwrap_or_default().fix(),
        )
    }
}

#[scope]
impl Alignment {
    pub const START: Self = Alignment::H(HAlignment::Start);
    pub const LEFT: Self = Alignment::H(HAlignment::Left);
    pub const CENTER: Self = Alignment::H(HAlignment::Center);
    pub const RIGHT: Self = Alignment::H(HAlignment::Right);
    pub const END: Self = Alignment::H(HAlignment::End);
    pub const TOP: Self = Alignment::V(VAlignment::Top);
    pub const HORIZON: Self = Alignment::V(VAlignment::Horizon);
    pub const BOTTOM: Self = Alignment::V(VAlignment::Bottom);

    /// The axis this alignment belongs to.
    /// - `{"horizontal"}` for `start`, `left`, `center`, `right`, and `end`
    /// - `{"vertical"}` for `top`, `horizon`, and `bottom`
    /// - `{none}` for 2-dimensional alignments
    ///
    /// ```example
    /// #left.axis() \
    /// #bottom.axis()
    /// ```
    #[func]
    pub const fn axis(self) -> Option<Axis> {
        match self {
            Self::H(_) => Some(Axis::X),
            Self::V(_) => Some(Axis::Y),
            Self::Both(..) => None,
        }
    }

    /// The inverse alignment.
    ///
    /// ```example
    /// #top.inv() \
    /// #left.inv() \
    /// #center.inv() \
    /// #(left + bottom).inv()
    /// ```
    #[func(title = "Inverse")]
    pub const fn inv(self) -> Alignment {
        match self {
            Self::H(h) => Self::H(h.inv()),
            Self::V(v) => Self::V(v.inv()),
            Self::Both(h, v) => Self::Both(h.inv(), v.inv()),
        }
    }
}

impl Default for Alignment {
    fn default() -> Self {
        HAlignment::default() + VAlignment::default()
    }
}

impl Add for Alignment {
    type Output = StrResult<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::H(x), Self::V(y)) | (Self::V(y), Self::H(x)) => Ok(x + y),
            (Self::H(_), Self::H(_)) => bail!("cannot add two horizontal alignments"),
            (Self::V(_), Self::V(_)) => bail!("cannot add two vertical alignments"),
            (Self::H(_), Self::Both(..)) | (Self::Both(..), Self::H(_)) => {
                bail!("cannot add a horizontal and a 2D alignment")
            }
            (Self::V(_), Self::Both(..)) | (Self::Both(..), Self::V(_)) => {
                bail!("cannot add a vertical and a 2D alignment")
            }
            (Self::Both(..), Self::Both(..)) => {
                bail!("cannot add two 2D alignments")
            }
        }
    }
}

impl Repr for Alignment {
    fn repr(&self) -> EcoString {
        match self {
            Self::H(x) => x.repr(),
            Self::V(y) => y.repr(),
            Self::Both(x, y) => eco_format!("{} + {}", x.repr(), y.repr()),
        }
    }
}

impl Fold for Alignment {
    fn fold(self, outer: Self) -> Self {
        match (self, outer) {
            (Self::H(x), Self::V(y) | Self::Both(_, y)) => Self::Both(x, y),
            (Self::V(y), Self::H(x) | Self::Both(x, _)) => Self::Both(x, y),
            _ => self,
        }
    }
}

impl Resolve for Alignment {
    type Output = Axes<FixedAlignment>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.fix(TextElem::dir_in(styles))
    }
}

impl From<Side> for Alignment {
    fn from(side: Side) -> Self {
        match side {
            Side::Left => Self::LEFT,
            Side::Top => Self::TOP,
            Side::Right => Self::RIGHT,
            Side::Bottom => Self::BOTTOM,
        }
    }
}

/// Where to align something horizontally.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum HAlignment {
    #[default]
    Start,
    Left,
    Center,
    Right,
    End,
}

impl HAlignment {
    /// The inverse horizontal alignment.
    pub const fn inv(self) -> Self {
        match self {
            Self::Start => Self::End,
            Self::Left => Self::Right,
            Self::Center => Self::Center,
            Self::Right => Self::Left,
            Self::End => Self::Start,
        }
    }

    /// Resolve the axis alignment based on the horizontal direction.
    pub const fn fix(self, dir: Dir) -> FixedAlignment {
        match (self, dir.is_positive()) {
            (Self::Start, true) | (Self::End, false) => FixedAlignment::Start,
            (Self::Left, _) => FixedAlignment::Start,
            (Self::Center, _) => FixedAlignment::Center,
            (Self::Right, _) => FixedAlignment::End,
            (Self::End, true) | (Self::Start, false) => FixedAlignment::End,
        }
    }
}

impl Repr for HAlignment {
    fn repr(&self) -> EcoString {
        match self {
            Self::Start => "start".into(),
            Self::Left => "left".into(),
            Self::Center => "center".into(),
            Self::Right => "right".into(),
            Self::End => "end".into(),
        }
    }
}

impl Add<VAlignment> for HAlignment {
    type Output = Alignment;

    fn add(self, rhs: VAlignment) -> Self::Output {
        Alignment::Both(self, rhs)
    }
}

impl From<HAlignment> for Alignment {
    fn from(align: HAlignment) -> Self {
        Self::H(align)
    }
}

impl Resolve for HAlignment {
    type Output = FixedAlignment;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.fix(TextElem::dir_in(styles))
    }
}

cast! {
    HAlignment,
    self => Alignment::H(self).into_value(),
    align: Alignment => match align {
        Alignment::H(v) => v,
        v => bail!("expected `start`, `left`, `center`, `right`, or `end`, found {}", v.repr()),
    }
}

/// Where to align something vertically.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum VAlignment {
    #[default]
    Top,
    Horizon,
    Bottom,
}

impl VAlignment {
    /// The inverse vertical alignment.
    pub const fn inv(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Horizon => Self::Horizon,
            Self::Bottom => Self::Top,
        }
    }

    /// Turns into a fixed alignment.
    pub const fn fix(self) -> FixedAlignment {
        match self {
            Self::Top => FixedAlignment::Start,
            Self::Horizon => FixedAlignment::Center,
            Self::Bottom => FixedAlignment::End,
        }
    }
}

impl Repr for VAlignment {
    fn repr(&self) -> EcoString {
        match self {
            Self::Top => "top".into(),
            Self::Horizon => "horizon".into(),
            Self::Bottom => "bottom".into(),
        }
    }
}

impl Add<HAlignment> for VAlignment {
    type Output = Alignment;

    fn add(self, rhs: HAlignment) -> Self::Output {
        Alignment::Both(rhs, self)
    }
}

impl From<VAlignment> for Alignment {
    fn from(align: VAlignment) -> Self {
        Self::V(align)
    }
}

cast! {
    VAlignment,
    self => Alignment::V(self).into_value(),
    align: Alignment => match align {
        Alignment::V(v) => v,
        v => bail!("expected `top`, `horizon`, or `bottom`, found {}", v.repr()),
    }
}

/// A fixed alignment in the global coordinate space.
///
/// For horizontal alignment, start is globally left and for vertical alignment
/// it is globally top.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FixedAlignment {
    Start,
    Center,
    End,
}

impl FixedAlignment {
    /// Returns the position of this alignment in a container with the given
    /// extent.
    pub fn position(self, extent: Abs) -> Abs {
        match self {
            Self::Start => Abs::zero(),
            Self::Center => extent / 2.0,
            Self::End => extent,
        }
    }
}

impl From<Side> for FixedAlignment {
    fn from(side: Side) -> Self {
        match side {
            Side::Left => Self::Start,
            Side::Top => Self::Start,
            Side::Right => Self::End,
            Side::Bottom => Self::End,
        }
    }
}
