//! Different formatting functions.

use std::time::Duration;

/// Returns value with `n` digits after floating point where `n` is `precision`.
/// Standard rounding rules apply (if `n+1`th digit >= 5, round up).
///
/// If rounding the `value` will have no effect (e.g., it's infinite or NaN),
/// returns `value` unchanged.
///
/// # Examples
///
/// ```
/// # use typst_utils::format::round_with_precision;
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

/// Returns `(days, hours, minutes, seconds, milliseconds, microseconds)`.
fn get_duration_parts(duration: &Duration) -> (u64, u8, u8, u8, u16, u16) {
    // In practice we probably don't need nanoseconds.
    let micros = duration.as_micros();
    let (millis, micros) = (micros / 1000, (micros % 1000) as u16);
    let (sec, millis) = (millis / 1000, (millis % 1000) as u16);
    let (mins, sec) = (sec / 60, (sec % 60) as u8);
    let (hours, mins) = (mins / 60, (mins % 60) as u8);
    let (days, hours) = ((hours / 24) as u64, (hours % 24) as u8);
    (days, hours, mins, sec, millis, micros)
}

/// Format string using `days`, `hours`, `minutes`, `seconds`.
fn format_dhms(days: u64, hours: u8, minutes: u8, seconds: u8) -> String {
    match (days, hours, minutes, seconds) {
        (0, 0, 0, s) => format!("{s:2} s"),
        (0, 0, m, s) => format!("{m:2} m {s:2} s"),
        (0, h, m, s) => format!("{h:2} h {m:2} m {s:2} s"),
        (d, h, m, s) => format!("{d:3} d {h:2} h {m:2} m {s:2} s"),
    }
}

/// Format string starting with number of seconds and going bigger from there.
///
/// # Examples
///
/// ```
/// # use std::time::Duration;
/// # use typst_utils::format::time_starting_with_seconds;
/// let duration1 = time_starting_with_seconds(&Duration::from_secs(0));
/// assert_eq!(" 0 s", &duration1);
///
/// let duration2 = time_starting_with_seconds(&Duration::from_secs(
///     24 * 60 * 60 * 100 + // days
///     60 * 60 * 10 + // hours
///     60 * 10 + // minutes
///     10 // seconds
/// ));
/// assert_eq!("100 d 10 h 10 m 10 s", &duration2);
/// ```
pub fn time_starting_with_seconds(duration: &Duration) -> String {
    let (days, hours, minutes, seconds, _, _) = get_duration_parts(duration);
    format_dhms(days, hours, minutes, seconds)
}

/// Format string starting with number of milliseconds and going bigger
/// from there. `precision` is how many digits of microseconds
/// from floating point to the right will be preserved (with rounding).
/// Keep in mind that this function will always remove all trailing zeros
/// for microseconds.
///
/// Note: if duration is 1 second or longer, then output will be identical
/// to [time_starting_with_seconds], which also means that precision,
/// number of milliseconds and microseconds will not be used.
///
/// # Examples
///
/// ```
/// # use std::time::Duration;
/// # use typst_utils::format::time_starting_with_milliseconds;
/// let duration1 = time_starting_with_milliseconds(&Duration::from_micros(
///     123 * 1000 + // milliseconds
///     456 // microseconds
/// ), 2);
/// assert_eq!("123.46 ms", &duration1);
///
/// let duration2 = time_starting_with_milliseconds(&Duration::from_micros(
///     123 * 1000 // milliseconds
/// ), 2);
/// assert_eq!("123 ms", &duration2);
///
/// let duration3 = time_starting_with_milliseconds(&Duration::from_secs(1), 2);
/// assert_eq!(" 1 s", &duration3);
/// ```
pub fn time_starting_with_milliseconds(duration: &Duration, precision: u8) -> String {
    let (d, h, m, s, ms, mcs) = get_duration_parts(duration);
    match (d, h, m, s) {
        (0, 0, 0, 0) => {
            let ms_mcs = ms as f64 + mcs as f64 / 1000.0;
            format!("{} ms", round_with_precision(ms_mcs, precision))
        }
        (d, h, m, s) => format_dhms(d, h, m, s),
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

    fn duration_from_milli_micro(milliseconds: u16, microseconds: u16) -> Duration {
        let microseconds = microseconds as u64;
        let milliseconds = 1000 * milliseconds as u64;
        Duration::from_micros(milliseconds + microseconds)
    }

    #[test]
    fn test_time_starting_with_seconds() {
        let f = |duration| time_starting_with_seconds(&duration);
        fn duration(days: u16, hours: u8, minutes: u8, seconds: u8) -> Duration {
            let seconds = seconds as u64;
            let minutes = 60 * minutes as u64;
            let hours = 60 * 60 * hours as u64;
            let days = 24 * 60 * 60 * days as u64;
            Duration::from_secs(days + hours + minutes + seconds)
        }
        assert_eq!(" 0 s", &f(duration(0, 0, 0, 0)));
        assert_eq!("59 s", &f(duration(0, 0, 0, 59)));
        assert_eq!(" 1 m 12 s", &f(duration(0, 0, 1, 12)));
        assert_eq!("59 m  0 s", &f(duration(0, 0, 59, 0)));
        assert_eq!(" 5 h  1 m  2 s", &f(duration(0, 5, 1, 2)));
        assert_eq!("  1 d  0 h  0 m  0 s", &f(duration(1, 0, 0, 0)));
        assert_eq!(" 69 d  0 h  0 m  0 s", &f(duration(69, 0, 0, 0)));
        assert_eq!("100 d 10 h 10 m 10 s", &f(duration(100, 10, 10, 10)));
    }

    #[test]
    fn test_time_as_ms_with_precision_1() {
        let f = |duration| time_starting_with_milliseconds(&duration, 1);
        let duration = duration_from_milli_micro;
        assert_eq!("123.5 ms", &f(duration(123, 456)));
        assert_eq!("123.5 ms", &f(duration(123, 455)));
        assert_eq!("123.4 ms", &f(duration(123, 445)));
        assert_eq!("123.4 ms", &f(duration(123, 440)));
        assert_eq!("123.1 ms", &f(duration(123, 100)));
        assert_eq!("123 ms", &f(duration(123, 0)));
    }

    #[test]
    fn test_time_as_ms_with_precision_2() {
        let f = |duration| time_starting_with_milliseconds(&duration, 2);
        let duration = duration_from_milli_micro;
        assert_eq!("123.46 ms", &f(duration(123, 456)));
        assert_eq!("123.46 ms", &f(duration(123, 455)));
        assert_eq!("123.45 ms", &f(duration(123, 454)));
        assert_eq!("123.45 ms", &f(duration(123, 450)));
        assert_eq!("123.1 ms", &f(duration(123, 100)));
        assert_eq!("123 ms", &f(duration(123, 0)));
    }

    #[test]
    fn test_time_as_ms_with_precision_3() {
        let f = |duration| time_starting_with_milliseconds(&duration, 3);
        let duration = duration_from_milli_micro;
        assert_eq!("123.456 ms", &f(duration(123, 456)));
        assert_eq!("123.455 ms", &f(duration(123, 455)));
        assert_eq!("123.454 ms", &f(duration(123, 454)));
        assert_eq!("123.45 ms", &f(duration(123, 450)));
        assert_eq!("123.1 ms", &f(duration(123, 100)));
        assert_eq!("123 ms", &f(duration(123, 0)));
    }
}
