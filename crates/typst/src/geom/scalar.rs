use super::*;

/// A 64-bit float that implements `Eq`, `Ord` and `Hash`.
///
/// Panics if it's `NaN` during any of those operations.
#[derive(Debug, Default, Copy, Clone)]
pub struct Scalar(f64);

// We have to detect NaNs this way since `f64::is_nan` isn’t const
// on stable yet:
// ([tracking issue](https://github.com/rust-lang/rust/issues/57241))
#[allow(clippy::unusual_byte_groupings)]
const fn is_nan_const(x: f64) -> bool {
    // Safety: all bit patterns are valid for u64, and f64 has no padding bits.
    // We cannot use `f64::to_bits` because it is not const.
    let x_bits = unsafe { std::mem::transmute::<f64, u64>(x) };
    (x_bits << 1 >> (64 - 12 + 1)) == 0b0_111_1111_1111 && (x_bits << 12) != 0
}

impl Scalar {
    /// Creates a [`Scalar`] with the given value.
    ///
    /// If the value is NaN, then it is set to `0.0` in the result.
    pub const fn new(x: f64) -> Self {
        Self(if is_nan_const(x) { 0.0 } else { x })
    }

    /// Gets the value of this [`Scalar`].
    #[inline]
    pub const fn get(self) -> f64 {
        self.0
    }

    /// The scalar containing `0.0`.
    pub const ZERO: Self = Self(0.0);
    /// The scalar containing `1.0`.
    pub const ONE: Self = Self(1.0);
    /// The scalar containing `f64::INFINITY`.
    pub const INFINITY: Self = Self(f64::INFINITY);
}

impl Numeric for Scalar {
    fn zero() -> Self {
        Self(0.0)
    }

    fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl From<f64> for Scalar {
    fn from(float: f64) -> Self {
        Self::new(float)
    }
}

impl From<Scalar> for f64 {
    fn from(scalar: Scalar) -> Self {
        scalar.0
    }
}

impl Repr for Scalar {
    fn repr(&self) -> EcoString {
        self.0.repr()
    }
}

impl Eq for Scalar {}

impl PartialEq for Scalar {
    fn eq(&self, other: &Self) -> bool {
        assert!(!self.0.is_nan() && !other.0.is_nan(), "float is NaN");
        self.0 == other.0
    }
}

impl PartialEq<f64> for Scalar {
    fn eq(&self, other: &f64) -> bool {
        self == &Self(*other)
    }
}

impl Ord for Scalar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).expect("float is NaN")
    }
}

impl PartialOrd for Scalar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Scalar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        debug_assert!(!self.0.is_nan(), "float is NaN");
        self.0.to_bits().hash(state);
    }
}

impl Neg for Scalar {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.0)
    }
}

impl<T: Into<Self>> Add<T> for Scalar {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Self::new(self.0 + rhs.into().0)
    }
}

impl<T: Into<Self>> AddAssign<T> for Scalar {
    fn add_assign(&mut self, rhs: T) {
        *self = *self + rhs.into();
    }
}

impl<T: Into<Self>> Sub<T> for Scalar {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        Self::new(self.0 - rhs.into().0)
    }
}

impl<T: Into<Self>> SubAssign<T> for Scalar {
    fn sub_assign(&mut self, rhs: T) {
        *self = *self - rhs.into();
    }
}

impl<T: Into<Self>> Mul<T> for Scalar {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self::new(self.0 * rhs.into().0)
    }
}

impl<T: Into<Self>> MulAssign<T> for Scalar {
    fn mul_assign(&mut self, rhs: T) {
        *self = *self * rhs.into();
    }
}

impl<T: Into<Self>> Div<T> for Scalar {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Self::new(self.0 / rhs.into().0)
    }
}

impl<T: Into<Self>> DivAssign<T> for Scalar {
    fn div_assign(&mut self, rhs: T) {
        *self = *self / rhs.into();
    }
}

impl<T: Into<Self>> Rem<T> for Scalar {
    type Output = Self;

    fn rem(self, rhs: T) -> Self::Output {
        Self::new(self.0 % rhs.into().0)
    }
}

impl<T: Into<Self>> RemAssign<T> for Scalar {
    fn rem_assign(&mut self, rhs: T) {
        *self = *self % rhs.into();
    }
}

impl Sum for Scalar {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::new(iter.map(|s| s.0).sum())
    }
}

impl<'a> Sum<&'a Self> for Scalar {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        Self::new(iter.map(|s| s.0).sum())
    }
}
