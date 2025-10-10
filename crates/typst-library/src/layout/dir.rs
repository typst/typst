use ecow::EcoString;

use crate::foundations::{Repr, func, scope, ty};
use crate::layout::{Axis, Side};

/// The four directions into which content can be laid out.
///
///  Possible values are:
/// - `{ltr}`: Left to right.
/// - `{rtl}`: Right to left.
/// - `{ttb}`: Top to bottom.
/// - `{btt}`: Bottom to top.
///
/// These values are available globally and
/// also in the direction type's scope, so you can write either of the following
/// two:
/// ```example
/// #stack(dir: rtl)[A][B][C]
/// #stack(dir: direction.rtl)[A][B][C]
/// ```
#[ty(scope, name = "direction")]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Dir {
    /// Left to right.
    LTR,
    /// Right to left.
    RTL,
    /// Top to bottom.
    TTB,
    /// Bottom to top.
    BTT,
}

impl Dir {
    /// Whether this direction points into the positive coordinate direction.
    ///
    /// The positive directions are left-to-right and top-to-bottom.
    pub const fn is_positive(self) -> bool {
        match self {
            Self::LTR | Self::TTB => true,
            Self::RTL | Self::BTT => false,
        }
    }
}

#[scope]
impl Dir {
    pub const LTR: Self = Self::LTR;
    pub const RTL: Self = Self::RTL;
    pub const TTB: Self = Self::TTB;
    pub const BTT: Self = Self::BTT;

    /// Returns a direction from a starting point.
    ///
    /// ```example
    /// #direction.from(left) \
    /// #direction.from(right) \
    /// #direction.from(top) \
    /// #direction.from(bottom)
    /// ```
    #[func]
    pub const fn from(side: Side) -> Dir {
        match side {
            Side::Left => Self::LTR,
            Side::Right => Self::RTL,
            Side::Top => Self::TTB,
            Side::Bottom => Self::BTT,
        }
    }

    /// Returns a direction from an end point.
    ///
    /// ```example
    /// #direction.to(left) \
    /// #direction.to(right) \
    /// #direction.to(top) \
    /// #direction.to(bottom)
    /// ```
    #[func]
    pub const fn to(side: Side) -> Dir {
        match side {
            Side::Right => Self::LTR,
            Side::Left => Self::RTL,
            Side::Bottom => Self::TTB,
            Side::Top => Self::BTT,
        }
    }

    /// The axis this direction belongs to, either `{"horizontal"}` or
    /// `{"vertical"}`.
    ///
    /// ```example
    /// #ltr.axis() \
    /// #ttb.axis()
    /// ```
    #[func]
    pub const fn axis(self) -> Axis {
        match self {
            Self::LTR | Self::RTL => Axis::X,
            Self::TTB | Self::BTT => Axis::Y,
        }
    }

    /// The corresponding sign, for use in calculations.
    ///
    /// ```example
    /// #ltr.sign() \
    /// #rtl.sign() \
    /// #ttb.sign() \
    /// #btt.sign()
    /// ```
    #[func]
    pub const fn sign(self) -> i64 {
        match self {
            Self::LTR | Self::TTB => 1,
            Self::RTL | Self::BTT => -1,
        }
    }

    /// The start point of this direction, as an alignment.
    ///
    /// ```example
    /// #ltr.start() \
    /// #rtl.start() \
    /// #ttb.start() \
    /// #btt.start()
    /// ```
    #[func]
    pub const fn start(self) -> Side {
        match self {
            Self::LTR => Side::Left,
            Self::RTL => Side::Right,
            Self::TTB => Side::Top,
            Self::BTT => Side::Bottom,
        }
    }

    /// The end point of this direction, as an alignment.
    ///
    /// ```example
    /// #ltr.end() \
    /// #rtl.end() \
    /// #ttb.end() \
    /// #btt.end()
    /// ```
    #[func]
    pub const fn end(self) -> Side {
        match self {
            Self::LTR => Side::Right,
            Self::RTL => Side::Left,
            Self::TTB => Side::Bottom,
            Self::BTT => Side::Top,
        }
    }

    /// The inverse direction.
    ///
    /// ```example
    /// #ltr.inv() \
    /// #rtl.inv() \
    /// #ttb.inv() \
    /// #btt.inv()
    /// ```
    #[func(title = "Inverse")]
    pub const fn inv(self) -> Dir {
        match self {
            Self::LTR => Self::RTL,
            Self::RTL => Self::LTR,
            Self::TTB => Self::BTT,
            Self::BTT => Self::TTB,
        }
    }
}

impl Repr for Dir {
    fn repr(&self) -> EcoString {
        match self {
            Self::LTR => "ltr".into(),
            Self::RTL => "rtl".into(),
            Self::TTB => "ttb".into(),
            Self::BTT => "btt".into(),
        }
    }
}
