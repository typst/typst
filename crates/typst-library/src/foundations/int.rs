use std::num::{
    NonZeroI64, NonZeroIsize, NonZeroU32, NonZeroU64, NonZeroUsize, ParseIntError,
};

use ecow::{eco_format, EcoString};
use smallvec::SmallVec;

use crate::diag::{bail, StrResult};
use crate::foundations::{
    cast, func, repr, scope, ty, Bytes, Cast, Decimal, Repr, Str, Value,
};

/// A whole number.
///
/// The number can be negative, zero, or positive. As Typst uses 64 bits to
/// store integers, integers cannot be smaller than `{-9223372036854775808}` or
/// larger than `{9223372036854775807}`. Integer literals are always positive,
/// so a negative integer such as `{-1}` is semantically the negation `-` of the
/// positive literal `1`. A positive integer greater than the maximum value and
/// a negative integer less than or equal to the minimum value cannot be
/// represented as an integer literal, and are instead parsed as a `{float}`.
/// The minimum integer value can still be obtained through integer arithmetic.
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
    /// Converts a value to an integer. Raises an error if there is an attempt
    /// to produce an integer larger than the maximum 64-bit signed integer
    /// or smaller than the minimum 64-bit signed integer.
    ///
    /// - Booleans are converted to `0` or `1`.
    /// - Floats and decimals are rounded to the next 64-bit integer towards zero.
    /// - Strings are parsed in base 10.
    ///
    /// ```example
    /// #int(false) \
    /// #int(true) \
    /// #int(2.7) \
    /// #int(decimal("3.8")) \
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
    /// #(0).signum()
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
    /// #4.bit-not() \
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
    /// #33.bit-lshift(2) \
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
    /// #64.bit-rshift(2) \
    /// #(-8).bit-rshift(2) \
    /// #(-8).bit-rshift(2, logical: true)
    /// ```
    #[func(title = "Bitwise Right Shift")]
    pub fn bit_rshift(
        self,
        /// The amount of bits to shift. Must not be negative.
        ///
        /// Shifts larger than 63 are allowed and will cause the return value to
        /// saturate. For non-negative numbers, the return value saturates at
        /// `{0}`, while, for negative numbers, it saturates at `{-1}` if
        /// `logical` is set to `{false}`, or `{0}` if it is `{true}`. This
        /// behavior is consistent with just applying this operation multiple
        /// times. Therefore, the shift will always succeed.
        shift: u32,
        /// Toggles whether a logical (unsigned) right shift should be performed
        /// instead of arithmetic right shift.
        /// If this is `{true}`, negative operands will not preserve their sign
        /// bit, and bits which appear to the left after the shift will be
        /// `{0}`. This parameter has no effect on non-negative operands.
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

    /// Converts bytes to an integer.
    ///
    /// ```example
    /// #int.from-bytes(bytes((0, 0, 0, 0, 0, 0, 0, 1))) \
    /// #int.from-bytes(bytes((1, 0, 0, 0, 0, 0, 0, 0)), endian: "big")
    /// ```
    #[func]
    pub fn from_bytes(
        /// The bytes that should be converted to an integer.
        ///
        /// Must be of length at most 8 so that the result fits into a 64-bit
        /// signed integer.
        bytes: Bytes,
        /// The endianness of the conversion.
        #[named]
        #[default(Endianness::Little)]
        endian: Endianness,
        /// Whether the bytes should be treated as a signed integer. If this is
        /// `{true}` and the most significant bit is set, the resulting number
        /// will negative.
        #[named]
        #[default(true)]
        signed: bool,
    ) -> StrResult<i64> {
        let len = bytes.len();
        if len == 0 {
            return Ok(0);
        } else if len > 8 {
            bail!("too many bytes to convert to a 64 bit number");
        }

        // `decimal` will hold the part of the buffer that should be filled with
        // the input bytes, `rest` will remain as is or be filled with 0xFF for
        // negative numbers if signed is true.
        //
        // – big-endian: `decimal` will be the rightmost bytes of the buffer.
        // - little-endian: `decimal` will be the leftmost bytes of the buffer.
        let mut buf = [0u8; 8];
        let (rest, decimal) = match endian {
            Endianness::Big => buf.split_at_mut(8 - len),
            Endianness::Little => {
                let (first, second) = buf.split_at_mut(len);
                (second, first)
            }
        };

        decimal.copy_from_slice(bytes.as_ref());

        // Perform sign-extension if necessary.
        if signed {
            let most_significant_byte = match endian {
                Endianness::Big => decimal[0],
                Endianness::Little => decimal[len - 1],
            };

            if most_significant_byte & 0b1000_0000 != 0 {
                rest.fill(0xFF);
            }
        }

        Ok(match endian {
            Endianness::Big => i64::from_be_bytes(buf),
            Endianness::Little => i64::from_le_bytes(buf),
        })
    }

    /// Converts an integer to bytes.
    ///
    /// ```example
    /// #array(10000.to-bytes(endian: "big")) \
    /// #array(10000.to-bytes(size: 4))
    /// ```
    #[func]
    pub fn to_bytes(
        self,
        /// The endianness of the conversion.
        #[named]
        #[default(Endianness::Little)]
        endian: Endianness,
        /// The size in bytes of the resulting bytes (must be at least zero). If
        /// the integer is too large to fit in the specified size, the
        /// conversion will truncate the remaining bytes based on the
        /// endianness. To keep the same resulting value, if the endianness is
        /// big-endian, the truncation will happen at the rightmost bytes.
        /// Otherwise, if the endianness is little-endian, the truncation will
        /// happen at the leftmost bytes.
        ///
        /// Be aware that if the integer is negative and the size is not enough
        /// to make the number fit, when passing the resulting bytes to
        /// `int.from-bytes`, the resulting number might be positive, as the
        /// most significant bit might not be set to 1.
        #[named]
        #[default(8)]
        size: usize,
    ) -> Bytes {
        let array = match endian {
            Endianness::Big => self.to_be_bytes(),
            Endianness::Little => self.to_le_bytes(),
        };

        let mut buf = SmallVec::<[u8; 8]>::from_elem(0, size);
        match endian {
            Endianness::Big => {
                // Copy the bytes from the array to the buffer, starting from
                // the end of the buffer.
                let buf_start = size.saturating_sub(8);
                let array_start = 8usize.saturating_sub(size);
                buf[buf_start..].copy_from_slice(&array[array_start..])
            }
            Endianness::Little => {
                // Copy the bytes from the array to the buffer, starting from
                // the beginning of the buffer.
                let end = size.min(8);
                buf[..end].copy_from_slice(&array[..end])
            }
        }

        Bytes::new(buf)
    }
}

impl Repr for i64 {
    fn repr(&self) -> EcoString {
        eco_format!("{:?}", self)
    }
}

/// Represents the byte order used for converting integers and floats to bytes
/// and vice versa.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Endianness {
    /// Big-endian byte order: The highest-value byte is at the beginning of the
    /// bytes.
    Big,
    /// Little-endian byte order: The lowest-value byte is at the beginning of
    /// the bytes.
    Little,
}

/// A value that can be cast to an integer.
pub struct ToInt(i64);

cast! {
    ToInt,
    v: i64 => Self(v),
    v: bool => Self(v as i64),
    v: f64 => Self(convert_float_to_int(v)?),
    v: Decimal => Self(i64::try_from(v).map_err(|_| eco_format!("number too large"))?),
    v: Str => Self(parse_int(&v).map_err(|_| eco_format!("invalid integer: {}", v))?),
}

pub fn convert_float_to_int(f: f64) -> StrResult<i64> {
    if f <= i64::MIN as f64 - 1.0 || f >= i64::MAX as f64 + 1.0 {
        Err(eco_format!("number too large"))
    } else {
        Ok(f as i64)
    }
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
            self => {
                #[allow(irrefutable_let_patterns)]
                if let Ok(int) = i64::try_from(self) {
                    Value::Int(int)
                } else {
                    // Some u64 are too large to be cast as i64
                    // In that case, we accept that there may be a
                    // precision loss, and use a floating point number
                    Value::Float(self as _)
                }
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

cast! {
    NonZeroU32,
    self => Value::Int(self.get() as _),
    v: i64 => v
        .try_into()
        .and_then(|v: u32| v.try_into())
        .map_err(|_| if v <= 0 {
            "number must be positive"
        } else {
            "number too large"
        })?,
}
