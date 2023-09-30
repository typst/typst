use ecow::{eco_format, EcoString};

/// Format an integer in a base.
pub fn format_int_with_base(mut n: i64, base: i64) -> EcoString {
    if n == 0 {
        return "0".into();
    }

    // In Rust, `format!("{:x}", -14i64)` is not `-e` but `fffffffffffffff2`.
    // So we can only use the built-in for decimal, not bin/oct/hex.
    if base == 10 {
        return eco_format!("{n}");
    }

    // The largest output is `to_base(i64::MIN, 2)`, which is 65 chars long.
    const SIZE: usize = 65;
    let mut digits = [b'\0'; SIZE];
    let mut i = SIZE;

    // It's tempting to take the absolute value, but this will fail for i64::MIN.
    // Instead, we turn n negative, as -i64::MAX is perfectly representable.
    let negative = n < 0;
    if n > 0 {
        n = -n;
    }

    while n != 0 {
        let digit = char::from_digit(-(n % base) as u32, base as u32);
        i -= 1;
        digits[i] = digit.unwrap_or('?') as u8;
        n /= base;
    }

    if negative {
        i -= 1;
        digits[i] = b'-';
    }

    std::str::from_utf8(&digits[i..]).unwrap_or_default().into()
}

/// A trait for values that can be represented by a string.
pub trait Repr {
    /// Returns a string representation of the value.
    fn repr(&self) -> EcoString;
}

impl Repr for i64 {
    fn repr(&self) -> EcoString {
        eco_format!("{}", self)
    }
}

impl Repr for f64 {
    fn repr(&self) -> EcoString {
        eco_format!("{}", self)
    }
}
