use std::num::{NonZeroI64, NonZeroIsize, NonZeroU64, NonZeroUsize, ParseIntError};

use ecow::{eco_format, EcoString};

use crate::{
    diag::StrResult,
    foundations::{cast, func, repr, scope, ty, Repr, Str, Value},
};

/// A whole number.
///
/// The number can be negative, zero, or positive. As Typst uses 64 bits to
/// store integers, integers cannot be smaller than `{-9223372036854775808}` or
/// larger than `{9223372036854775807}`.
///
/// The number can also be specified as hexadecimal, octal, or binary by
/// starting it with a zero followed by either `x`, `o`, or `b`.
///
/// You can convert a value to an integer with this type's constructor.
///
/// # Example
/// ```example
/// #(1 + 2) \
/// #(2 - 5) \
/// #(3 + 4 < 8)
///
/// #0xff \
/// #0o10 \
/// #0b1001
/// ```
#[ty(scope, cast, name = "int", title = "Integer")]
type i64;

#[scope]
impl i64 {
    /// Converts a value to an integer.
    ///
    /// - Booleans are converted to `0` or `1`.
    /// - Floats are floored to the next 64-bit integer.
    /// - Strings are parsed in base 10.
    ///
    /// ```example
    /// #int(false) \
    /// #int(true) \
    /// #int(2.7) \
    /// #(int("27") + int("4"))
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// The value that should be converted to an integer.
        value: ToInt,
    ) -> i64 {
        value.0
    }

    /// Calculates the sign of an integer.
    ///
    /// - If the number is positive, returns `{1}`.
    /// - If the number is negative, returns `{-1}`.
    /// - If the number is zero, returns `{0}`.
    ///
    /// ```example
    /// #(5).signum() \
    /// #(-5).signum() \
    /// #(0).signum() \
    /// ```
    #[func]
    pub fn signum(self) -> i64 {
        i64::signum(self)
    }

    /// Calculates the bitwise NOT of an integer.
    ///
    /// For the purposes of this function, the operand is treated as a signed
    /// integer of 64 bits.
    ///
    /// ```example
    /// #4.bit-not()
    /// #(-1).bit-not()
    /// ```
    #[func(title = "Bitwise NOT")]
    pub fn bit_not(self) -> i64 {
        !self
    }

    /// Calculates the bitwise AND between two integers.
    ///
    /// For the purposes of this function, the operands are treated as signed
    /// integers of 64 bits.
    ///
    /// ```example
    /// #128.bit-and(192)
    /// ```
    #[func(title = "Bitwise AND")]
    pub fn bit_and(
        self,
        /// The right-hand operand of the bitwise AND.
        rhs: i64,
    ) -> i64 {
        self & rhs
    }

    /// Calculates the bitwise OR between two integers.
    ///
    /// For the purposes of this function, the operands are treated as signed
    /// integers of 64 bits.
    ///
    /// ```example
    /// #64.bit-or(32)
    /// ```
    #[func(title = "Bitwise OR")]
    pub fn bit_or(
        self,
        /// The right-hand operand of the bitwise OR.
        rhs: i64,
    ) -> i64 {
        self | rhs
    }

    /// Calculates the bitwise XOR between two integers.
    ///
    /// For the purposes of this function, the operands are treated as signed
    /// integers of 64 bits.
    ///
    /// ```example
    /// #64.bit-xor(96)
    /// ```
    #[func(title = "Bitwise XOR")]
    pub fn bit_xor(
        self,
        /// The right-hand operand of the bitwise XOR.
        rhs: i64,
    ) -> i64 {
        self ^ rhs
    }

    /// Shifts the operand's bits to the left by the specified amount.
    ///
    /// For the purposes of this function, the operand is treated as a signed
    /// integer of 64 bits. An error will occur if the result is too large to
    /// fit in a 64-bit integer.
    ///
    /// ```example
    /// #33.bit-lshift(2)
    /// #(-1).bit-lshift(3)
    /// ```
    #[func(title = "Bitwise Left Shift")]
    pub fn bit_lshift(
        self,

        /// The amount of bits to shift. Must not be negative.
        shift: u32,
    ) -> StrResult<i64> {
        Ok(self.checked_shl(shift).ok_or("the result is too large")?)
    }

    /// Shifts the operand's bits to the right by the specified amount.
    /// Performs an arithmetic shift by default (extends the sign bit to the left,
    /// such that negative numbers stay negative), but that can be changed by the
    /// `logical` parameter.
    ///
    /// For the purposes of this function, the operand is treated as a signed
    /// integer of 64 bits.
    ///
    /// ```example
    /// #64.bit-rshift(2)
    /// #(-8).bit-rshift(2)
    /// #(-8).bit-rshift(2, logical: true)
    /// ```
    #[func(title = "Bitwise Right Shift")]
    pub fn bit_rshift(
        self,

        /// The amount of bits to shift. Must not be negative.
        ///
        /// Shifts larger than 63 are allowed and will cause the return value to
        /// saturate. For non-negative numbers, the return value saturates at `0`,
        /// while, for negative numbers, it saturates at `-1` if `logical` is set
        /// to `false`, or `0` if it is `true`. This behavior is consistent with
        /// just applying this operation multiple times. Therefore, the shift will
        /// always succeed.
        shift: u32,

        /// Toggles whether a logical (unsigned) right shift should be performed
        /// instead of arithmetic right shift.
        /// If this is `true`, negative operands will not preserve their sign bit,
        /// and bits which appear to the left after the shift will be `0`.
        /// This parameter has no effect on non-negative operands.
        #[named]
        #[default(false)]
        logical: bool,
    ) -> i64 {
        if logical {
            if shift >= u64::BITS {
                // Excessive logical right shift would be equivalent to setting
                // all bits to zero. Using `.min(63)` is not enough for logical
                // right shift, since `-1 >> 63` returns 1, whereas
                // `calc.bit-rshift(-1, 64)` should return the same as
                // `(-1 >> 63) >> 1`, which is zero.
                0
            } else {
                // Here we reinterpret the signed integer's bits as unsigned to
                // perform logical right shift, and then reinterpret back as signed.
                // This is valid as, according to the Rust reference, casting between
                // two integers of same size (i64 <-> u64) is a no-op (two's complement
                // is used).
                // Reference:
                // https://doc.rust-lang.org/stable/reference/expressions/operator-expr.html#numeric-cast
                ((self as u64) >> shift) as i64
            }
        } else {
            // Saturate at -1 (negative) or 0 (otherwise) on excessive arithmetic
            // right shift. Shifting those numbers any further does not change
            // them, so it is consistent.
            let shift = shift.min(i64::BITS - 1);
            self >> shift
        }
    }
}

impl Repr for i64 {
    fn repr(&self) -> EcoString {
        eco_format!("{:?}", self)
    }
}

/// A value that can be cast to an integer.
pub struct ToInt(i64);

cast! {
    ToInt,
    v: i64 => Self(v),
    v: bool => Self(v as i64),
    v: f64 => Self(v as i64),
    v: Str => Self(parse_int(&v).map_err(|_| eco_format!("invalid integer: {}", v))?),
}

fn parse_int(mut s: &str) -> Result<i64, ParseIntError> {
    let mut sign = 1;
    if let Some(rest) = s.strip_prefix('-').or_else(|| s.strip_prefix(repr::MINUS_SIGN)) {
        sign = -1;
        s = rest;
    }
    if sign == -1 && s == "9223372036854775808" {
        return Ok(i64::MIN);
    }
    Ok(sign * s.parse::<i64>()?)
}

macro_rules! signed_int {
    ($($ty:ty)*) => {
        $(cast! {
            $ty,
            self => Value::Int(self as _),
            v: i64 => v.try_into().map_err(|_| "number too large")?,
        })*
    }
}

macro_rules! unsigned_int {
    ($($ty:ty)*) => {
        $(cast! {
            $ty,
            self => if let Ok(int) = i64::try_from(self) {
                Value::Int(int)
            } else {
                // Some u64 are too large to be cast as i64
                // In that case, we accept that there may be a
                // precision loss, and use a floating point number
                Value::Float(self as _)
            },
            v: i64 => v.try_into().map_err(|_| {
                if v < 0 {
                    "number must be at least zero"
                } else {
                    "number too large"
                }
            })?,
        })*
    }
}

signed_int! { i8 i16 i32 isize }
unsigned_int! { u8 u16 u32 u64 usize }

cast! {
    NonZeroI64,
    self => Value::Int(self.get() as _),
    v: i64 => v.try_into()
        .map_err(|_| if v == 0 {
            "number must not be zero"
        } else {
            "number too large"
        })?,
}

cast! {
    NonZeroIsize,
    self => Value::Int(self.get() as _),
    v: i64 => v
        .try_into()
        .and_then(|v: isize| v.try_into())
        .map_err(|_| if v == 0 {
            "number must not be zero"
        } else {
            "number too large"
        })?,
}

cast! {
    NonZeroU64,
    self => Value::Int(self.get() as _),
    v: i64 => v
        .try_into()
        .and_then(|v: u64| v.try_into())
        .map_err(|_| if v <= 0 {
            "number must be positive"
        } else {
            "number too large"
        })?,
}

cast! {
    NonZeroUsize,
    self => Value::Int(self.get() as _),
    v: i64 => v
        .try_into()
        .and_then(|v: usize| v.try_into())
        .map_err(|_| if v <= 0 {
            "number must be positive"
        } else {
            "number too large"
        })?,
}
