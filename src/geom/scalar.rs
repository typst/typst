use super::*;

/// A 64-bit float that implements `Eq`, `Ord` and `Hash`.
///
/// Panics if it's `NaN` during any of those operations.
#[derive(Default, Copy, Clone)]
pub struct Scalar(pub f64);

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
        Self(float)
    }
}

impl From<Scalar> for f64 {
    fn from(scalar: Scalar) -> Self {
        scalar.0
    }
}

impl Debug for Scalar {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
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
        self.partial_cmp(other).expect("float is NaN")
    }
}

impl PartialOrd for Scalar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }

    fn lt(&self, other: &Self) -> bool {
        self.0 < other.0
    }

    fn le(&self, other: &Self) -> bool {
        self.0 <= other.0
    }

    fn gt(&self, other: &Self) -> bool {
        self.0 > other.0
    }

    fn ge(&self, other: &Self) -> bool {
        self.0 >= other.0
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
        Self(-self.0)
    }
}

impl<T: Into<Self>> Add<T> for Scalar {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Self(self.0 + rhs.into().0)
    }
}

impl<T: Into<Self>> AddAssign<T> for Scalar {
    fn add_assign(&mut self, rhs: T) {
        self.0 += rhs.into().0;
    }
}

impl<T: Into<Self>> Sub<T> for Scalar {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        Self(self.0 - rhs.into().0)
    }
}

impl<T: Into<Self>> SubAssign<T> for Scalar {
    fn sub_assign(&mut self, rhs: T) {
        self.0 -= rhs.into().0;
    }
}

impl<T: Into<Self>> Mul<T> for Scalar {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self(self.0 * rhs.into().0)
    }
}

impl<T: Into<Self>> MulAssign<T> for Scalar {
    fn mul_assign(&mut self, rhs: T) {
        self.0 *= rhs.into().0;
    }
}

impl<T: Into<Self>> Div<T> for Scalar {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Self(self.0 / rhs.into().0)
    }
}

impl<T: Into<Self>> DivAssign<T> for Scalar {
    fn div_assign(&mut self, rhs: T) {
        self.0 /= rhs.into().0;
    }
}

impl<T: Into<Self>> Rem<T> for Scalar {
    type Output = Self;

    fn rem(self, rhs: T) -> Self::Output {
        Self(self.0 % rhs.into().0)
    }
}

impl<T: Into<Self>> RemAssign<T> for Scalar {
    fn rem_assign(&mut self, rhs: T) {
        self.0 %= rhs.into().0;
    }
}

impl Sum for Scalar {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}

impl<'a> Sum<&'a Self> for Scalar {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}
