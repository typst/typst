//! Geometrical types.

use std::fmt::{self, Debug, Formatter};
use std::ops::*;

#[cfg(feature = "serialize")]
use serde::Serialize;

use crate::layout::prelude::*;

/// A value in two dimensions.
#[derive(Default, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
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
    pub fn get(self, axis: SpecAxis) -> T {
        match axis {
            Horizontal => self.x,
            Vertical => self.y,
        }
    }

    /// Borrow the specificed component mutably.
    pub fn get_mut(&mut self, axis: SpecAxis) -> &mut T {
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
pub type Size = Value2<f64>;

impl Size {
    /// The zeroed size.
    pub const ZERO: Size = Size { x: 0.0, y: 0.0 };

    /// Whether the given size fits into this one, that is, both coordinate
    /// values are smaller or equal.
    pub fn fits(self, other: Size) -> bool {
        self.x >= other.x && self.y >= other.y
    }

    /// Return a size padded by the paddings of the given box.
    pub fn padded(self, padding: Margins) -> Size {
        Size {
            x: self.x + padding.left + padding.right,
            y: self.y + padding.top + padding.bottom,
        }
    }

    /// Return a size reduced by the paddings of the given box.
    pub fn unpadded(self, padding: Margins) -> Size {
        Size {
            x: self.x - padding.left - padding.right,
            y: self.y - padding.top - padding.bottom,
        }
    }

    /// The anchor position along the given axis for an item with the given
    /// alignment in a container with this size.
    ///
    /// This assumes the size to be generalized such that `x` corresponds to the
    /// primary axis.
    pub fn anchor(self, align: LayoutAlign, axes: LayoutAxes) -> Size {
        Size {
            x: anchor(self.x, align.primary, axes.primary),
            y: anchor(self.y, align.secondary, axes.secondary),
        }
    }
}

fn anchor(length: f64, align: GenAlign, dir: Dir) -> f64 {
    match (dir.is_positive(), align) {
        (true, Start) | (false, End) => 0.0,
        (_, Center) => length / 2.0,
        (true, End) | (false, Start) => length,
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
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
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
    pub fn get_mut(&mut self, mut dir: Dir, align: GenAlign) -> &mut T {
        if align == End {
            dir = dir.inv();
        }

        match dir {
            LTT => &mut self.left,
            RTL => &mut self.right,
            TTB => &mut self.top,
            BTT => &mut self.bottom,
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
pub type Margins = Value4<f64>;

impl Margins {
    /// The zero margins.
    pub const ZERO: Margins = Margins {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };
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

implement_size! { Size(self, other) [x, y] }
