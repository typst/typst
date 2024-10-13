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
        Some(value)
    } else {
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
        let f64_digits = f64::DIGITS as i16;

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

    #[test]
    fn test_round_with_precision_negative_1() {
        let round = |value| round_with_precision(value, -1);
        assert_eq!(0.0, round(0.0));
        assert_eq!(-0.0, round(-0.0));
        assert_eq!(0.0, round(0.4));
        assert_eq!(-0.0, round(-0.4));
        assert_eq!(1230.0, round(1234.5));
        assert_eq!(-1230.0, round(-1234.5));
        assert_eq!(1250.0, round(1245.232));
        assert_eq!(-1250.0, round(-1245.232));
    }

    #[test]
    fn test_round_with_precision_negative_2() {
        let round = |value| round_with_precision(value, -2);
        assert_eq!(0.0, round(0.0));
        assert_eq!(-0.0, round(-0.0));
        assert_eq!(0.0, round(0.4));
        assert_eq!(-0.0, round(-0.4));
        assert_eq!(1200.0, round(1243.232));
        assert_eq!(-1200.0, round(-1243.232));
        assert_eq!(1300.0, round(1253.232));
        assert_eq!(-1300.0, round(-1253.232));
    }

    #[test]
    fn test_round_with_precision_fuzzy_negative() {
        let round = |value| round_with_precision(value, -1);
        assert_eq!(f64::INFINITY, round(f64::INFINITY));
        assert_eq!(f64::NEG_INFINITY, round(f64::NEG_INFINITY));
        assert!(round(f64::NAN).is_nan());

        let max_int_digits = f64::MAX_10_EXP as i16;
        let ten_exp = |exponent: i16| 10_f64.powi(exponent.into());

        // max
        assert_eq!(f64::INFINITY, round_with_precision(f64::MAX, -max_int_digits));
        assert_eq!(f64::NEG_INFINITY, round_with_precision(f64::MIN, -max_int_digits));
        assert_eq!(
            f64::INFINITY,
            round_with_precision(1.66 * ten_exp(max_int_digits), -max_int_digits)
        );
        assert_eq!(
            f64::NEG_INFINITY,
            round_with_precision(-1.66 * ten_exp(max_int_digits), -max_int_digits)
        );
        assert_eq!(
            0.0,
            round_with_precision(1.66 * ten_exp(max_int_digits - 1), -max_int_digits)
        );
        assert_eq!(
            -0.0,
            round_with_precision(-1.66 * ten_exp(max_int_digits - 1), -max_int_digits)
        );
        assert_eq!(0.0, round_with_precision(1234.5678, -max_int_digits));
        assert_eq!(-0.0, round_with_precision(-1234.5678, -max_int_digits));

        // max - 1
        assert_eq!(f64::INFINITY, round_with_precision(f64::MAX, -(max_int_digits - 1)));
        assert_eq!(
            f64::NEG_INFINITY,
            round_with_precision(f64::MIN, -(max_int_digits - 1))
        );
        assert_eq!(
            // Must be approx equal to 1.7e308.
            //
            // Using some division and flooring to avoid weird results due to
            // imprecision.
            17.0,
            (round_with_precision(1.66 * ten_exp(max_int_digits), -(max_int_digits - 1))
                / ten_exp(max_int_digits - 1))
            .floor()
        );
        assert_eq!(
            // Must be approx equal to -1.7e308.
            //
            // Using some division and flooring to avoid weird results due to
            // imprecision.
            -17.0,
            (round_with_precision(
                -1.66 * ten_exp(max_int_digits),
                -(max_int_digits - 1)
            ) / ten_exp(max_int_digits - 1))
            .floor()
        );
        assert_eq!(
            2.0 * ten_exp(max_int_digits - 1),
            round_with_precision(
                1.66 * ten_exp(max_int_digits - 1),
                -(max_int_digits - 1)
            )
        );
        assert_eq!(
            -2.0 * ten_exp(max_int_digits - 1),
            round_with_precision(
                -1.66 * ten_exp(max_int_digits - 1),
                -(max_int_digits - 1)
            )
        );
        assert_eq!(0.0, round_with_precision(1234.5678, -(max_int_digits - 1)));
        assert_eq!(-0.0, round_with_precision(-1234.5678, -(max_int_digits - 1)));

        // max + 1
        assert_eq!(0.0, round_with_precision(f64::MAX, -(max_int_digits + 1)));
        assert_eq!(-0.0, round_with_precision(f64::MIN, -(max_int_digits + 1)));
        assert_eq!(
            0.0,
            round_with_precision(1.66 * ten_exp(max_int_digits), -(max_int_digits + 1))
        );
        assert_eq!(
            -0.0,
            round_with_precision(-1.66 * ten_exp(max_int_digits), -(max_int_digits + 1))
        );
        assert_eq!(
            0.0,
            round_with_precision(
                1.66 * ten_exp(max_int_digits - 1),
                -(max_int_digits + 1)
            )
        );
        assert_eq!(
            -0.0,
            round_with_precision(
                -1.66 * ten_exp(max_int_digits - 1),
                -(max_int_digits + 1)
            )
        );
        assert_eq!(0.0, round_with_precision(1234.5678, -(max_int_digits + 1)));
        assert_eq!(-0.0, round_with_precision(-1234.5678, -(max_int_digits + 1)));
    }

    #[test]
    fn test_round_int_with_precision_positive() {
        assert_eq!(Some(0), round_int_with_precision(0, 0));
        assert_eq!(Some(10), round_int_with_precision(10, 0));
        assert_eq!(Some(23), round_int_with_precision(23, 235));
        assert_eq!(Some(i64::MAX), round_int_with_precision(i64::MAX, 235));
    }

    #[test]
    fn test_round_int_with_precision_negative_1() {
        let round = |value| round_int_with_precision(value, -1);
        assert_eq!(Some(0), round(0));
        assert_eq!(Some(0), round(3));
        assert_eq!(Some(10), round(5));
        assert_eq!(Some(10), round(13));
        assert_eq!(Some(1230), round(1234));
        assert_eq!(Some(-1230), round(-1234));
        assert_eq!(Some(1250), round(1245));
        assert_eq!(Some(-1250), round(-1245));
        assert_eq!(None, round(i64::MAX));
        assert_eq!(None, round(i64::MIN));
    }

    #[test]
    fn test_round_int_with_precision_negative_2() {
        let round = |value| round_int_with_precision(value, -2);
        assert_eq!(Some(0), round(0));
        assert_eq!(Some(0), round(3));
        assert_eq!(Some(0), round(5));
        assert_eq!(Some(0), round(13));
        assert_eq!(Some(1200), round(1243));
        assert_eq!(Some(-1200), round(-1243));
        assert_eq!(Some(1300), round(1253));
        assert_eq!(Some(-1300), round(-1253));
        assert_eq!(Some(i64::MAX - 7), round(i64::MAX));
        assert_eq!(Some(i64::MIN + 8), round(i64::MIN));
    }
}
