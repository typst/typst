use ecow::{eco_format, EcoString};

pub fn to_base(mut n: i64, base: i64) -> EcoString {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_base() {
        assert_eq!(&to_base(0, 10), "0");
        assert_eq!(&to_base(0, 16), "0");
        assert_eq!(&to_base(0, 36), "0");
        assert_eq!(
            &to_base(i64::MAX, 2),
            "111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(
            &to_base(i64::MIN, 2),
            "-1000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(&to_base(i64::MAX, 10), "9223372036854775807");
        assert_eq!(&to_base(i64::MIN, 10), "-9223372036854775808");
        assert_eq!(&to_base(i64::MAX, 16), "7fffffffffffffff");
        assert_eq!(&to_base(i64::MIN, 16), "-8000000000000000");
        assert_eq!(&to_base(i64::MAX, 36), "1y2p0ij32e8e7");
        assert_eq!(&to_base(i64::MIN, 36), "-1y2p0ij32e8e8");
    }
}
