//! Debug representation of values.

use ecow::{eco_format, EcoString};

use crate::foundations::{func, Str, Value};
use crate::utils::round_with_precision;

/// The Unicode minus sign.
pub const MINUS_SIGN: &str = "\u{2212}";

/// Returns the string representation of a value.
///
/// When inserted into content, most values are displayed as this representation
/// in monospace with syntax-highlighting. The exceptions are `{none}`,
/// integers, floats, strings, content, and functions.
///
/// **Note:** This function is for debugging purposes. Its output should not be
/// considered stable and may change at any time!
///
/// # Example
/// ```example
/// #none vs #repr(none) \
/// #"hello" vs #repr("hello") \
/// #(1, 2) vs #repr((1, 2)) \
/// #[*Hi*] vs #repr([*Hi*])
/// ```
#[func(title = "Representation")]
pub fn repr(
    /// The value whose string representation to produce.
    value: Value,
) -> Str {
    value.repr().into()
}

/// A trait that defines the `repr` of a Typst value.
pub trait Repr {
    /// Return the debug representation of the value.
    fn repr(&self) -> EcoString;
}

/// Format an integer in a base.
pub fn format_int_with_base(mut n: i64, base: i64) -> EcoString {
    if n == 0 {
        return "0".into();
    }

    // The largest output is `to_base(i64::MIN, 2)`, which is 64 bytes long,
    // plus the length of the minus sign.
    const SIZE: usize = 64 + MINUS_SIGN.len();
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
        let prev = i;
        i -= MINUS_SIGN.len();
        digits[i..prev].copy_from_slice(MINUS_SIGN.as_bytes());
    }

    std::str::from_utf8(&digits[i..]).unwrap_or_default().into()
}

/// Converts a float to a string representation with a specific precision and a
/// unit, all with a single allocation.
///
/// The returned string is always valid Typst code. As such, it might not be a
/// float literal. For example, it may return `"float.inf"`.
pub fn format_float(
    mut value: f64,
    precision: Option<u8>,
    force_separator: bool,
    unit: &str,
) -> EcoString {
    if let Some(p) = precision {
        value = round_with_precision(value, p as i16);
    }
    // Debug for f64 always prints a decimal separator, while Display only does
    // when necessary.
    let unit_multiplication = if unit.is_empty() { "" } else { " * 1" };
    if value.is_nan() {
        eco_format!("float.nan{unit_multiplication}{unit}")
    } else if value.is_infinite() {
        let sign = if value < 0.0 { "-" } else { "" };
        eco_format!("{sign}float.inf{unit_multiplication}{unit}")
    } else if force_separator {
        eco_format!("{value:?}{unit}")
    } else {
        eco_format!("{value}{unit}")
    }
}

/// Converts a float to a string representation with a precision of three
/// decimal places. This is intended to be used as part of a larger structure
/// containing multiple float components, such as colors.
pub fn format_float_component(value: f64) -> EcoString {
    format_float(value, Some(3), false, "")
}

/// Converts a float to a string representation with a precision of two decimal
/// places, followed by a unit.
pub fn format_float_with_unit(value: f64, unit: &str) -> EcoString {
    format_float(value, Some(2), false, unit)
}

/// Converts a float to a string that can be used to display the float as text.
pub fn display_float(value: f64) -> EcoString {
    if value.is_nan() {
        "NaN".into()
    } else if value.is_infinite() {
        let sign = if value < 0.0 { MINUS_SIGN } else { "" };
        eco_format!("{sign}∞")
    } else if value < 0.0 {
        eco_format!("{}{}", MINUS_SIGN, value.abs())
    } else {
        eco_format!("{}", value.abs())
    }
}

/// Formats pieces separated with commas and a final "and" or "or".
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

/// Formats a comma-separated list.
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

/// Formats an array-like construct.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_base() {
        assert_eq!(&format_int_with_base(0, 10), "0");
        assert_eq!(&format_int_with_base(0, 16), "0");
        assert_eq!(&format_int_with_base(0, 36), "0");
        assert_eq!(
            &format_int_with_base(i64::MAX, 2),
            "111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(
            &format_int_with_base(i64::MIN, 2),
            "\u{2212}1000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(&format_int_with_base(i64::MAX, 10), "9223372036854775807");
        assert_eq!(&format_int_with_base(i64::MIN, 10), "\u{2212}9223372036854775808");
        assert_eq!(&format_int_with_base(i64::MAX, 16), "7fffffffffffffff");
        assert_eq!(&format_int_with_base(i64::MIN, 16), "\u{2212}8000000000000000");
        assert_eq!(&format_int_with_base(i64::MAX, 36), "1y2p0ij32e8e7");
        assert_eq!(&format_int_with_base(i64::MIN, 36), "\u{2212}1y2p0ij32e8e8");
    }
}
