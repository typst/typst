use std::num::ParseFloatError;

use ecow::{eco_format, EcoString};

use crate::foundations::{cast, func, repr, scope, ty, Repr, Str};
use crate::layout::Ratio;

/// A floating-point number.
///
/// A limited-precision representation of a real number. Typst uses 64 bits to
/// store floats. Wherever a float is expected, you can also pass an
/// [integer]($int).
///
/// You can convert a value to a float with this type's constructor.
///
/// # Example
/// ```example
/// #3.14 \
/// #1e4 \
/// #(10 / 4)
/// ```
#[ty(scope, cast, name = "float")]
type f64;

#[scope]
impl f64 {
    /// Converts a value to a float.
    ///
    /// - Booleans are converted to `0.0` or `1.0`.
    /// - Integers are converted to the closest 64-bit float.
    /// - Ratios are divided by 100%.
    /// - Strings are parsed in base 10 to the closest 64-bit float.
    ///   Exponential notation is supported.
    ///
    /// ```example
    /// #float(false) \
    /// #float(true) \
    /// #float(4) \
    /// #float(40%) \
    /// #float("2.7") \
    /// #float("1e5")
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// The value that should be converted to a float.
        value: ToFloat,
    ) -> f64 {
        value.0
    }

    /// Checks if a float is not a number.
    ///
    /// In IEEE 754, more than one bit pattern represents a NaN. This function
    /// returns `true` if the float is any of those bit patterns.
    ///
    /// ```example
    /// #float.is-nan(0) \
    /// #float.is-nan(1) \
    /// #float.is-nan(calc.nan)
    /// ```
    #[func]
    pub fn is_nan(self) -> bool {
        f64::is_nan(self)
    }

    /// Checks if a float is infinite.
    ///
    /// For floats, there is positive and negative infinity. This function
    /// returns `true` if the float is either positive or negative infinity.
    ///
    /// ```example
    /// #float.is-infinite(0) \
    /// #float.is-infinite(1) \
    /// #float.is-infinite(calc.inf)
    /// ```
    #[func]
    pub fn is_infinite(self) -> bool {
        f64::is_infinite(self)
    }

    /// Calculates the sign of a floating point number.
    ///
    /// - If the number is positive (including `{+0.0}`), returns `{1.0}`.
    /// - If the number is negative (including `{-0.0}`), returns `{-1.0}`.
    /// - If the number is [`{calc.nan}`]($calc.nan), returns
    ///   [`{calc.nan}`]($calc.nan).
    ///
    /// ```example
    /// #(5.0).signum() \
    /// #(-5.0).signum() \
    /// #(0.0).signum() \
    /// ```
    #[func]
    pub fn signum(self) -> f64 {
        f64::signum(self)
    }
}

impl Repr for f64 {
    fn repr(&self) -> EcoString {
        repr::format_float(*self, None, true, "")
    }
}

/// A value that can be cast to a float.
pub struct ToFloat(f64);

cast! {
    ToFloat,
    v: f64 => Self(v),
    v: bool => Self(v as i64 as f64),
    v: i64 => Self(v as f64),
    v: Ratio => Self(v.get()),
    v: Str => Self(
        parse_float(v.clone().into())
            .map_err(|_| eco_format!("invalid float: {}", v))?
    ),
}

fn parse_float(s: EcoString) -> Result<f64, ParseFloatError> {
    s.replace(repr::MINUS_SIGN, "-").parse()
}
