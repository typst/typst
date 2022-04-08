use super::*;

/// A value that is composed of a relative and an absolute part.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Relative<T: Numeric> {
    /// The relative part.
    pub rel: Ratio,
    /// The absolute part.
    pub abs: T,
}

impl<T: Numeric> Relative<T> {
    /// The zero relative.
    pub fn zero() -> Self {
        Self { rel: Ratio::zero(), abs: T::zero() }
    }

    /// A relative with a ratio of `100%` and no absolute part.
    pub fn one() -> Self {
        Self { rel: Ratio::one(), abs: T::zero() }
    }

    /// Create a new relative from its parts.
    pub fn new(rel: Ratio, abs: T) -> Self {
        Self { rel, abs }
    }

    /// Whether both parts are zero.
    pub fn is_zero(self) -> bool {
        self.rel.is_zero() && self.abs.is_zero()
    }

    /// Resolve this relative to the given `whole`.
    pub fn resolve(self, whole: T) -> T {
        self.rel.resolve(whole) + self.abs
    }

    /// Map the absolute part with `f`.
    pub fn map<F, U>(self, f: F) -> Relative<U>
    where
        F: FnOnce(T) -> U,
        U: Numeric,
    {
        Relative { rel: self.rel, abs: f(self.abs) }
    }
}

impl<T: Numeric> Debug for Relative<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?} + {:?}", self.rel, self.abs)
    }
}

impl<T: Numeric> From<T> for Relative<T> {
    fn from(abs: T) -> Self {
        Self { rel: Ratio::zero(), abs }
    }
}

impl<T: Numeric> From<Ratio> for Relative<T> {
    fn from(rel: Ratio) -> Self {
        Self { rel, abs: T::zero() }
    }
}

impl<T: Numeric> Neg for Relative<T> {
    type Output = Self;

    fn neg(self) -> Self {
        Self { rel: -self.rel, abs: -self.abs }
    }
}

impl<T: Numeric> Add for Relative<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            rel: self.rel + other.rel,
            abs: self.abs + other.abs,
        }
    }
}

impl<T: Numeric> Sub for Relative<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        self + -other
    }
}

impl<T: Numeric> Mul<f64> for Relative<T> {
    type Output = Self;

    fn mul(self, other: f64) -> Self::Output {
        Self {
            rel: self.rel * other,
            abs: self.abs * other,
        }
    }
}

impl<T: Numeric> Mul<Relative<T>> for f64 {
    type Output = Relative<T>;

    fn mul(self, other: Relative<T>) -> Self::Output {
        other * self
    }
}

impl<T: Numeric> Div<f64> for Relative<T> {
    type Output = Self;

    fn div(self, other: f64) -> Self::Output {
        Self {
            rel: self.rel / other,
            abs: self.abs / other,
        }
    }
}

impl<T: Numeric> AddAssign for Relative<T> {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl<T: Numeric> SubAssign for Relative<T> {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl<T: Numeric> MulAssign<f64> for Relative<T> {
    fn mul_assign(&mut self, other: f64) {
        *self = *self * other;
    }
}

impl<T: Numeric> DivAssign<f64> for Relative<T> {
    fn div_assign(&mut self, other: f64) {
        *self = *self * other;
    }
}

impl<T: Numeric> Add<T> for Ratio {
    type Output = Relative<T>;

    fn add(self, other: T) -> Self::Output {
        Relative::from(self) + Relative::from(other)
    }
}

impl<T: Numeric> Add<T> for Relative<T> {
    type Output = Self;

    fn add(self, other: T) -> Self::Output {
        self + Relative::from(other)
    }
}

impl<T: Numeric> Add<Ratio> for Relative<T> {
    type Output = Self;

    fn add(self, other: Ratio) -> Self::Output {
        self + Relative::from(other)
    }
}
