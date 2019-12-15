//! Different-dimensional spacing types.

use std::fmt::{self, Display, Formatter};
use std::iter::Sum;
use std::ops::*;
use std::str::FromStr;

use crate::layout::prelude::*;

/// A general spacing type.
#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct Size {
    /// The size in typographic points (1/72 inches).
    pub points: f32,
}

impl Size {
    /// The zeroed size.
    pub const ZERO: Size = Size { points: 0.0 };

    /// Create a size from an amount of points.
    pub fn pt(points: f32) -> Size { Size { points } }

    /// Create a size from an amount of millimeters.
    pub fn mm(mm: f32) -> Size { Size { points: 2.83465 * mm } }

    /// Create a size from an amount of centimeters.
    pub fn cm(cm: f32) -> Size { Size { points: 28.3465 * cm } }

    /// Create a size from an amount of inches.
    pub fn inches(inches: f32) -> Size { Size { points: 72.0 * inches } }

    /// Convert this size into points.
    pub fn to_pt(self) -> f32 { self.points }

    /// Convert this size into millimeters.
    pub fn to_mm(self) -> f32 { self.points * 0.352778 }

    /// Convert this size into centimeters.
    pub fn to_cm(self) -> f32 { self.points * 0.0352778 }

    /// Convert this size into inches.
    pub fn to_inches(self) -> f32 { self.points * 0.0138889 }

    /// The maximum of this and the other size.
    pub fn max(self, other: Size) -> Size {
        if self > other { self } else { other }
    }

    /// The minimum of this and the other size.
    pub fn min(self, other: Size) -> Size {
        if self <= other { self } else { other }
    }

    /// Set this size to the maximum of itself and the other size.
    pub fn max_eq(&mut self, other: Size) { *self = self.max(other); }

    /// Set this size to the minimum of itself and the other size.
    pub fn min_eq(&mut self, other: Size) { *self = self.min(other); }

    /// The anchor position along the given direction for an item with the given
    /// alignment in a container with this size.
    pub fn anchor(self, alignment: Alignment, direction: Direction) -> Size {
        match (direction.is_positive(), alignment) {
            (true, Origin) | (false, End) => Size::ZERO,
            (_, Center) => self / 2,
            (true, End) | (false, Origin) => self,
        }
    }
}

impl Display for Size {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}cm", self.to_cm())
    }
}

debug_display!(Size);

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

/// A position or extent in 2-dimensional space.
#[derive(Copy, Clone, PartialEq)]
pub struct Size2D {
    /// The horizontal coordinate.
    pub x: Size,
    /// The vertical coordinate.
    pub y: Size,
}

impl Size2D {
    /// The zeroed 2D-size.
    pub const ZERO: Size2D = Size2D { x: Size::ZERO, y: Size::ZERO };

    /// Create a new 2D-size from two sizes.
    pub fn new(x: Size, y: Size) -> Size2D { Size2D { x, y } }

    /// Create a new 2D-size with `x` set to a value and `y` zero.
    pub fn with_x(x: Size) -> Size2D { Size2D { x, y: Size::ZERO } }

    /// Create a new 2D-size with `y` set to a value and `x` zero.
    pub fn with_y(y: Size) -> Size2D { Size2D { x: Size::ZERO, y } }

    /// Create a 2D-size with `x` and `y` set to the same value `s`.
    pub fn with_all(s: Size) -> Size2D { Size2D { x: s, y: s } }

    /// Get the specificed component.
    pub fn get(self, axis: SpecificAxis) -> Size {
        match axis {
            Horizontal => self.x,
            Vertical => self.y,
        }
    }

    /// Get the specificed component mutably.
    pub fn get_mut(&mut self, axis: SpecificAxis) -> &mut Size {
        match axis {
            Horizontal => &mut self.x,
            Vertical => &mut self.y,
        }
    }

    /// Access the primary size of this specialized 2D-size.
    pub fn get_primary(self, axes: LayoutAxes) -> Size {
        if axes.primary.axis() == Horizontal { self.x } else { self.y }
    }

    /// Access the primary size of this specialized 2D-size mutably.
    pub fn get_primary_mut(&mut self, axes: LayoutAxes) -> &mut Size {
        if axes.primary.axis() == Horizontal { &mut self.x } else { &mut self.y }
    }

    /// Access the secondary size of this specialized 2D-size.
    pub fn get_secondary(self, axes: LayoutAxes) -> Size {
        if axes.primary.axis() == Horizontal { self.y } else { self.x }
    }

    /// Access the secondary size of this specialized 2D-size mutably.
    pub fn get_secondary_mut(&mut self, axes: LayoutAxes) -> &mut Size {
        if axes.primary.axis() == Horizontal { &mut self.y } else { &mut self.x }
    }

    /// Returns the generalized version of a `Size2D` dependent on the layouting
    /// axes, that is:
    /// - `x` describes the primary axis instead of the horizontal one.
    /// - `y` describes the secondary axis instead of the vertical one.
    pub fn generalized(self, axes: LayoutAxes) -> Size2D {
        match axes.primary.axis() {
            Horizontal => self,
            Vertical => Size2D { x: self.y, y: self.x },
        }
    }

    /// Returns the specialized version of this generalized Size2D (inverse to
    /// `generalized`).
    pub fn specialized(self, axes: LayoutAxes) -> Size2D {
        // In fact, generalized is its own inverse. For reasons of clarity
        // at the call site, we still have this second function.
        self.generalized(axes)
    }

    /// Whether the given 2D-size fits into this one, that is, both coordinate
    /// values are smaller or equal.
    pub fn fits(self, other: Size2D) -> bool {
        self.x >= other.x && self.y >= other.y
    }

    /// Return a 2D-size padded by the paddings of the given box.
    pub fn padded(self, padding: SizeBox) -> Size2D {
        Size2D {
            x: self.x + padding.left + padding.right,
            y: self.y + padding.top + padding.bottom,
        }
    }

    /// Return a 2D-size reduced by the paddings of the given box.
    pub fn unpadded(self, padding: SizeBox) -> Size2D {
        Size2D {
            x: self.x - padding.left - padding.right,
            y: self.y - padding.top - padding.bottom,
        }
    }

    /// The anchor position along the given axis for an item with the given
    /// alignment in a container with this size.
    ///
    /// This assumes the size to be generalized such that `x` corresponds to the
    /// primary axis.
    pub fn anchor(self, alignment: LayoutAlignment, axes: LayoutAxes) -> Size2D {
        Size2D {
            x: self.x.anchor(alignment.primary, axes.primary),
            y: self.y.anchor(alignment.secondary, axes.secondary),
        }
    }
}

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

/// A size in four dimensions.
pub type SizeBox = ValueBox<Size>;

/// A value in four dimensions.
#[derive(Copy, Clone, PartialEq)]
pub struct ValueBox<T: Copy> {
    /// The left extent.
    pub left: T,
    /// The top extent.
    pub top: T,
    /// The right extent.
    pub right: T,
    /// The bottom extent.
    pub bottom: T,
}

impl SizeBox {
    /// The zeroed size box.
    pub const ZERO: SizeBox = SizeBox {
        left: Size::ZERO,
        top: Size::ZERO,
        right: Size::ZERO,
        bottom: Size::ZERO,
    };
}

impl<T: Copy> ValueBox<T> {
    /// Create a new box from four sizes.
    pub fn new(left: T, top: T, right: T, bottom: T) -> ValueBox<T> {
        ValueBox { left, top, right, bottom }
    }

    /// Create a box with all four fields set to the same value `s`.
    pub fn with_all(value: T) -> ValueBox<T> {
        ValueBox { left: value, top: value, right: value, bottom: value }
    }

    /// Get a mutable reference to the value for the specified direction and
    /// alignment. Center alignment will be treated the same as origin
    /// alignment.
    pub fn get_mut(&mut self, mut direction: Direction, alignment: Alignment) -> &mut T {
        if alignment == End {
            direction = direction.inv();
        }

        match direction {
            LeftToRight => &mut self.left,
            RightToLeft => &mut self.right,
            TopToBottom => &mut self.top,
            BottomToTop => &mut self.bottom,
        }
    }

    /// Set all values to the given value.
    pub fn set_all(&mut self, value: T) {
        *self = ValueBox::with_all(value);
    }

    /// Set the `left` and `right` values.
    pub fn set_horizontal(&mut self, value: T) {
        self.left = value;
        self.right = value;
    }

    /// Set the `top` and `bottom` values.
    pub fn set_vertical(&mut self, value: T) {
        self.top = value;
        self.bottom = value;
    }
}

impl<T: Copy> Display for ValueBox<T> where T: std::fmt::Debug {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[left: {:?}, top: {:?}, right: {:?}, bottom: {:?}]",
            self.left, self.top, self.right, self.bottom)
    }
}

debug_display!(ValueBox; T where T: std::fmt::Debug + Copy);

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

impl ScaleSize {
    /// Use the absolute value or scale the entity.
    pub fn scaled(&self, entity: Size) -> Size {
        match self {
            ScaleSize::Absolute(s) => *s,
            ScaleSize::Scaled(s) => *s * entity,
        }
    }
}

impl Display for ScaleSize {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ScaleSize::Absolute(size) => write!(f, "{}", size),
            ScaleSize::Scaled(scale) => write!(f, "x{}", scale),
        }
    }
}

debug_display!(ScaleSize);

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

macro_rules! implement_traits {
    ($ty:ident, $t:ident, $o:ident
        reflexive {$(
            ($tr:ident($tf:ident), $at:ident($af:ident), [$($f:ident),*])
        )*}
        numbers { $(($w:ident: $($rest:tt)*))* }
    ) => {
        $(impl $tr for $ty {
            type Output = $ty;
            fn $tf($t, $o: $ty) -> $ty {
                $ty { $($f: $tr::$tf($t.$f, $o.$f),)* }
            }
        }

        impl $at for $ty {
            fn $af(&mut $t, $o: $ty) { $($at::$af(&mut $t.$f, $o.$f);)* }
        })*

        $(implement_traits!(@$w i32, $ty $t $o $($rest)*);)*
        $(implement_traits!(@$w f32, $ty $t $o $($rest)*);)*
    };

    (@front $num:ty, $ty:ident $t:ident $o:ident
        $tr:ident($tf:ident),
        [$($f:ident),*]
    ) => {
        impl $tr<$ty> for $num {
            type Output = $ty;
            fn $tf($t, $o: $ty) -> $ty {
                $ty { $($f: $tr::$tf($t as f32, $o.$f),)* }
            }
        }
    };

    (@back $num:ty, $ty:ident $t:ident $o:ident
        $tr:ident($tf:ident), $at:ident($af:ident),
        [$($f:ident),*]
    ) => {
        impl $tr<$num> for $ty {
            type Output = $ty;
            fn $tf($t, $o: $num) -> $ty {
                $ty { $($f: $tr::$tf($t.$f, $o as f32),)* }
            }
        }

        impl $at<$num> for $ty {
            fn $af(&mut $t, $o: $num) { $($at::$af(&mut $t.$f, $o as f32);)* }
        }
    };
}

macro_rules! implement_size {
    ($ty:ident($t:ident, $o:ident) [$($f:ident),*]) => {
        implement_traits! {
            $ty, $t, $o

            reflexive {
                (Add(add), AddAssign(add_assign), [$($f),*])
                (Sub(sub), SubAssign(sub_assign), [$($f),*])
            }

            numbers {
                (front: Mul(mul), [$($f),*])
                (back:  Mul(mul), MulAssign(mul_assign), [$($f),*])
                (back:  Div(div), DivAssign(div_assign), [$($f),*])
            }
        }
    };
}

implement_size! { Size(self, other) [points] }
implement_size! { Size2D(self, other) [x, y] }
