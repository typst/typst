use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, Div, Mul, Neg};

use ecow::EcoString;
use typst_utils::{Numeric, Scalar};

use crate::foundations::{repr, ty, Repr};

/// A ratio of a whole.
///
/// Written as a number, followed by a percent sign. A common use case is
/// setting the width or height of a container (e.g., [block], [rect], etc.),
/// as it can be used as part of a [relative length]($relative) to represent
/// a certain percentage of the size of the surrounding container or of the
/// current page. For example:
///
/// ```example
/// #block(width: 240pt, {
///   rect(width: 25%, inset: 0pt, layout(size => size.width))
/// })
/// ```
///
/// Here the block width is set to `{240pt}` (just to demonstrate the use of
/// ratio with containers), and inside of it the rectangle width is set to
/// `{25%}`, which means "get 25% of the width of the innermost container" (240
/// ⋅ 0.25 = 60). Notice that the inset is equal to `{0pt}`, if it's not set
/// then it will show `{50pt}` instead of `{60pt}`, which is also why the number
/// looks cramped.
///
/// See [relative length]($relative) for more details.
///
/// However, within your own code, you can use ratios as you'd like. You can
/// multiply ratio by ratio, [length], [relative length]($relative), [angle],
/// [int], [float], and [fraction].
///
/// ```example
/// #ratio: #(27% * 10%) \
/// #length: #(27% * 100pt) \
/// #relative: #(27% * (10% + 100pt)) \
/// #angle: #(27% * 100deg) \
/// #int: #(27% * 2) \
/// #float: #(27% * 0.37037) \ // Some rounding is happening.
/// #fraction: #(27% * 3fr)
///
/// #table(
///   columns: 2,
///   align: (right, left),
///   inset: (x: 2pt),
///   table.vline(x: 1, stroke: none),
///   [#ratio:], [#(27% * 10%)],
///   [#length:], [#(27% * 100pt)],
///   [#relative:], [#(27% * (10% + 100pt))],
///   [#angle:], [#(27% * 100deg)],
///   [#int:], [#(27% * 2)],
///   [#float:], [#(27% * 0.37037)], // Some rounding is happening.
///   [#fraction:], [#(27% * 3fr)],
/// )
/// ```
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
