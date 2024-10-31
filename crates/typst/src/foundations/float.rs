use std::num::ParseFloatError;

use ecow::{eco_format, EcoString};

use crate::diag::StrResult;
use crate::foundations::{
    bail, cast, func, repr, scope, ty, Bytes, Decimal, Endianness, Repr, Str,
};
use crate::layout::Ratio;

/// A floating-point number.
///
/// A limited-precision representation of a real number. Typst uses 64 bits to
/// store floats. Wherever a float is expected, you can also pass an
/// [integer]($int).
///
/// You can convert a value to a float with this type's constructor.
///
/// NaN and positive infinity are available as `{float.nan}` and `{float.inf}`
/// respectively.
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
    /// Positive infinity.
    const INF: f64 = f64::INFINITY;

    /// A NaN value, as defined by the
    /// [IEEE 754 standard](https://en.wikipedia.org/wiki/IEEE_754).
    const NAN: f64 = f64::NAN;

    /// Converts a value to a float.
    ///
    /// - Booleans are converted to `0.0` or `1.0`.
    /// - Integers are converted to the closest 64-bit float. For integers with
    ///   absolute value less than `{calc.pow(2, 53)}`, this conversion is
    ///   exact.
    /// - Ratios are divided by 100%.
    /// - Strings are parsed in base 10 to the closest 64-bit float. Exponential
    ///   notation is supported.
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
    /// #float.is-nan(float.nan)
    /// ```
    #[func]
    pub fn is_nan(self) -> bool {
        f64::is_nan(self)
    }

    /// Checks if a float is infinite.
    ///
    /// Floats can represent positive infinity and negative infinity. This
    /// function returns `{true}` if the float is an infinity.
    ///
    /// ```example
    /// #float.is-infinite(0) \
    /// #float.is-infinite(1) \
    /// #float.is-infinite(float.inf)
    /// ```
    #[func]
    pub fn is_infinite(self) -> bool {
        f64::is_infinite(self)
    }

    /// Calculates the sign of a floating point number.
    ///
    /// - If the number is positive (including `{+0.0}`), returns `{1.0}`.
    /// - If the number is negative (including `{-0.0}`), returns `{-1.0}`.
    /// - If the number is NaN, returns `{float.nan}`.
    ///
    /// ```example
    /// #(5.0).signum() \
    /// #(-5.0).signum() \
    /// #(0.0).signum() \
    /// #float.nan.signum()
    /// ```
    #[func]
    pub fn signum(self) -> f64 {
        f64::signum(self)
    }

    /// Converts bytes to a float.
    ///
    /// ```example
    /// #float.from-bytes(bytes((0, 0, 0, 0, 0, 0, 240, 63))) \
    /// #float.from-bytes(bytes((63, 240, 0, 0, 0, 0, 0, 0)), endian: "big")
    /// ```
    #[func]
    pub fn from_bytes(
        /// The bytes that should be converted to a float.
        ///
        /// Must be of length exactly 8 so that the result fits into a 64-bit
        /// float.
        bytes: Bytes,
        /// The endianness of the conversion.
        #[named]
        #[default(Endianness::Little)]
        endian: Endianness,
    ) -> StrResult<f64> {
        // Convert slice to an array of length 8.
        let buf: [u8; 8] = match bytes.as_ref().try_into() {
            Ok(buffer) => buffer,
            Err(_) => bail!("bytes must have a length of exactly 8"),
        };

        Ok(match endian {
            Endianness::Little => f64::from_le_bytes(buf),
            Endianness::Big => f64::from_be_bytes(buf),
        })
    }

    /// Converts a float to bytes.
    ///
    /// ```example
    /// #array(1.0.to-bytes(endian: "big")) \
    /// #array(1.0.to-bytes())
    /// ```
    #[func]
    pub fn to_bytes(
        self,
        /// The endianness of the conversion.
        #[named]
        #[default(Endianness::Little)]
        endian: Endianness,
    ) -> Bytes {
        match endian {
            Endianness::Little => self.to_le_bytes(),
            Endianness::Big => self.to_be_bytes(),
        }
        .as_slice()
        .into()
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
    v: Decimal => Self(f64::try_from(v).map_err(|_| eco_format!("invalid float: {}", v))?),
    v: Ratio => Self(v.get()),
    v: Str => Self(
        parse_float(v.clone().into())
            .map_err(|_| eco_format!("invalid float: {}", v))?
    ),
}

fn parse_float(s: EcoString) -> Result<f64, ParseFloatError> {
    s.replace(repr::MINUS_SIGN, "-").parse()
}
