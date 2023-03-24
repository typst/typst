#[allow(clippy::wildcard_imports /* this module exists to reduce file size, not to introduce a new scope */)]
use super::*;

/// A value that is composed of a relative and an absolute part.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rel<T: Numeric> {
    /// The relative part.
    pub rel: Ratio,
    /// The absolute part.
    pub abs: T,
}

impl<T: Numeric> Rel<T> {
    /// The zero relative.
    #[must_use]
    #[inline]
    pub fn zero() -> Self {
        Self { rel: Ratio::zero(), abs: T::zero() }
    }

    /// A relative with a ratio of `100%` and no absolute part.
    #[must_use]
    #[inline]
    pub fn one() -> Self {
        Self { rel: Ratio::one(), abs: T::zero() }
    }

    /// Create a new relative from its parts.
    #[must_use]
    #[inline]
    pub fn new(rel: Ratio, abs: T) -> Self {
        Self { rel, abs }
    }

    /// Whether both parts are zero.
    #[must_use]
    #[inline]
    pub fn is_zero(self) -> bool {
        self.rel.is_zero() && self.abs == T::zero()
    }

    /// Whether the relative part is one and the absolute part is zero.
    #[must_use]
    #[inline]
    pub fn is_one(self) -> bool {
        self.rel.is_one() && self.abs == T::zero()
    }

    /// Evaluate this relative to the given `whole`.
    #[must_use]
    #[inline]
    pub fn relative_to(self, whole: T) -> T {
        self.rel.of(whole) + self.abs
    }

    /// Map the absolute part with `f`.
    #[must_use]
    #[inline]
    pub fn map<F, U>(self, f: F) -> Rel<U>
    where
        F: FnOnce(T) -> U,
        U: Numeric,
    {
        Rel { rel: self.rel, abs: f(self.abs) }
    }
}

impl Rel<Length> {
    /// Try to divide two relative lengths.
    #[must_use]
    #[inline]
    pub fn try_div(self, other: Self) -> Option<f64> {
        if self.rel.is_zero() && other.rel.is_zero() {
            self.abs.try_div(other.abs)
        } else if self.abs.is_zero() && other.abs.is_zero() {
            Some(self.rel / other.rel)
        } else {
            None
        }
    }
}

impl<T: Numeric> Debug for Rel<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match (self.rel.is_zero(), self.abs.is_zero()) {
            (false, false) => write!(f, "{:?} + {:?}", self.rel, self.abs),
            (false, true) => self.rel.fmt(f),
            (true, _) => self.abs.fmt(f),
        }
    }
}

impl<T: Numeric> From<T> for Rel<T> {
    #[inline]
    fn from(abs: T) -> Self {
        Self { rel: Ratio::zero(), abs }
    }
}

impl<T: Numeric> From<Ratio> for Rel<T> {
    #[inline]
    fn from(rel: Ratio) -> Self {
        Self { rel, abs: T::zero() }
    }
}

impl<T: Numeric + PartialOrd> PartialOrd for Rel<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.rel.is_zero() && other.rel.is_zero() {
            self.abs.partial_cmp(&other.abs)
        } else if self.abs.is_zero() && other.abs.is_zero() {
            self.rel.partial_cmp(&other.rel)
        } else {
            None
        }
    }
}

impl<T: Numeric> Neg for Rel<T> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self { rel: -self.rel, abs: -self.abs }
    }
}

impl<T: Numeric> Add for Rel<T> {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        Self {
            rel: self.rel + other.rel,
            abs: self.abs + other.abs,
        }
    }
}

impl<T: Numeric> Sub for Rel<T> {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        self + -other
    }
}

impl<T: Numeric> Mul<f64> for Rel<T> {
    type Output = Self;

    #[inline]
    fn mul(self, other: f64) -> Self::Output {
        Self { rel: self.rel * other, abs: self.abs * other }
    }
}

impl<T: Numeric> Mul<Rel<T>> for f64 {
    type Output = Rel<T>;

    #[inline]
    fn mul(self, other: Rel<T>) -> Self::Output {
        other * self
    }
}

impl<T: Numeric> Div<f64> for Rel<T> {
    type Output = Self;

    #[inline]
    fn div(self, other: f64) -> Self::Output {
        Self { rel: self.rel / other, abs: self.abs / other }
    }
}

impl<T: Numeric + AddAssign> AddAssign for Rel<T> {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.rel += other.rel;
        self.abs += other.abs;
    }
}

impl<T: Numeric + SubAssign> SubAssign for Rel<T> {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.rel -= other.rel;
        self.abs -= other.abs;
    }
}

impl<T: Numeric + MulAssign<f64>> MulAssign<f64> for Rel<T> {
    #[inline]
    fn mul_assign(&mut self, other: f64) {
        self.rel *= other;
        self.abs *= other;
    }
}

impl<T: Numeric + DivAssign<f64>> DivAssign<f64> for Rel<T> {
    #[inline]
    fn div_assign(&mut self, other: f64) {
        self.rel /= other;
        self.abs /= other;
    }
}

impl<T: Numeric> Add<T> for Ratio {
    type Output = Rel<T>;

    #[inline]
    fn add(self, other: T) -> Self::Output {
        Rel::from(self) + Rel::from(other)
    }
}

impl<T: Numeric> Add<T> for Rel<T> {
    type Output = Self;

    #[inline]
    fn add(self, other: T) -> Self::Output {
        self + Rel::from(other)
    }
}

impl<T: Numeric> Add<Ratio> for Rel<T> {
    type Output = Self;

    #[inline]
    fn add(self, other: Ratio) -> Self::Output {
        self + Rel::from(other)
    }
}

impl<T> Resolve for Rel<T>
where
    T: Resolve + Numeric,
    <T as Resolve>::Output: Numeric,
{
    type Output = Rel<<T as Resolve>::Output>;

    #[inline]
    fn resolve(self, styles: StyleChain<'_>) -> Self::Output {
        self.map(|abs| abs.resolve(styles))
    }
}

impl Fold for Rel<Abs> {
    type Output = Self;

    #[inline]
    fn fold(self, _: Self::Output) -> Self::Output {
        self
    }
}

impl Fold for Rel<Length> {
    type Output = Self;

    #[inline]
    fn fold(self, _: Self::Output) -> Self::Output {
        self
    }
}
