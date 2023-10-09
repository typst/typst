use std::num::{NonZeroI64, NonZeroIsize, NonZeroU64, NonZeroUsize, ParseIntError};

use crate::util::fmt::{format_int_with_base, MINUS_SIGN};
use ecow::{eco_format, EcoString};

use super::{cast, func, scope, ty, Repr, Str, Value};

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
#[ty(scope, name = "int", title = "Integer")]
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
}

impl Repr for i64 {
    fn repr(&self) -> EcoString {
        format_int_with_base(*self, 10)
    }
}

/// A value that can be cast to an integer.
pub struct ToInt(i64);

cast! {
    ToInt,
    v: bool => Self(v as i64),
    v: f64 => Self(v as i64),
    v: Str => Self(parse_int(&v).map_err(|_| eco_format!("invalid integer: {}", v))?),
    v: i64 => Self(v),
}

fn parse_int(mut s: &str) -> Result<i64, ParseIntError> {
    let mut sign = 1;
    if let Some(rest) = s.strip_prefix('-').or_else(|| s.strip_prefix(MINUS_SIGN)) {
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
            self => Value::Int(self as _),
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
