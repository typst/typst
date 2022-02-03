use std::any::Any;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use super::*;

/// A container with a horizontal and vertical component.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Spec<T> {
    /// The horizontal component.
    pub x: T,
    /// The vertical component.
    pub y: T,
}

impl<T> Spec<T> {
    /// Create a new instance from the two components.
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    /// Create a new instance with two equal components.
    pub fn splat(v: T) -> Self
    where
        T: Clone,
    {
        Self { x: v.clone(), y: v }
    }

    /// Maps the individual fields with `f`.
    pub fn map<F, U>(self, mut f: F) -> Spec<U>
    where
        F: FnMut(T) -> U,
    {
        Spec { x: f(self.x), y: f(self.y) }
    }

    /// Convert from `&Spec<T>` to `Spec<&T>`.
    pub fn as_ref(&self) -> Spec<&T> {
        Spec { x: &self.x, y: &self.y }
    }

    /// Convert from `&Spec<T>` to `Spec<&<T as Deref>::Target>`.
    pub fn as_deref(&self) -> Spec<&T::Target>
    where
        T: Deref,
    {
        Spec { x: &self.x, y: &self.y }
    }

    /// Convert from `&mut Spec<T>` to `Spec<&mut T>`.
    pub fn as_mut(&mut self) -> Spec<&mut T> {
        Spec { x: &mut self.x, y: &mut self.y }
    }

    /// Zip two instances into an instance over a tuple.
    pub fn zip<U>(self, other: Spec<U>) -> Spec<(T, U)> {
        Spec {
            x: (self.x, other.x),
            y: (self.y, other.y),
        }
    }

    /// Whether a condition is true for at least one of fields.
    pub fn any<F>(self, mut f: F) -> bool
    where
        F: FnMut(&T) -> bool,
    {
        f(&self.x) || f(&self.y)
    }

    /// Whether a condition is true for both fields.
    pub fn all<F>(self, mut f: F) -> bool
    where
        F: FnMut(&T) -> bool,
    {
        f(&self.x) && f(&self.y)
    }

    /// Filter the individual fields with a mask.
    pub fn filter(self, mask: Spec<bool>) -> Spec<Option<T>> {
        Spec {
            x: if mask.x { Some(self.x) } else { None },
            y: if mask.y { Some(self.y) } else { None },
        }
    }

    /// Convert to the generic representation.
    pub fn to_gen(self, main: SpecAxis) -> Gen<T> {
        match main {
            SpecAxis::Horizontal => Gen::new(self.y, self.x),
            SpecAxis::Vertical => Gen::new(self.x, self.y),
        }
    }
}

impl<T: Default> Spec<T> {
    /// Create a new instance with y set to its default value.
    pub fn with_x(x: T) -> Self {
        Self { x, y: T::default() }
    }

    /// Create a new instance with x set to its default value.
    pub fn with_y(y: T) -> Self {
        Self { x: T::default(), y }
    }
}

impl<T: Ord> Spec<T> {
    /// The component-wise minimum of this and another instance.
    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    /// The component-wise minimum of this and another instance.
    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }
}

impl<T> Get<SpecAxis> for Spec<T> {
    type Component = T;

    fn get(self, axis: SpecAxis) -> T {
        match axis {
            SpecAxis::Horizontal => self.x,
            SpecAxis::Vertical => self.y,
        }
    }

    fn get_mut(&mut self, axis: SpecAxis) -> &mut T {
        match axis {
            SpecAxis::Horizontal => &mut self.x,
            SpecAxis::Vertical => &mut self.y,
        }
    }
}

impl<T> Debug for Spec<T>
where
    T: Debug + 'static,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Spec { x: Some(x), y: Some(y) } =
            self.as_ref().map(|v| (v as &dyn Any).downcast_ref::<Align>())
        {
            write!(f, "{:?}-{:?}", x, y)
        } else if (&self.x as &dyn Any).is::<Length>() {
            write!(f, "Size({:?}, {:?})", self.x, self.y)
        } else {
            write!(f, "Spec({:?}, {:?})", self.x, self.y)
        }
    }
}

/// The two specific layouting axes.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum SpecAxis {
    /// The horizontal layouting axis.
    Horizontal,
    /// The vertical layouting axis.
    Vertical,
}

impl SpecAxis {
    /// The direction with the given positivity for this axis.
    pub fn dir(self, positive: bool) -> Dir {
        match (self, positive) {
            (Self::Horizontal, true) => Dir::LTR,
            (Self::Horizontal, false) => Dir::RTL,
            (Self::Vertical, true) => Dir::TTB,
            (Self::Vertical, false) => Dir::BTT,
        }
    }

    /// The other axis.
    pub fn other(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

impl Debug for SpecAxis {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        })
    }
}

/// A size in 2D.
pub type Size = Spec<Length>;

impl Size {
    /// The zero value.
    pub fn zero() -> Self {
        Self { x: Length::zero(), y: Length::zero() }
    }

    /// Whether the other size fits into this one (smaller width and height).
    pub fn fits(self, other: Self) -> bool {
        self.x.fits(other.x) && self.y.fits(other.y)
    }

    /// Whether both components are zero.
    pub fn is_zero(self) -> bool {
        self.x.is_zero() && self.y.is_zero()
    }

    /// Whether both components are finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Whether any of the two components is infinite.
    pub fn is_infinite(self) -> bool {
        self.x.is_infinite() || self.y.is_infinite()
    }

    /// Convert to a point.
    pub fn to_point(self) -> Point {
        Point::new(self.x, self.y)
    }
}

impl Neg for Size {
    type Output = Self;

    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y }
    }
}

impl Add for Size {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}

sub_impl!(Size - Size -> Size);

impl Mul<f64> for Size {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self { x: self.x * other, y: self.y * other }
    }
}

impl Mul<Size> for f64 {
    type Output = Size;

    fn mul(self, other: Size) -> Size {
        other * self
    }
}

impl Div<f64> for Size {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self { x: self.x / other, y: self.y / other }
    }
}

assign_impl!(Size -= Size);
assign_impl!(Size += Size);
assign_impl!(Size *= f64);
assign_impl!(Size /= f64);

impl<T> Spec<Option<T>> {
    /// Whether the individual fields are some.
    pub fn map_is_some(&self) -> Spec<bool> {
        self.as_ref().map(Option::is_some)
    }

    /// Whether the individual fields are none.
    pub fn map_is_none(&self) -> Spec<bool> {
        self.as_ref().map(Option::is_none)
    }

    /// Unwrap the individual fields.
    pub fn unwrap_or(self, other: Spec<T>) -> Spec<T> {
        Spec {
            x: self.x.unwrap_or(other.x),
            y: self.y.unwrap_or(other.y),
        }
    }
}

impl Spec<bool> {
    /// Select `t.x` if `self.x` is true and `f.x` otherwise and same for `y`.
    pub fn select<T>(self, t: Spec<T>, f: Spec<T>) -> Spec<T> {
        Spec {
            x: if self.x { t.x } else { f.x },
            y: if self.y { t.y } else { f.y },
        }
    }
}

impl Not for Spec<bool> {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self { x: !self.x, y: !self.y }
    }
}

impl BitOr for Spec<bool> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self { x: self.x | rhs.x, y: self.y | rhs.y }
    }
}

impl BitOr<bool> for Spec<bool> {
    type Output = Self;

    fn bitor(self, rhs: bool) -> Self::Output {
        Self { x: self.x | rhs, y: self.y | rhs }
    }
}

impl BitAnd for Spec<bool> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self { x: self.x & rhs.x, y: self.y & rhs.y }
    }
}

impl BitAnd<bool> for Spec<bool> {
    type Output = Self;

    fn bitand(self, rhs: bool) -> Self::Output {
        Self { x: self.x & rhs, y: self.y & rhs }
    }
}

impl BitOrAssign for Spec<bool> {
    fn bitor_assign(&mut self, rhs: Self) {
        self.x |= rhs.x;
        self.y |= rhs.y;
    }
}

impl BitAndAssign for Spec<bool> {
    fn bitand_assign(&mut self, rhs: Self) {
        self.x &= rhs.x;
        self.y &= rhs.y;
    }
}
