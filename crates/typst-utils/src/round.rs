/// Returns value with `n` digits after floating point where `n` is `precision`.
/// Standard rounding rules apply (if `n+1`th digit >= 5, round up).
///
/// If rounding the `value` will have no effect (e.g., it's infinite or NaN),
/// returns `value` unchanged.
///
/// # Examples
///
/// ```
/// # use typst_utils::round_with_precision;
/// let rounded = round_with_precision(-0.56553, 2);
/// assert_eq!(-0.57, rounded);
/// ```
pub fn round_with_precision(value: f64, precision: u8) -> f64 {
    // Don't attempt to round the float if that wouldn't have any effect.
    // This includes infinite or NaN values, as well as integer values
    // with a filled mantissa (which can't have a fractional part).
    // Rounding with a precision larger than the amount of digits that can be
    // effectively represented would also be a no-op. Given that, the check
    // below ensures we won't proceed if `|value| >= 2^53` or if
    // `precision >= 15`, which also ensures the multiplication by `offset`
    // won't return `inf`, since `2^53 * 10^15` (larger than any possible
    // `value * offset` multiplication) does not.
    if value.is_infinite()
        || value.is_nan()
        || value.abs() >= (1_i64 << f64::MANTISSA_DIGITS) as f64
        || precision as u32 >= f64::DIGITS
    {
        return value;
    }
    let offset = 10_f64.powi(precision.into());
    assert!((value * offset).is_finite(), "{value} * {offset} is not finite!");
    (value * offset).round() / offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_with_precision_0() {
        let round = |value| round_with_precision(value, 0);
        assert_eq!(0.0, round(0.0));
        assert_eq!(-0.0, round(-0.0));
        assert_eq!(0.0, round(0.4));
        assert_eq!(-0.0, round(-0.4));
        assert_eq!(1.0, round(0.56453));
        assert_eq!(-1.0, round(-0.56453));
    }

    #[test]
    fn test_round_with_precision_1() {
        let round = |value| round_with_precision(value, 1);
        assert_eq!(0.0, round(0.0));
        assert_eq!(-0.0, round(-0.0));
        assert_eq!(0.4, round(0.4));
        assert_eq!(-0.4, round(-0.4));
        assert_eq!(0.4, round(0.44));
        assert_eq!(-0.4, round(-0.44));
        assert_eq!(0.6, round(0.56453));
        assert_eq!(-0.6, round(-0.56453));
        assert_eq!(1.0, round(0.96453));
        assert_eq!(-1.0, round(-0.96453));
    }

    #[test]
    fn test_round_with_precision_2() {
        let round = |value| round_with_precision(value, 2);
        assert_eq!(0.0, round(0.0));
        assert_eq!(-0.0, round(-0.0));
        assert_eq!(0.4, round(0.4));
        assert_eq!(-0.4, round(-0.4));
        assert_eq!(0.44, round(0.44));
        assert_eq!(-0.44, round(-0.44));
        assert_eq!(0.44, round(0.444));
        assert_eq!(-0.44, round(-0.444));
        assert_eq!(0.57, round(0.56553));
        assert_eq!(-0.57, round(-0.56553));
        assert_eq!(1.0, round(0.99553));
        assert_eq!(-1.0, round(-0.99553));
    }

    #[test]
    fn test_round_with_precision_fuzzy() {
        let round = |value| round_with_precision(value, 0);
        assert_eq!(f64::INFINITY, round(f64::INFINITY));
        assert_eq!(f64::NEG_INFINITY, round(f64::NEG_INFINITY));
        assert!(round(f64::NAN).is_nan());

        let max_int = (1_i64 << f64::MANTISSA_DIGITS) as f64;
        let f64_digits = f64::DIGITS as u8;

        // max
        assert_eq!(max_int, round(max_int));
        assert_eq!(0.123456, round_with_precision(0.123456, f64_digits));
        assert_eq!(max_int, round_with_precision(max_int, f64_digits));

        // max - 1
        assert_eq!(max_int - 1f64, round(max_int - 1f64));
        assert_eq!(0.123456, round_with_precision(0.123456, f64_digits - 1));
        assert_eq!(max_int - 1f64, round_with_precision(max_int - 1f64, f64_digits));
        assert_eq!(max_int, round_with_precision(max_int, f64_digits - 1));
        assert_eq!(max_int - 1f64, round_with_precision(max_int - 1f64, f64_digits - 1));
    }
}
