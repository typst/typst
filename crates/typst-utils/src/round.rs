/// Returns value with `n` digits after floating point where `n` is `precision`.
/// Standard rounding rules apply (if `n+1`th digit >= 5, round away from zero).
///
/// If `precision` is negative, returns value with `n` less significant integer
/// digits before floating point where `n` is `-precision`. Standard rounding
/// rules apply to the first remaining significant digit (if `n`th digit from
/// the floating point >= 5, round away from zero).
///
/// If rounding the `value` will have no effect (e.g., it's infinite or NaN),
/// returns `value` unchanged.
///
/// Note that rounding with negative precision may return plus or minus
/// infinity if the result would overflow or underflow (respectively) the range
/// of floating-point numbers.
///
/// # Examples
///
/// ```
/// # use typst_utils::round_with_precision;
/// let rounded = round_with_precision(-0.56553, 2);
/// assert_eq!(-0.57, rounded);
///
/// let rounded_negative = round_with_precision(823543.0, -3);
/// assert_eq!(824000.0, rounded_negative);
/// ```
pub fn round_with_precision(value: f64, precision: i16) -> f64 {
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
        || precision >= 0 && value.abs() >= (1_i64 << f64::MANTISSA_DIGITS) as f64
        || precision >= f64::DIGITS as i16
    {
        return value;
    }
    // Floats cannot have more than this amount of base-10 integer digits.
    if precision < -(f64::MAX_10_EXP as i16) {
        // Multiply by zero to ensure sign is kept.
        return value * 0.0;
    }
    if precision > 0 {
        let offset = 10_f64.powi(precision.into());
        assert!((value * offset).is_finite(), "{value} * {offset} is not finite!");
        (value * offset).round() / offset
    } else {
        // Divide instead of multiplying by a negative exponent given that
        // `f64::MAX_10_EXP` is larger than `f64::MIN_10_EXP` in absolute value
        // (|308| > |-307|), allowing for the precision of -308 to be used.
        let offset = 10_f64.powi((-precision).into());
        (value / offset).round() * offset
    }
}

/// This is used for rounding into integer digits, and is a no-op for positive
/// `precision`.
///
/// If `precision` is negative, returns value with `n` less significant integer
/// digits from the first digit where `n` is `-precision`. Standard rounding
/// rules apply to the first remaining significant digit (if `n`th digit from
/// the first digit >= 5, round away from zero).
///
/// Note that this may return `None` for negative precision when rounding
/// beyond [`i64::MAX`] or [`i64::MIN`].
///
/// # Examples
///
/// ```
/// # use typst_utils::round_int_with_precision;
/// let rounded = round_int_with_precision(-154, -2);
/// assert_eq!(Some(-200), rounded);
///
/// let rounded = round_int_with_precision(823543, -3);
/// assert_eq!(Some(824000), rounded);
/// ```
pub fn round_int_with_precision(value: i64, precision: i16) -> Option<i64> {
    if precision >= 0 {
        return Some(value);
    }

    let digits = -precision as u32;
    let Some(ten_to_digits) = 10i64.checked_pow(digits - 1) else {
        // Larger than any possible amount of integer digits.
        return Some(0);
    };

    // Divide by 10^(digits - 1).
    //
    // We keep the last digit we want to remove as the first digit of this
    // number, so we can check it with mod 10 for rounding purposes.
    let truncated = value / ten_to_digits;
    if truncated == 0 {
        return Some(0);
    }

    let rounded = if (truncated % 10).abs() >= 5 {
        // Round away from zero (towards the next multiple of 10).
        //
        // This may overflow in the particular case of rounding MAX/MIN
        // with -1.
        truncated.checked_add(truncated.signum() * (10 - (truncated % 10).abs()))?
    } else {
        // Just replace the last digit with zero, since it's < 5.
        truncated - (truncated % 10)
    };

    // Multiply back by 10^(digits - 1).
    //
    // May overflow / underflow, in which case we fail.
    rounded.checked_mul(ten_to_digits)
}

#[cfg(test)]
mod tests {
    use super::{round_int_with_precision as rip, round_with_precision as rp};

    #[test]
    fn test_round_with_precision_0() {
        let round = |value| rp(value, 0);
        assert_eq!(round(0.0), 0.0);
        assert_eq!(round(-0.0), -0.0);
        assert_eq!(round(0.4), 0.0);
        assert_eq!(round(-0.4), -0.0);
        assert_eq!(round(0.56453), 1.0);
        assert_eq!(round(-0.56453), -1.0);
    }

    #[test]
    fn test_round_with_precision_1() {
        let round = |value| rp(value, 1);
        assert_eq!(round(0.0), 0.0);
        assert_eq!(round(-0.0), -0.0);
        assert_eq!(round(0.4), 0.4);
        assert_eq!(round(-0.4), -0.4);
        assert_eq!(round(0.44), 0.4);
        assert_eq!(round(-0.44), -0.4);
        assert_eq!(round(0.56453), 0.6);
        assert_eq!(round(-0.56453), -0.6);
        assert_eq!(round(0.96453), 1.0);
        assert_eq!(round(-0.96453), -1.0);
    }

    #[test]
    fn test_round_with_precision_2() {
        let round = |value| rp(value, 2);
        assert_eq!(round(0.0), 0.0);
        assert_eq!(round(-0.0), -0.0);
        assert_eq!(round(0.4), 0.4);
        assert_eq!(round(-0.4), -0.4);
        assert_eq!(round(0.44), 0.44);
        assert_eq!(round(-0.44), -0.44);
        assert_eq!(round(0.444), 0.44);
        assert_eq!(round(-0.444), -0.44);
        assert_eq!(round(0.56553), 0.57);
        assert_eq!(round(-0.56553), -0.57);
        assert_eq!(round(0.99553), 1.0);
        assert_eq!(round(-0.99553), -1.0);
    }

    #[test]
    fn test_round_with_precision_negative_1() {
        let round = |value| rp(value, -1);
        assert_eq!(round(0.0), 0.0);
        assert_eq!(round(-0.0), -0.0);
        assert_eq!(round(0.4), 0.0);
        assert_eq!(round(-0.4), -0.0);
        assert_eq!(round(1234.5), 1230.0);
        assert_eq!(round(-1234.5), -1230.0);
        assert_eq!(round(1245.232), 1250.0);
        assert_eq!(round(-1245.232), -1250.0);
    }

    #[test]
    fn test_round_with_precision_negative_2() {
        let round = |value| rp(value, -2);
        assert_eq!(round(0.0), 0.0);
        assert_eq!(round(-0.0), -0.0);
        assert_eq!(round(0.4), 0.0);
        assert_eq!(round(-0.4), -0.0);
        assert_eq!(round(1243.232), 1200.0);
        assert_eq!(round(-1243.232), -1200.0);
        assert_eq!(round(1253.232), 1300.0);
        assert_eq!(round(-1253.232), -1300.0);
    }

    #[test]
    fn test_round_with_precision_fuzzy() {
        let max_int = (1_i64 << f64::MANTISSA_DIGITS) as f64;
        let max_digits = f64::DIGITS as i16;

        // Special cases.
        assert_eq!(rp(f64::INFINITY, 0), f64::INFINITY);
        assert_eq!(rp(f64::NEG_INFINITY, 0), f64::NEG_INFINITY);
        assert!(rp(f64::NAN, 0).is_nan());

        // Max
        assert_eq!(rp(max_int, 0), max_int);
        assert_eq!(rp(0.123456, max_digits), 0.123456);
        assert_eq!(rp(max_int, max_digits), max_int);

        // Max - 1
        assert_eq!(rp(max_int - 1.0, 0), max_int - 1.0);
        assert_eq!(rp(0.123456, max_digits - 1), 0.123456);
        assert_eq!(rp(max_int - 1.0, max_digits), max_int - 1.0);
        assert_eq!(rp(max_int, max_digits - 1), max_int);
        assert_eq!(rp(max_int - 1.0, max_digits - 1), max_int - 1.0);
    }

    #[test]
    fn test_round_with_precision_fuzzy_negative() {
        let exp10 = |exponent: i16| 10_f64.powi(exponent.into());
        let max_digits = f64::MAX_10_EXP as i16;
        let max_up = max_digits + 1;
        let max_down = max_digits - 1;

        // Special cases.
        assert_eq!(rp(f64::INFINITY, -1), f64::INFINITY);
        assert_eq!(rp(f64::NEG_INFINITY, -1), f64::NEG_INFINITY);
        assert!(rp(f64::NAN, -1).is_nan());

        // Max
        assert_eq!(rp(f64::MAX, -max_digits), f64::INFINITY);
        assert_eq!(rp(f64::MIN, -max_digits), f64::NEG_INFINITY);
        assert_eq!(rp(1.66 * exp10(max_digits), -max_digits), f64::INFINITY);
        assert_eq!(rp(-1.66 * exp10(max_digits), -max_digits), f64::NEG_INFINITY);
        assert_eq!(rp(1.66 * exp10(max_down), -max_digits), 0.0);
        assert_eq!(rp(-1.66 * exp10(max_down), -max_digits), -0.0);
        assert_eq!(rp(1234.5678, -max_digits), 0.0);
        assert_eq!(rp(-1234.5678, -max_digits), -0.0);

        // Max + 1
        assert_eq!(rp(f64::MAX, -max_up), 0.0);
        assert_eq!(rp(f64::MIN, -max_up), -0.0);
        assert_eq!(rp(1.66 * exp10(max_digits), -max_up), 0.0);
        assert_eq!(rp(-1.66 * exp10(max_digits), -max_up), -0.0);
        assert_eq!(rp(1.66 * exp10(max_down), -max_up), 0.0);
        assert_eq!(rp(-1.66 * exp10(max_down), -max_up), -0.0);
        assert_eq!(rp(1234.5678, -max_up), 0.0);
        assert_eq!(rp(-1234.5678, -max_up), -0.0);

        // Max - 1
        assert_eq!(rp(f64::MAX, -max_down), f64::INFINITY);
        assert_eq!(rp(f64::MIN, -max_down), f64::NEG_INFINITY);
        assert_eq!(rp(1.66 * exp10(max_down), -max_down), 2.0 * exp10(max_down));
        assert_eq!(rp(-1.66 * exp10(max_down), -max_down), -2.0 * exp10(max_down));
        assert_eq!(rp(1234.5678, -max_down), 0.0);
        assert_eq!(rp(-1234.5678, -max_down), -0.0);

        // Must be approx equal to 1.7e308. Using some division and flooring
        // to avoid weird results due to imprecision.
        assert_eq!(
            (rp(1.66 * exp10(max_digits), -max_down) / exp10(max_down)).floor(),
            17.0,
        );
        assert_eq!(
            (rp(-1.66 * exp10(max_digits), -max_down) / exp10(max_down)).floor(),
            -17.0,
        );
    }

    #[test]
    fn test_round_int_with_precision_positive() {
        assert_eq!(rip(0, 0), Some(0));
        assert_eq!(rip(10, 0), Some(10));
        assert_eq!(rip(23, 235), Some(23));
        assert_eq!(rip(i64::MAX, 235), Some(i64::MAX));
    }

    #[test]
    fn test_round_int_with_precision_negative_1() {
        let round = |value| rip(value, -1);
        assert_eq!(round(0), Some(0));
        assert_eq!(round(3), Some(0));
        assert_eq!(round(5), Some(10));
        assert_eq!(round(13), Some(10));
        assert_eq!(round(1234), Some(1230));
        assert_eq!(round(-1234), Some(-1230));
        assert_eq!(round(1245), Some(1250));
        assert_eq!(round(-1245), Some(-1250));
        assert_eq!(round(i64::MAX), None);
        assert_eq!(round(i64::MIN), None);
    }

    #[test]
    fn test_round_int_with_precision_negative_2() {
        let round = |value| rip(value, -2);
        assert_eq!(round(0), Some(0));
        assert_eq!(round(3), Some(0));
        assert_eq!(round(5), Some(0));
        assert_eq!(round(13), Some(0));
        assert_eq!(round(1245), Some(1200));
        assert_eq!(round(-1245), Some(-1200));
        assert_eq!(round(1253), Some(1300));
        assert_eq!(round(-1253), Some(-1300));
        assert_eq!(round(i64::MAX), Some(i64::MAX - 7));
        assert_eq!(round(i64::MIN), Some(i64::MIN + 8));
    }
}
