use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, Div, Mul, Neg};

use ecow::EcoString;
use typst_utils::{Numeric, Scalar};

use crate::foundations::{Repr, repr, ty};

/// A ratio of a whole.
///
/// A ratio is written as a number, followed by a percent sign. Ratios most
/// often appear as part of a [relative length]($relative), to specify the size
/// of some layout element relative to the page or some container.
///
/// ```example
/// #rect(width: 25%)
/// ```
///
/// However, they can also describe any other property that is relative to some
/// base, e.g. an amount of [horizontal scaling]($scale.x) or the
/// [height of parentheses]($math.lr.size) relative to the height of the content
/// they enclose.
///
/// # Scripting
/// Within your own code, you can use ratios as you like. You can multiply them
/// with various other types as shown below:
///
/// |  Multiply by    |  Example                | Result          |
/// |-----------------|-------------------------|-----------------|
/// | [`ratio`]       | `{27% * 10%}`           | `{2.7%}`        |
/// | [`length`]      | `{27% * 100pt}`         | `{27pt}`        |
/// | [`relative`]    | `{27% * (10% + 100pt)}` | `{2.7% + 27pt}` |
/// | [`angle`]       | `{27% * 100deg}`        | `{27deg}`       |
/// | [`int`]         | `{27% * 2}`             | `{54%}`         |
/// | [`float`]       | `{27% * 0.37037}`       | `{10%}`         |
/// | [`fraction`]    | `{27% * 3fr}`           | `{0.81fr}`      |
///
/// When ratios are [displayed]($repr) in the document, they are rounded to two
/// significant digits for readability.
#[ty(cast)]
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ratio(Scalar);

impl Ratio {
    /// A ratio of `0%` represented as `0.0`.
    pub const fn zero() -> Self {
        Self(Scalar::ZERO)
    }

    /// A ratio of `100%` represented as `1.0`.
    pub const fn one() -> Self {
        Self(Scalar::ONE)
    }

    /// Create a new ratio from a value, where `1.0` means `100%`.
    pub const fn new(ratio: f64) -> Self {
        Self(Scalar::new(ratio))
    }

    /// Get the underlying ratio.
    pub const fn get(self) -> f64 {
        (self.0).get()
    }

    /// Whether the ratio is zero.
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }

    /// Whether the ratio is one.
    pub fn is_one(self) -> bool {
        self.0 == 1.0
    }

    /// The absolute value of this ratio.
    pub fn abs(self) -> Self {
        Self::new(self.get().abs())
    }

    /// Return the ratio of the given `whole`.
    pub fn of<T: Numeric>(self, whole: T) -> T {
        let resolved = whole * self.get();
        if resolved.is_finite() { resolved } else { T::zero() }
    }
}

impl Debug for Ratio {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}%", self.get() * 100.0)
    }
}

impl Repr for Ratio {
    fn repr(&self) -> EcoString {
        repr::format_float_with_unit(self.get() * 100.0, "%")
    }
}

impl Neg for Ratio {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Ratio {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

typst_utils::sub_impl!(Ratio - Ratio -> Ratio);

impl Mul for Ratio {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }
}

impl Mul<f64> for Ratio {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Ratio> for f64 {
    type Output = Ratio;

    fn mul(self, other: Ratio) -> Ratio {
        other * self
    }
}

impl Div for Ratio {
    type Output = f64;

    fn div(self, other: Self) -> f64 {
        self.get() / other.get()
    }
}

impl Div<f64> for Ratio {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

impl Div<Ratio> for f64 {
    type Output = Self;

    fn div(self, other: Ratio) -> Self {
        self / other.get()
    }
}

typst_utils::assign_impl!(Ratio += Ratio);
typst_utils::assign_impl!(Ratio -= Ratio);
typst_utils::assign_impl!(Ratio *= Ratio);
typst_utils::assign_impl!(Ratio *= f64);
typst_utils::assign_impl!(Ratio /= f64);
