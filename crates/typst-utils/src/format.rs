//! Different formatting functions.

use std::time::Duration;

/// Returns value with `n` digits after floating point where `n` is `precision`.
/// Standard rounding rule applies (if `n+1`th digit >= 5 then round up).
/// If `value` is +/- infinity or NaN returns `value`.
pub fn round_with_precision(value: f64, precision: u8) -> f64 {
    if value.is_infinite()
        || value.is_nan()
        // Integers that big can't have fractional part (mantissa is already full).
        || value.abs() >= (1_i64 << 53) as f64
        // Binary64 format can only precisely represent up to log_10(2^53) digits.
        || precision >= 17
    {
        return value;
    }
    let offset = 10_f64.powi(precision.into());
    if !(value * offset).is_finite() {
        return value;
    }
    (value * offset).round() / offset
}

/// Returns number of `(days, hours, minutes, seconds, milliseconds, microseconds)`.
fn get_duration_parts(duration: &Duration) -> (u16, u8, u8, u8, u16, u16) {
    // In practice we probably don't need nanoseconds.
    let micros = duration.as_micros();
    let (millis, micros) = (micros / 1000, (micros % 1000) as u16);
    let (sec, millis) = (millis / 1000, (millis % 1000) as u16);
    let (mins, sec) = (sec / 60, (sec % 60) as u8);
    let (hours, mins) = (mins / 60, (mins % 60) as u8);
    let (days, hours) = ((hours / 24) as u16, (hours % 24) as u8);
    (days, hours, mins, sec, millis, micros)
}

/// Format string using `days`, `hours`, `minutes`, `seconds`.
fn format_dhms(days: u16, hours: u8, minutes: u8, seconds: u8) -> String {
    match (days, hours, minutes, seconds) {
        (0, 0, 0, s) => format!("{s:2} s"),
        (0, 0, m, s) => format!("{m:2} m {s:2} s"),
        (0, h, m, s) => format!("{h:2} h {m:2} m {s:2} s"),
        (d, h, m, s) => format!("{d:3} d {h:2} h {m:2} m {s:2} s"),
    }
}

/// Format string starting with number of days and going bigger from there.
pub fn time_starting_with_seconds(duration: &Duration) -> String {
    let (days, hours, minutes, seconds, _, _) = get_duration_parts(duration);
    format_dhms(days, hours, minutes, seconds)
}

/// Format string starting with number of milliseconds and going bigger from there.
/// `precision` is how many digits of microseconds from floating point to the right
/// will be preserved. Note that this function will always remove all trailing zeros
/// for microseconds.
pub fn time_starting_with_ms_with_precision(
    duration: &Duration,
    precision: u8,
) -> String {
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
        let f = |duration| time_starting_with_ms_with_precision(&duration, 1);
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
        let f = |duration| time_starting_with_ms_with_precision(&duration, 2);
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
        let f = |duration| time_starting_with_ms_with_precision(&duration, 3);
        let duration = duration_from_milli_micro;
        assert_eq!("123.456 ms", &f(duration(123, 456)));
        assert_eq!("123.455 ms", &f(duration(123, 455)));
        assert_eq!("123.454 ms", &f(duration(123, 454)));
        assert_eq!("123.45 ms", &f(duration(123, 450)));
        assert_eq!("123.1 ms", &f(duration(123, 100)));
        assert_eq!("123 ms", &f(duration(123, 0)));
    }
}
