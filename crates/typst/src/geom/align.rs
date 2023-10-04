use super::*;

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
#[ty(scope, name = "alignment")]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Align {
    H(HAlign),
    V(VAlign),
    Both(HAlign, VAlign),
}

impl Align {
    /// The horizontal component.
    pub const fn x(self) -> Option<HAlign> {
        match self {
            Self::H(x) | Self::Both(x, _) => Some(x),
            Self::V(_) => None,
        }
    }

    /// The vertical component.
    pub const fn y(self) -> Option<VAlign> {
        match self {
            Self::V(y) | Self::Both(_, y) => Some(y),
            Self::H(_) => None,
        }
    }

    /// Normalize the alignment to a LTR-TTB space.
    pub fn fix(self, text_dir: Dir) -> Axes<FixedAlign> {
        Axes::new(
            self.x().unwrap_or_default().fix(text_dir),
            self.y().unwrap_or_default().fix(),
        )
    }
}

#[scope]
impl Align {
    pub const START: Self = Align::H(HAlign::Start);
    pub const LEFT: Self = Align::H(HAlign::Left);
    pub const CENTER: Self = Align::H(HAlign::Center);
    pub const RIGHT: Self = Align::H(HAlign::Right);
    pub const END: Self = Align::H(HAlign::End);
    pub const TOP: Self = Align::V(VAlign::Top);
    pub const HORIZON: Self = Align::V(VAlign::Horizon);
    pub const BOTTOM: Self = Align::V(VAlign::Bottom);

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
    pub const fn inv(self) -> Align {
        match self {
            Self::H(h) => Self::H(h.inv()),
            Self::V(v) => Self::V(v.inv()),
            Self::Both(h, v) => Self::Both(h.inv(), v.inv()),
        }
    }
}

impl Default for Align {
    fn default() -> Self {
        HAlign::default() + VAlign::default()
    }
}

impl Add for Align {
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

impl Repr for Align {
    fn repr(&self) -> EcoString {
        match self {
            Self::H(x) => x.repr(),
            Self::V(y) => y.repr(),
            Self::Both(x, y) => eco_format!("{} + {}", x.repr(), y.repr()),
        }
    }
}

impl Fold for Align {
    type Output = Self;

    fn fold(self, outer: Self::Output) -> Self::Output {
        match (self, outer) {
            (Self::H(x), Self::V(y) | Self::Both(_, y)) => Self::Both(x, y),
            (Self::V(y), Self::H(x) | Self::Both(x, _)) => Self::Both(x, y),
            _ => self,
        }
    }
}

impl Resolve for Align {
    type Output = Axes<FixedAlign>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.fix(item!(dir)(styles))
    }
}

impl From<Side> for Align {
    fn from(side: Side) -> Self {
        match side {
            Side::Left => Self::LEFT,
            Side::Top => Self::TOP,
            Side::Right => Self::RIGHT,
            Side::Bottom => Self::BOTTOM,
        }
    }
}

cast! {
    type Align,
}

/// Where to align something horizontally.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum HAlign {
    #[default]
    Start,
    Left,
    Center,
    Right,
    End,
}

impl HAlign {
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
    pub const fn fix(self, dir: Dir) -> FixedAlign {
        match (self, dir.is_positive()) {
            (Self::Start, true) | (Self::End, false) => FixedAlign::Start,
            (Self::Left, _) => FixedAlign::Start,
            (Self::Center, _) => FixedAlign::Center,
            (Self::Right, _) => FixedAlign::End,
            (Self::End, true) | (Self::Start, false) => FixedAlign::End,
        }
    }
}

impl Repr for HAlign {
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

impl Add<VAlign> for HAlign {
    type Output = Align;

    fn add(self, rhs: VAlign) -> Self::Output {
        Align::Both(self, rhs)
    }
}

impl From<HAlign> for Align {
    fn from(align: HAlign) -> Self {
        Self::H(align)
    }
}

impl Resolve for HAlign {
    type Output = FixedAlign;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.fix(item!(dir)(styles))
    }
}

cast! {
    HAlign,
    self => Align::H(self).into_value(),
    align: Align => match align {
        Align::H(v) => v,
        v => bail!("expected `start`, `left`, `center`, `right`, or `end`, found {}", v.repr()),
    }
}

/// Where to align something vertically.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum VAlign {
    #[default]
    Top,
    Horizon,
    Bottom,
}

impl VAlign {
    /// The inverse vertical alignment.
    pub const fn inv(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Horizon => Self::Horizon,
            Self::Bottom => Self::Top,
        }
    }

    /// Turns into a fixed alignment.
    pub const fn fix(self) -> FixedAlign {
        match self {
            Self::Top => FixedAlign::Start,
            Self::Horizon => FixedAlign::Center,
            Self::Bottom => FixedAlign::End,
        }
    }
}

impl Repr for VAlign {
    fn repr(&self) -> EcoString {
        match self {
            Self::Top => "top".into(),
            Self::Horizon => "horizon".into(),
            Self::Bottom => "bottom".into(),
        }
    }
}

impl Add<HAlign> for VAlign {
    type Output = Align;

    fn add(self, rhs: HAlign) -> Self::Output {
        Align::Both(rhs, self)
    }
}

impl From<VAlign> for Align {
    fn from(align: VAlign) -> Self {
        Self::V(align)
    }
}

cast! {
    VAlign,
    self => Align::V(self).into_value(),
    align: Align => match align {
        Align::V(v) => v,
        v => bail!("expected `top`, `horizon`, or `bottom`, found {}", v.repr()),
    }
}

/// A fixed alignment in the global coordinate space.
///
/// For horizontal alignment, start is globally left and for vertical alignment
/// it is globally top.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FixedAlign {
    Start,
    Center,
    End,
}

impl FixedAlign {
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

impl From<Side> for FixedAlign {
    fn from(side: Side) -> Self {
        match side {
            Side::Left => Self::Start,
            Side::Top => Self::Start,
            Side::Right => Self::End,
            Side::Bottom => Self::End,
        }
    }
}
