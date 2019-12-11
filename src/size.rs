//! Different-dimensional spacing types.

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::iter::Sum;
use std::ops::*;
use std::str::FromStr;

use crate::layout::{LayoutAxes, LayoutAlignment, Axis, Alignment};

/// A general space type.
#[derive(Copy, Clone, PartialEq)]
pub struct Size {
    /// The size in typographic points (1/72 inches).
    points: f32,
}

/// A position or extent in 2-dimensional space.
#[derive(Copy, Clone, PartialEq)]
pub struct Size2D {
    /// The horizontal coordinate.
    pub x: Size,
    /// The vertical coordinate.
    pub y: Size,
}

/// A size in four directions.
#[derive(Copy, Clone, PartialEq)]
pub struct SizeBox {
    /// The left extent.
    pub left: Size,
    /// The top extent.
    pub top: Size,
    /// The right extent.
    pub right: Size,
    /// The bottom extent.
    pub bottom: Size,
}

/// Either an absolute size or a factor of some metric.
#[derive(Copy, Clone, PartialEq)]
pub enum ScaleSize {
    Absolute(Size),
    Scaled(f32),
}

/// A scale size that is scaled by the font size.
pub type FSize = ScaleSize;

/// A scale size that is scaled by the size of the padded parent container.
pub type PSize = ScaleSize;

impl Size {
    /// The zeroed size.
    pub const ZERO: Size = Size { points: 0.0 };

    /// Create a zeroed size.
    pub fn zero() -> Size { Size::ZERO }

    /// Create a size from an amount of points.
    pub fn pt(points: f32) -> Size { Size { points } }

    /// Create a size from an amount of millimeters.
    pub fn mm(mm: f32) -> Size { Size { points: 2.83465 * mm } }

    /// Create a size from an amount of centimeters.
    pub fn cm(cm: f32) -> Size { Size { points: 28.3465 * cm } }

    /// Create a size from an amount of inches.
    pub fn inches(inches: f32) -> Size { Size { points: 72.0 * inches } }

    /// Convert this size into points.
    pub fn to_pt(&self) -> f32 { self.points }

    /// Convert this size into millimeters.
    pub fn to_mm(&self) -> f32 { self.points * 0.352778 }

    /// Convert this size into centimeters.
    pub fn to_cm(&self) -> f32 { self.points * 0.0352778 }

    /// Convert this size into inches.
    pub fn to_inches(&self) -> f32 { self.points * 0.0138889 }

    /// Set this size to the maximum of itself and the other size.
    pub fn max_eq(&mut self, other: Size) {
        *self = max(*self, other);
    }

    /// Set this size to the minimum of itself and the other size.
    pub fn min_eq(&mut self, other: Size) {
        *self = min(*self, other);
    }

    /// The anchor position along the given axis for an item with the given
    /// alignment in a container with this size.
    pub fn anchor(&self, alignment: Alignment, axis: Axis) -> Size {
        use Alignment::*;
        match (axis.is_positive(), alignment) {
            (true, Origin) | (false, End) => Size::ZERO,
            (_, Center) => *self / 2,
            (true, End) | (false, Origin) => *self,
        }
    }
}

impl Size2D {
    /// The zeroed 2D-size.
    pub const ZERO: Size2D = Size2D { x: Size::ZERO, y: Size::ZERO };

    /// Create a new 2D-size from two sizes.
    pub fn new(x: Size, y: Size) -> Size2D {
        Size2D { x, y }
    }

    /// Create a 2D-size with both sizes set to zero.
    pub fn zero() -> Size2D {
        Size2D::ZERO
    }

    /// Create a 2D-size with `x` and `y` set to the same value `s`.
    pub fn with_all(s: Size) -> Size2D {
        Size2D { x: s, y: s }
    }

    /// Create a new 2D-size with `x` set to a value and `y` zero.
    pub fn with_x(x: Size) -> Size2D {
        Size2D { x, y: Size::ZERO }
    }

    /// Create a new 2D-size with `y` set to a value and `x` zero.
    pub fn with_y(y: Size) -> Size2D {
        Size2D { x: Size::ZERO, y }
    }

    /// Access the primary size of this 2D-size.
    pub fn primary(&self, axes: LayoutAxes) -> Size {
        match axes.primary.is_horizontal() {
            true => self.x,
            false => self.y,
        }
    }

    /// Access the secondary size of this 2D-size.
    pub fn secondary(&self, axes: LayoutAxes) -> Size {
        match axes.primary.is_horizontal() {
            true => self.y,
            false => self.x,
        }
    }

    /// Access the primary size of this 2D-size.
    pub fn primary_mut(&mut self, axes: LayoutAxes) -> &mut Size {
        match axes.primary.is_horizontal() {
            true => &mut self.x,
            false => &mut self.y,
        }
    }

    /// Access the secondary size of this 2D-size.
    pub fn secondary_mut(&mut self, axes: LayoutAxes) -> &mut Size {
        match axes.primary.is_horizontal() {
            true => &mut self.y,
            false => &mut self.x,
        }
    }

    /// Returns the generalized version of a `Size2D` dependent on
    /// the layouting axes, that is:
    /// - The x coordinate describes the primary axis instead of the horizontal one.
    /// - The y coordinate describes the secondary axis instead of the vertical one.
    pub fn generalized(&self, axes: LayoutAxes) -> Size2D {
        match axes.primary.is_horizontal() {
            true => *self,
            false => Size2D { x: self.y, y: self.x },
        }
    }

    /// Returns the specialized version of this generalized Size2D.
    /// (Inverse to `generalized`).
    pub fn specialized(&self, axes: LayoutAxes) -> Size2D {
        // In fact, generalized is its own inverse. For reasons of clarity
        // at the call site, we still have this second function.
        self.generalized(axes)
    }

    /// Return a 2D-size padded by the paddings of the given box.
    pub fn padded(&self, padding: SizeBox) -> Size2D {
        Size2D {
            x: self.x + padding.left + padding.right,
            y: self.y + padding.top + padding.bottom,
        }
    }

    /// Return a 2D-size reduced by the paddings of the given box.
    pub fn unpadded(&self, padding: SizeBox) -> Size2D {
        Size2D {
            x: self.x - padding.left - padding.right,
            y: self.y - padding.top - padding.bottom,
        }
    }

    /// Whether the given 2D-size fits into this one, that is,
    /// both coordinate values are smaller or equal.
    pub fn fits(&self, other: Size2D) -> bool {
        self.x >= other.x && self.y >= other.y
    }

    /// Set this size to the maximum of itself and the other size
    /// (for both dimensions).
    pub fn max_eq(&mut self, other: Size2D) {
        self.x.max_eq(other.x);
        self.y.max_eq(other.y);
    }

    /// Set this size to the minimum of itself and the other size
    /// (for both dimensions).
    pub fn min_eq(&mut self, other: Size2D) {
        self.x.min_eq(other.x);
        self.y.min_eq(other.y);
    }

    /// The anchor position along the given axis for an item with the given
    /// alignment in a container with this size.
    ///
    /// This assumes the size to be generalized such that `x` corresponds to the
    /// primary axis.
    pub fn anchor(&self, alignment: LayoutAlignment, axes: LayoutAxes) -> Size2D {
        Size2D {
            x: self.x.anchor(alignment.primary, axes.primary),
            y: self.y.anchor(alignment.secondary, axes.secondary),
        }
    }
}

impl SizeBox {
    /// The zeroed size box.
    pub const ZERO: SizeBox = SizeBox {
        left: Size::ZERO,
        top: Size::ZERO,
        right: Size::ZERO,
        bottom: Size::ZERO,
    };

    /// Create a new box from four sizes.
    pub fn new(left: Size, top: Size, right: Size, bottom: Size) -> SizeBox {
        SizeBox {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Create a box with all values set to zero.
    pub fn zero() -> SizeBox {
        SizeBox::ZERO
    }

    /// Create a box with all four fields set to the same value `s`.
    pub fn with_all(value: Size) -> SizeBox {
        SizeBox { left: value, top: value, right: value, bottom: value }
    }

    /// Access the origin direction on the secondary axis of this box.
    pub fn secondary_origin_mut(&mut self, axes: LayoutAxes) -> &mut Size {
        match axes.secondary {
            Axis::LeftToRight => &mut self.left,
            Axis::RightToLeft => &mut self.right,
            Axis::TopToBottom => &mut self.top,
            Axis::BottomToTop => &mut self.bottom,
        }
    }

    /// Access the end direction on the secondary axis of this box.
    pub fn secondary_end_mut(&mut self, axes: LayoutAxes) -> &mut Size {
        match axes.secondary {
            Axis::LeftToRight => &mut self.right,
            Axis::RightToLeft => &mut self.left,
            Axis::TopToBottom => &mut self.bottom,
            Axis::BottomToTop => &mut self.top,
        }
    }

    /// Set the `left` and `right` values.
    pub fn set_all(&mut self, value: Size) {
        *self = SizeBox::with_all(value);
    }

    /// Set the `left` and `right` values.
    pub fn set_horizontal(&mut self, value: Size) {
        self.left = value;
        self.right = value;
    }

    /// Set the `top` and `bottom` values.
    pub fn set_vertical(&mut self, value: Size) {
        self.top = value;
        self.bottom = value;
    }
}

impl ScaleSize {
    /// Use the absolute value or scale the entity.
    pub fn concretize(&self, entity: Size) -> Size {
        match self {
            ScaleSize::Absolute(s) => *s,
            ScaleSize::Scaled(s) => *s * entity,
        }
    }
}

/// The maximum of two sizes.
pub fn max(a: Size, b: Size) -> Size {
    if a >= b { a } else { b }
}

/// The minimum of two sizes.
pub fn min(a: Size, b: Size) -> Size {
    if a <= b { a } else { b }
}

//------------------------------------------------------------------------------------------------//

impl Display for Size {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}cm", self.to_cm())
    }
}

debug_display!(Size);

/// An error which can be returned when parsing a size.
pub struct ParseSizeError;

error_type! {
    self: ParseSizeError,
    show: f => write!(f, "failed to parse size"),
}

impl FromStr for Size {
    type Err = ParseSizeError;

    fn from_str(src: &str) -> Result<Size, ParseSizeError> {
        if src.len() < 2 {
            return Err(ParseSizeError);
        }

        let value = src[..src.len() - 2]
            .parse::<f32>()
            .map_err(|_| ParseSizeError)?;

        Ok(match &src[src.len() - 2..] {
            "pt" => Size::pt(value),
            "mm" => Size::mm(value),
            "cm" => Size::cm(value),
            "in" => Size::inches(value),
            _ => return Err(ParseSizeError),
        })
    }
}

impl PartialOrd for Size {
    fn partial_cmp(&self, other: &Size) -> Option<Ordering> {
        self.points.partial_cmp(&other.points)
    }
}

impl Neg for Size {
    type Output = Size;

    fn neg(self) -> Size {
        Size { points: -self.points }
    }
}

impl Sum for Size {
    fn sum<I>(iter: I) -> Size
    where I: Iterator<Item = Size> {
        iter.fold(Size::ZERO, Add::add)
    }
}

macro_rules! impl_reflexive {
    ($trait:ident, $func:ident, $assign_trait:ident, $assign_func:ident) => (
        impl $trait for Size {
            type Output = Size;

            fn $func(self, other: Size) -> Size {
                Size { points: $trait::$func(self.points, other.points) }
            }
        }

        impl $assign_trait for Size {
            fn $assign_func(&mut self, other: Size) {
                $assign_trait::$assign_func(&mut self.points, other.points);
            }
        }
    );
}

macro_rules! impl_num_back {
    ($trait:ident, $func:ident, $assign_trait:ident, $assign_func:ident, $ty:ty) => (
        impl $trait<$ty> for Size {
            type Output = Size;

            fn $func(self, other: $ty) -> Size {
                Size { points: $trait::$func(self.points, other as f32) }
            }
        }

        impl $assign_trait<$ty> for Size {
            fn $assign_func(&mut self, other: $ty) {
                $assign_trait::$assign_func(&mut self.points, other as f32);
            }
        }
    );
}

macro_rules! impl_num_both {
    ($trait:ident, $func:ident, $assign_trait:ident, $assign_func:ident, $ty:ty) => (
        impl_num_back!($trait, $func, $assign_trait, $assign_func, $ty);

        impl $trait<Size> for $ty {
            type Output = Size;

            fn $func(self, other: Size) -> Size {
                Size { points: $trait::$func(self as f32, other.points) }
            }
        }
    );
}

impl_reflexive!(Add, add, AddAssign, add_assign);
impl_reflexive!(Sub, sub, SubAssign, sub_assign);
impl_num_both!(Mul, mul, MulAssign, mul_assign, f32);
impl_num_both!(Mul, mul, MulAssign, mul_assign, i32);
impl_num_back!(Div, div, DivAssign, div_assign, f32);
impl_num_back!(Div, div, DivAssign, div_assign, i32);

//------------------------------------------------------------------------------------------------//

impl Display for Size2D {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[{}, {}]", self.x, self.y)
    }
}

debug_display!(Size2D);

impl Neg for Size2D {
    type Output = Size2D;

    fn neg(self) -> Size2D {
        Size2D {
            x: -self.x,
            y: -self.y,
        }
    }
}

macro_rules! impl_reflexive2d {
    ($trait:ident, $func:ident, $assign_trait:ident, $assign_func:ident) => (
        impl $trait for Size2D {
            type Output = Size2D;

            fn $func(self, other: Size2D) -> Size2D {
                Size2D {
                    x: $trait::$func(self.x, other.x),
                    y: $trait::$func(self.y, other.y),
                }
            }
        }

        impl $assign_trait for Size2D {
            fn $assign_func(&mut self, other: Size2D) {
                $assign_trait::$assign_func(&mut self.x, other.x);
                $assign_trait::$assign_func(&mut self.y, other.y);
            }
        }
    );
}

macro_rules! impl_num_back2d {
    ($trait:ident, $func:ident, $assign_trait:ident, $assign_func:ident, $ty:ty) => (
        impl $trait<$ty> for Size2D {
            type Output = Size2D;

            fn $func(self, other: $ty) -> Size2D {
                Size2D {
                    x: $trait::$func(self.x, other as f32),
                    y: $trait::$func(self.y, other as f32),
                }
            }
        }

        impl $assign_trait<$ty> for Size2D {
            fn $assign_func(&mut self, other: $ty) {
                $assign_trait::$assign_func(&mut self.x, other as f32);
                $assign_trait::$assign_func(&mut self.y, other as f32);
            }
        }
    );
}

macro_rules! impl_num_both2d {
    ($trait:ident, $func:ident, $assign_trait:ident, $assign_func:ident, $ty:ty) => (
        impl_num_back2d!($trait, $func, $assign_trait, $assign_func, $ty);

        impl $trait<Size2D> for $ty {
            type Output = Size2D;

            fn $func(self, other: Size2D) -> Size2D {
                Size2D {
                    x: $trait::$func(self as f32, other.x),
                    y: $trait::$func(self as f32, other.y),
                }
            }
        }
    );
}

impl_reflexive2d!(Add, add, AddAssign, add_assign);
impl_reflexive2d!(Sub, sub, SubAssign, sub_assign);
impl_num_both2d!(Mul, mul, MulAssign, mul_assign, f32);
impl_num_both2d!(Mul, mul, MulAssign, mul_assign, i32);
impl_num_back2d!(Div, div, DivAssign, div_assign, f32);
impl_num_back2d!(Div, div, DivAssign, div_assign, i32);

//------------------------------------------------------------------------------------------------//

impl Display for SizeBox {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[left: {}, top: {}, right: {}, bottom: {}]",
            self.left, self.top, self.right, self.bottom)
    }
}

debug_display!(SizeBox);

//------------------------------------------------------------------------------------------------//

impl Display for ScaleSize {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ScaleSize::Absolute(size) => write!(f, "{}", size),
            ScaleSize::Scaled(scale) => write!(f, "x{}", scale),
        }
    }
}

debug_display!(ScaleSize);
