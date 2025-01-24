use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, Div, Mul, Neg};

use ecow::EcoString;
use typst_utils::{Numeric, Scalar};

use crate::foundations::{repr, ty, Repr};

/// A ratio of a whole.
///
/// Written as a number, followed by a percent sign. A common use case is
/// setting the width or height of a container (e.g., [block], [rect], etc.),
/// thus it is used as a part of a [relative length](relative) (because
/// internally containers use relative length for width and height fields):
///
/// ```example
/// #block(width: 240pt, {
///   rect(width: 25%, layout(size => size.width))
/// })
/// ```
///
/// Here the block width is set to `{250pt}` (just to demonstrate the use of
/// ratio with containers), and inside of it the rectangle width is set to
/// `{25%}`, which means "get 25% of the width of the innermost container" (240
/// ⋅ 0.25 = 60). The reason it shows `{50pt}` instead of `{60pt}` is due to
/// the default value of inset. If we set it to `{0pt}`, then we will get
/// the expected result (although the number will be cramped):
///
/// ```example
/// #block(width: 240pt, {
///   rect(width: 25%, inset: 0pt, layout(size => size.width))
/// })
/// ```
///
/// If you are trying to use the page width/height's value as the base for the
/// ratio, then the value is equal to the width/height of the page minus the
/// margins (left and right for width, top and bottom for height). To force
/// Typst to use the full length you have a few options (this highly depends on
/// your exact use case):
///
/// 1. set page margins to `{0pt}` (`#set page(margin: 0pt)`)
/// 2. multiply by a full length value (`{21cm * 69%}`)
/// 3. use padding which will negate the margins (`#pad(x: -2.5cm, ...)`)
/// 4. use page [background](page.background)/[foreground](page.foreground)
///    field that doesn't use the margins (note that it will render the content
///    outside of the document flow, see [place] to control the content
///    position)
///
/// However, within your own code, you can use ratios as you'd like. You can
/// multiply ratio by ratio, [length], [relative length](relative), [angle],
/// [int], [float], and [fraction].
///
/// # Example
/// ```example
/// #set align(center)
/// #scale(x: 150%)[
///   Scaled apart.
/// ]
/// ```
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
        if resolved.is_finite() {
            resolved
        } else {
            T::zero()
        }
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
