//! Different-dimensional value and spacing types.

use std::fmt::{self, Debug, Display, Formatter};
use std::iter::Sum;
use std::ops::*;
use std::str::FromStr;
use serde::Serialize;

use crate::layout::prelude::*;

/// A general spacing type.
#[derive(Default, Copy, Clone, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Length {
    /// The length in typographic points (1/72 inches).
    pub points: f64,
}

impl Length {
    /// The zeroed length.
    pub const ZERO: Length = Length { points: 0.0 };

    /// Create a length from an amount of points.
    pub fn pt(points: f64) -> Length { Length { points } }

    /// Create a length from an amount of millimeters.
    pub fn mm(mm: f64) -> Length { Length { points: 2.83465 * mm } }

    /// Create a length from an amount of centimeters.
    pub fn cm(cm: f64) -> Length { Length { points: 28.3465 * cm } }

    /// Create a length from an amount of inches.
    pub fn inches(inches: f64) -> Length { Length { points: 72.0 * inches } }

    /// Convert this length into points.
    pub fn to_pt(self) -> f64 { self.points }

    /// Convert this length into millimeters.
    pub fn to_mm(self) -> f64 { self.points * 0.352778 }

    /// Convert this length into centimeters.
    pub fn to_cm(self) -> f64 { self.points * 0.0352778 }

    /// Convert this length into inches.
    pub fn to_inches(self) -> f64 { self.points * 0.0138889 }

    /// The maximum of this and the other length.
    pub fn max(self, other: Length) -> Length {
        if self > other { self } else { other }
    }

    /// The minimum of this and the other length.
    pub fn min(self, other: Length) -> Length {
        if self <= other { self } else { other }
    }

    /// Set this length to the maximum of itself and the other length.
    pub fn max_eq(&mut self, other: Length) { *self = self.max(other); }

    /// Set this length to the minimum of itself and the other length.
    pub fn min_eq(&mut self, other: Length) { *self = self.min(other); }

    /// The anchor position along the given direction for an item with the given
    /// alignment in a container with this length.
    pub fn anchor(self, alignment: Alignment, direction: Direction) -> Length {
        match (direction.is_positive(), alignment) {
            (true, Origin) | (false, End) => Length::ZERO,
            (_, Center) => self / 2,
            (true, End) | (false, Origin) => self,
        }
    }
}

impl Display for Length {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}pt", self.points)
    }
}

impl Debug for Length {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Neg for Length {
    type Output = Length;

    fn neg(self) -> Length {
        Length { points: -self.points }
    }
}

impl Sum for Length {
    fn sum<I>(iter: I) -> Length
    where I: Iterator<Item = Length> {
        iter.fold(Length::ZERO, Add::add)
    }
}

/// Either an absolute length or a factor of some entity.
#[derive(Copy, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum ScaleLength {
    Absolute(Length),
    Scaled(f64),
}

impl ScaleLength {
    /// Use the absolute value or scale the entity.
    pub fn scaled(&self, entity: Length) -> Length {
        match self {
            ScaleLength::Absolute(s) => *s,
            ScaleLength::Scaled(s) => *s * entity,
        }
    }
}

impl Display for ScaleLength {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ScaleLength::Absolute(length) => write!(f, "{}", length),
            ScaleLength::Scaled(scale) => write!(f, "{}%", scale * 100.0),
        }
    }
}

impl Debug for ScaleLength {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

/// A value in two dimensions.
#[derive(Default, Copy, Clone, Eq, PartialEq, Serialize)]
pub struct Value2<T> {
    /// The horizontal component.
    pub x: T,
    /// The vertical component.
    pub y: T,
}

impl<T: Clone> Value2<T> {
    /// Create a new 2D-value from two values.
    pub fn new(x: T, y: T) -> Value2<T> { Value2 { x, y } }

    /// Create a new 2D-value with `x` set to a value and `y` to default.
    pub fn with_x(x: T) -> Value2<T> where T: Default {
        Value2 { x, y: T::default() }
    }

    /// Create a new 2D-value with `y` set to a value and `x` to default.
    pub fn with_y(y: T) -> Value2<T> where T: Default {
        Value2 { x: T::default(), y }
    }

    /// Create a new 2D-value with the primary axis set to a value and the other
    /// one to default.
    pub fn with_primary(v: T, axes: LayoutAxes) -> Value2<T> where T: Default {
        Value2::with_x(v).generalized(axes)
    }

    /// Create a new 2D-value with the secondary axis set to a value and the
    /// other one to default.
    pub fn with_secondary(v: T, axes: LayoutAxes) -> Value2<T> where T: Default {
        Value2::with_y(v).generalized(axes)
    }

    /// Create a 2D-value with `x` and `y` set to the same value `s`.
    pub fn with_all(s: T) -> Value2<T> { Value2 { x: s.clone(), y: s } }

    /// Get the specificed component.
    pub fn get(self, axis: SpecificAxis) -> T {
        match axis {
            Horizontal => self.x,
            Vertical => self.y,
        }
    }

    /// Borrow the specificed component mutably.
    pub fn get_mut(&mut self, axis: SpecificAxis) -> &mut T {
        match axis {
            Horizontal => &mut self.x,
            Vertical => &mut self.y,
        }
    }

    /// Return the primary value of this specialized 2D-value.
    pub fn primary(self, axes: LayoutAxes) -> T {
        if axes.primary.axis() == Horizontal { self.x } else { self.y }
    }

    /// Borrow the primary value of this specialized 2D-value mutably.
    pub fn primary_mut(&mut self, axes: LayoutAxes) -> &mut T {
        if axes.primary.axis() == Horizontal { &mut self.x } else { &mut self.y }
    }

    /// Return the secondary value of this specialized 2D-value.
    pub fn secondary(self, axes: LayoutAxes) -> T {
        if axes.primary.axis() == Horizontal { self.y } else { self.x }
    }

    /// Borrow the secondary value of this specialized 2D-value mutably.
    pub fn secondary_mut(&mut self, axes: LayoutAxes) -> &mut T {
        if axes.primary.axis() == Horizontal { &mut self.y } else { &mut self.x }
    }

    /// Returns the generalized version of a `Size2D` dependent on the layouting
    /// axes, that is:
    /// - `x` describes the primary axis instead of the horizontal one.
    /// - `y` describes the secondary axis instead of the vertical one.
    pub fn generalized(self, axes: LayoutAxes) -> Value2<T> {
        match axes.primary.axis() {
            Horizontal => self,
            Vertical => Value2 { x: self.y, y: self.x },
        }
    }

    /// Returns the specialized version of this generalized Size2D (inverse to
    /// `generalized`).
    pub fn specialized(self, axes: LayoutAxes) -> Value2<T> {
        // In fact, generalized is its own inverse. For reasons of clarity
        // at the call site, we still have this second function.
        self.generalized(axes)
    }

    /// Swap the `x` and `y` values.
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.x, &mut self.y);
    }
}

impl<T> Debug for Value2<T> where T: Debug {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_list()
            .entry(&self.x)
            .entry(&self.y)
            .finish()
    }
}

/// A position or extent in 2-dimensional space.
pub type Size = Value2<Length>;

impl Size {
    /// The zeroed 2D-length.
    pub const ZERO: Size = Size { x: Length::ZERO, y: Length::ZERO };

    /// Whether the given 2D-length fits into this one, that is, both coordinate
    /// values are smaller or equal.
    pub fn fits(self, other: Size) -> bool {
        self.x >= other.x && self.y >= other.y
    }

    /// Return a 2D-length padded by the paddings of the given box.
    pub fn padded(self, padding: Margins) -> Size {
        Size {
            x: self.x + padding.left + padding.right,
            y: self.y + padding.top + padding.bottom,
        }
    }

    /// Return a 2D-length reduced by the paddings of the given box.
    pub fn unpadded(self, padding: Margins) -> Size {
        Size {
            x: self.x - padding.left - padding.right,
            y: self.y - padding.top - padding.bottom,
        }
    }

    /// The anchor position along the given axis for an item with the given
    /// alignment in a container with this length.
    ///
    /// This assumes the length to be generalized such that `x` corresponds to the
    /// primary axis.
    pub fn anchor(self, alignment: LayoutAlignment, axes: LayoutAxes) -> Size {
        Size {
            x: self.x.anchor(alignment.primary, axes.primary),
            y: self.y.anchor(alignment.secondary, axes.secondary),
        }
    }
}

impl Neg for Size {
    type Output = Size;

    fn neg(self) -> Size {
        Size {
            x: -self.x,
            y: -self.y,
        }
    }
}

/// A value in four dimensions.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Serialize)]
pub struct Value4<T> {
    /// The left extent.
    pub left: T,
    /// The top extent.
    pub top: T,
    /// The right extent.
    pub right: T,
    /// The bottom extent.
    pub bottom: T,
}

impl<T: Clone> Value4<T> {
    /// Create a new box from four sizes.
    pub fn new(left: T, top: T, right: T, bottom: T) -> Value4<T> {
        Value4 { left, top, right, bottom }
    }

    /// Create a box with all four fields set to the same value `s`.
    pub fn with_all(value: T) -> Value4<T> {
        Value4 {
            left: value.clone(),
            top: value.clone(),
            right: value.clone(),
            bottom: value
        }
    }

    /// Get a mutable reference to the value for the specified direction at the
    /// alignment.
    ///
    /// Center alignment is treated the same as origin alignment.
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
        *self = Value4::with_all(value);
    }

    /// Set the `left` and `right` values.
    pub fn set_horizontal(&mut self, value: T) {
        self.left = value.clone();
        self.right = value;
    }

    /// Set the `top` and `bottom` values.
    pub fn set_vertical(&mut self, value: T) {
        self.top = value.clone();
        self.bottom = value;
    }
}

/// A length in four dimensions.
pub type Margins = Value4<Length>;

impl Margins {
    /// The zeroed length box.
    pub const ZERO: Margins = Margins {
        left: Length::ZERO,
        top: Length::ZERO,
        right: Length::ZERO,
        bottom: Length::ZERO,
    };
}

impl FromStr for Length {
    type Err = ParseLengthError;

    fn from_str(src: &str) -> Result<Length, ParseLengthError> {
        let func = match () {
            _ if src.ends_with("pt") => Length::pt,
            _ if src.ends_with("mm") => Length::mm,
            _ if src.ends_with("cm") => Length::cm,
            _ if src.ends_with("in") => Length::inches,
            _ => return Err(ParseLengthError),
        };

        Ok(func(src[..src.len() - 2]
            .parse::<f64>()
            .map_err(|_| ParseLengthError)?))

    }
}

/// An error which can be returned when parsing a length.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ParseLengthError;

impl std::error::Error for ParseLengthError {}

impl Display for ParseLengthError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("invalid string for length")
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
        $(implement_traits!(@$w f64, $ty $t $o $($rest)*);)*
    };

    (@front $num:ty, $ty:ident $t:ident $o:ident
        $tr:ident($tf:ident),
        [$($f:ident),*]
    ) => {
        impl $tr<$ty> for $num {
            type Output = $ty;
            fn $tf($t, $o: $ty) -> $ty {
                $ty { $($f: $tr::$tf($t as f64, $o.$f),)* }
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
                $ty { $($f: $tr::$tf($t.$f, $o as f64),)* }
            }
        }

        impl $at<$num> for $ty {
            fn $af(&mut $t, $o: $num) { $($at::$af(&mut $t.$f, $o as f64);)* }
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

implement_size! { Length(self, other) [points] }
implement_size! { Size(self, other) [x, y] }
