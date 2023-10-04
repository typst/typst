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

/// Converts a float to a string representation with a specific precision and a
/// suffix, all with a single allocation.
pub fn format_float(mut value: f64, precision: Option<u8>, suffix: &str) -> EcoString {
    if let Some(p) = precision {
        let offset = 10_f64.powi(p as i32);
        value = (value * offset).round() / offset;
    }
    eco_format!("{}{}", value, suffix)
}

/// Format pieces separated with commas and a final "and" or "or".
pub fn separated_list(pieces: &[impl AsRef<str>], last: &str) -> String {
    let mut buf = String::new();
    for (i, part) in pieces.iter().enumerate() {
        match i {
            0 => {}
            1 if pieces.len() == 2 => {
                buf.push(' ');
                buf.push_str(last);
                buf.push(' ');
            }
            i if i + 1 == pieces.len() => {
                buf.push_str(", ");
                buf.push_str(last);
                buf.push(' ');
            }
            _ => buf.push_str(", "),
        }
        buf.push_str(part.as_ref());
    }
    buf
}

/// Format a comma-separated list.
///
/// Tries to format horizontally, but falls back to vertical formatting if the
/// pieces are too long.
pub fn pretty_comma_list(pieces: &[impl AsRef<str>], trailing_comma: bool) -> String {
    const MAX_WIDTH: usize = 50;

    let mut buf = String::new();
    let len = pieces.iter().map(|s| s.as_ref().len()).sum::<usize>()
        + 2 * pieces.len().saturating_sub(1);

    if len <= MAX_WIDTH {
        for (i, piece) in pieces.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            buf.push_str(piece.as_ref());
        }
        if trailing_comma {
            buf.push(',');
        }
    } else {
        for piece in pieces {
            buf.push_str(piece.as_ref().trim());
            buf.push_str(",\n");
        }
    }

    buf
}

/// Format an array-like construct.
///
/// Tries to format horizontally, but falls back to vertical formatting if the
/// pieces are too long.
pub fn pretty_array_like(parts: &[impl AsRef<str>], trailing_comma: bool) -> String {
    let list = pretty_comma_list(parts, trailing_comma);
    let mut buf = String::new();
    buf.push('(');
    if list.contains('\n') {
        buf.push('\n');
        for (i, line) in list.lines().enumerate() {
            if i > 0 {
                buf.push('\n');
            }
            buf.push_str("  ");
            buf.push_str(line);
        }
        buf.push('\n');
    } else {
        buf.push_str(&list);
    }
    buf.push(')');
    buf
}
