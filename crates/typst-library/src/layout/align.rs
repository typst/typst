use std::ops::Add;

use ecow::{EcoString, eco_format};

use crate::diag::{HintedStrResult, StrResult, bail};
use crate::foundations::{
    CastInfo, Content, Fold, FromValue, IntoValue, Reflect, Repr, Resolve, StyleChain,
    Value, cast, elem, func, scope, ty,
};
use crate::layout::{Abs, Axes, Axis, Dir, Side};
use crate::text::TextElem;

/// Aligns content horizontally and vertically.
///
/// # Example
/// Let's start with centering our content horizontally:
/// ```example
/// #set page(height: 120pt)
/// #set align(center)
///
/// Centered text, a sight to see \
/// In perfect balance, visually \
/// Not left nor right, it stands alone \
/// A work of art, a visual throne
/// ```
///
/// To center something vertically, use _horizon_ alignment:
/// ```example
/// #set page(height: 120pt)
/// #set align(horizon)
///
/// Vertically centered, \
/// the stage had entered, \
/// a new paragraph.
/// ```
///
/// # Combining alignments
/// You can combine two alignments with the `+` operator. Let's also only apply
/// this to one piece of content by using the function form instead of a set
/// rule:
/// ```example
/// #set page(height: 120pt)
/// Though left in the beginning ...
///
/// #align(right + bottom)[
///   ... they were right in the end, \
///   and with addition had gotten, \
///   the paragraph to the bottom!
/// ]
/// ```
///
/// # Nested alignment
/// You can use varying alignments for layout containers and the elements within
/// them. This way, you can create intricate layouts:
///
/// ```example
/// #align(center, block[
///   #set align(left)
///   Though centered together \
///   alone \
///   we \
///   are \
///   left.
/// ])
/// ```
///
/// # Alignment within the same line
/// The `align` function performs block-level alignment and thus always
/// interrupts the current paragraph. To have different alignment for parts
/// of the same line, you should use [fractional spacing]($h) instead:
///
/// ```example
/// Start #h(1fr) End
/// ```
#[elem]
pub struct AlignElem {
    /// The [alignment] along both axes.
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

/// Where to align something along an axis.
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
            Self::H(h) | Self::Both(h, _) => Some(h),
            Self::V(_) => None,
        }
    }

    /// The vertical component.
    pub const fn y(self) -> Option<VAlignment> {
        match self {
            Self::V(v) | Self::Both(_, v) => Some(v),
            Self::H(_) => None,
        }
    }

    /// Normalize the alignment to a LTR-TTB space.
    pub fn fix(self, text_dir: Dir) -> Axes<FixedAlignment> {
        Axes::new(
            self.x().unwrap_or_default().fix(text_dir),
            self.y().unwrap_or_default().fix(text_dir),
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
            (Self::H(h), Self::V(v)) | (Self::V(v), Self::H(h)) => Ok(h + v),
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
            Self::H(h) => h.repr(),
            Self::V(v) => v.repr(),
            Self::Both(h, v) => eco_format!("{} + {}", h.repr(), v.repr()),
        }
    }
}

impl Fold for Alignment {
    fn fold(self, outer: Self) -> Self {
        match (self, outer) {
            (Self::H(h), Self::V(v) | Self::Both(_, v)) => Self::Both(h, v),
            (Self::V(v), Self::H(h) | Self::Both(h, _)) => Self::Both(h, v),
            _ => self,
        }
    }
}

impl Resolve for Alignment {
    type Output = Axes<FixedAlignment>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.fix(styles.resolve(TextElem::dir))
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

/// Alignment on this axis can be fixed to an absolute direction.
pub trait FixAlignment {
    /// Resolve to the absolute alignment.
    fn fix(self, dir: Dir) -> FixedAlignment;
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
}

impl FixAlignment for HAlignment {
    fn fix(self, dir: Dir) -> FixedAlignment {
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

impl TryFrom<Alignment> for HAlignment {
    type Error = EcoString;

    fn try_from(value: Alignment) -> StrResult<Self> {
        match value {
            Alignment::H(h) => Ok(h),
            v => bail!(
                "expected `start`, `left`, `center`, `right`, or `end`, found {}",
                v.repr(),
            ),
        }
    }
}

impl Resolve for HAlignment {
    type Output = FixedAlignment;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.fix(styles.resolve(TextElem::dir))
    }
}

cast! {
    HAlignment,
    self => Alignment::H(self).into_value(),
    align: Alignment => Self::try_from(align)?,
}

/// A horizontal alignment which only allows `left`/`right` and `start`/`end`,
/// thus excluding `center`.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum OuterHAlignment {
    #[default]
    Start,
    Left,
    Right,
    End,
}

impl FixAlignment for OuterHAlignment {
    fn fix(self, dir: Dir) -> FixedAlignment {
        match (self, dir.is_positive()) {
            (Self::Start, true) | (Self::End, false) => FixedAlignment::Start,
            (Self::Left, _) => FixedAlignment::Start,
            (Self::Right, _) => FixedAlignment::End,
            (Self::End, true) | (Self::Start, false) => FixedAlignment::End,
        }
    }
}

impl Resolve for OuterHAlignment {
    type Output = FixedAlignment;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.fix(styles.resolve(TextElem::dir))
    }
}

impl From<OuterHAlignment> for HAlignment {
    fn from(value: OuterHAlignment) -> Self {
        match value {
            OuterHAlignment::Start => Self::Start,
            OuterHAlignment::Left => Self::Left,
            OuterHAlignment::Right => Self::Right,
            OuterHAlignment::End => Self::End,
        }
    }
}

impl TryFrom<Alignment> for OuterHAlignment {
    type Error = EcoString;

    fn try_from(value: Alignment) -> StrResult<Self> {
        match value {
            Alignment::H(HAlignment::Start) => Ok(Self::Start),
            Alignment::H(HAlignment::Left) => Ok(Self::Left),
            Alignment::H(HAlignment::Right) => Ok(Self::Right),
            Alignment::H(HAlignment::End) => Ok(Self::End),
            v => bail!("expected `start`, `left`, `right`, or `end`, found {}", v.repr()),
        }
    }
}

cast! {
    OuterHAlignment,
    self => HAlignment::from(self).into_value(),
    align: Alignment => Self::try_from(align)?,
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

    /// Returns the position of this alignment in a container with the given
    /// extent.
    pub fn position(self, extent: Abs) -> Abs {
        match self {
            Self::Top => Abs::zero(),
            Self::Horizon => extent / 2.0,
            Self::Bottom => extent,
        }
    }
}

impl FixAlignment for VAlignment {
    fn fix(self, _: Dir) -> FixedAlignment {
        // The vertical alignment does not depend on text direction.
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

impl Resolve for VAlignment {
    type Output = FixedAlignment;

    fn resolve(self, _: StyleChain) -> Self::Output {
        self.fix(Dir::TTB)
    }
}

impl From<VAlignment> for Alignment {
    fn from(align: VAlignment) -> Self {
        Self::V(align)
    }
}

impl TryFrom<Alignment> for VAlignment {
    type Error = EcoString;

    fn try_from(value: Alignment) -> StrResult<Self> {
        match value {
            Alignment::V(v) => Ok(v),
            v => bail!("expected `top`, `horizon`, or `bottom`, found {}", v.repr()),
        }
    }
}

cast! {
    VAlignment,
    self => Alignment::V(self).into_value(),
    align: Alignment => Self::try_from(align)?,
}

/// A vertical alignment which only allows `top` and `bottom`, thus excluding
/// `horizon`.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum OuterVAlignment {
    #[default]
    Top,
    Bottom,
}

impl FixAlignment for OuterVAlignment {
    fn fix(self, _: Dir) -> FixedAlignment {
        // The vertical alignment does not depend on text direction.
        match self {
            Self::Top => FixedAlignment::Start,
            Self::Bottom => FixedAlignment::End,
        }
    }
}

impl From<OuterVAlignment> for VAlignment {
    fn from(value: OuterVAlignment) -> Self {
        match value {
            OuterVAlignment::Top => Self::Top,
            OuterVAlignment::Bottom => Self::Bottom,
        }
    }
}

impl TryFrom<Alignment> for OuterVAlignment {
    type Error = EcoString;

    fn try_from(value: Alignment) -> StrResult<Self> {
        match value {
            Alignment::V(VAlignment::Top) => Ok(Self::Top),
            Alignment::V(VAlignment::Bottom) => Ok(Self::Bottom),
            v => bail!("expected `top` or `bottom`, found {}", v.repr()),
        }
    }
}

cast! {
    OuterVAlignment,
    self => VAlignment::from(self).into_value(),
    align: Alignment => Self::try_from(align)?,
}

/// An internal representation that combines horizontal or vertical alignments. The
/// allowed alignment positions are designated by the type parameter `H` and `V`.
///
/// This is not user-visible, but an internal type to impose type safety. For example,
/// `SpecificAlignment<HAlignment, OuterVAlignment>` does not allow vertical alignment
/// position "center", because `V = OuterVAlignment` doesn't have it.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SpecificAlignment<H, V> {
    H(H),
    V(V),
    Both(H, V),
}

impl<H, V> SpecificAlignment<H, V>
where
    H: Default + Copy + FixAlignment,
    V: Default + Copy + FixAlignment,
{
    /// The horizontal component.
    pub const fn x(self) -> Option<H> {
        match self {
            Self::H(h) | Self::Both(h, _) => Some(h),
            Self::V(_) => None,
        }
    }

    /// The vertical component.
    pub const fn y(self) -> Option<V> {
        match self {
            Self::V(v) | Self::Both(_, v) => Some(v),
            Self::H(_) => None,
        }
    }

    /// Normalize the alignment to a LTR-TTB space.
    pub fn fix(self, text_dir: Dir) -> Axes<FixedAlignment> {
        Axes::new(
            self.x().unwrap_or_default().fix(text_dir),
            self.y().unwrap_or_default().fix(text_dir),
        )
    }
}

impl<H, V> Resolve for SpecificAlignment<H, V>
where
    H: Default + Copy + FixAlignment,
    V: Default + Copy + FixAlignment,
{
    type Output = Axes<FixedAlignment>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.fix(styles.resolve(TextElem::dir))
    }
}

impl<H, V> From<SpecificAlignment<H, V>> for Alignment
where
    HAlignment: From<H>,
    VAlignment: From<V>,
{
    fn from(value: SpecificAlignment<H, V>) -> Self {
        type FromType<H, V> = SpecificAlignment<H, V>;
        match value {
            FromType::H(h) => Self::H(HAlignment::from(h)),
            FromType::V(v) => Self::V(VAlignment::from(v)),
            FromType::Both(h, v) => Self::Both(HAlignment::from(h), VAlignment::from(v)),
        }
    }
}

impl<H, V> Reflect for SpecificAlignment<H, V>
where
    H: Reflect,
    V: Reflect,
{
    fn input() -> CastInfo {
        Alignment::input()
    }

    fn output() -> CastInfo {
        Alignment::output()
    }

    fn castable(value: &Value) -> bool {
        H::castable(value) || V::castable(value)
    }
}

impl<H, V> IntoValue for SpecificAlignment<H, V>
where
    HAlignment: From<H>,
    VAlignment: From<V>,
{
    fn into_value(self) -> Value {
        Alignment::from(self).into_value()
    }
}

impl<H, V> FromValue for SpecificAlignment<H, V>
where
    H: Reflect + TryFrom<Alignment, Error = EcoString>,
    V: Reflect + TryFrom<Alignment, Error = EcoString>,
{
    fn from_value(value: Value) -> HintedStrResult<Self> {
        if Alignment::castable(&value) {
            let align = Alignment::from_value(value)?;
            let result = match align {
                Alignment::H(_) => Self::H(H::try_from(align)?),
                Alignment::V(_) => Self::V(V::try_from(align)?),
                Alignment::Both(h, v) => {
                    Self::Both(H::try_from(h.into())?, V::try_from(v.into())?)
                }
            };
            return Ok(result);
        }
        Err(Self::error(&value))
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

    /// The inverse alignment.
    pub const fn inv(self) -> Self {
        match self {
            Self::Start => Self::End,
            Self::Center => Self::Center,
            Self::End => Self::Start,
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
