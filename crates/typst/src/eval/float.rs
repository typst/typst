use crate::eval::Repr;
use ecow::{eco_format, EcoString};
use std::num::ParseFloatError;

use super::{cast, func, scope, ty, Str};
use crate::geom::Ratio;
use crate::util::fmt::{format_float, MINUS_SIGN};

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
#[ty(scope, name = "float")]
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
}

impl Repr for f64 {
    fn repr(&self) -> EcoString {
        format_float(*self, None, "")
    }
}

/// A value that can be cast to a float.
pub struct ToFloat(f64);

fn parse_float(mut s: &str) -> Result<f64, ParseFloatError> {
    let mut sign = 1.0;
    if s.starts_with(MINUS_SIGN) {
        sign = -1.0;
        s = &s[MINUS_SIGN.len_utf8()..];
    } else if s.starts_with('-') {
        sign = -1.0;
        s = &s['-'.len_utf8()..];
    }
    Ok(sign * s.parse::<f64>()?)
}

cast! {
    ToFloat,
    v: bool => Self(v as i64 as f64),
    v: i64 => Self(v as f64),
    v: Ratio => Self(v.get()),
    v: Str => Self(parse_float(&v).map_err(|_| eco_format!("invalid float: {}", v))?),
    v: f64 => Self(v),
}
