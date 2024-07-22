use std::{cmp::min, time::Duration};

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
fn time_as_seconds(duration: &Duration) -> String {
    let (days, hours, minutes, seconds, _, _) = get_duration_parts(duration);
    format_dhms(days, hours, minutes, seconds)
}

/// Format string starting with number of milliseconds and going bigger from there.
/// `precision` is how many digits of microseconds from left will be preserved.
/// Note that this function will always remove all trailing zeros for microseconds.
fn time_as_ms_with_precision(duration: &Duration, precision: usize) -> String {
    let digits = precision;
    let (d, h, m, s, ms, mcs) = get_duration_parts(duration);
    let mcs_digits = {
        let mut mcs = mcs;
        println!("before {mcs}");
        while mcs % 10 == 0 && mcs != 0 {
            mcs /= 10
        }
        println!("after {mcs}");
        mcs.to_string().len()
    };
    dbg!(digits, mcs, mcs_digits);
    let ms = format!(
        "{1:.0$} ms",
        if mcs != 0 { min(digits, mcs_digits) } else { 0 },
        ms as f32 + mcs as f32 / 1000.0
    );
    match (d, h, m, s) {
        (0, 0, 0, 0) => ms.to_string(),
        (d, h, m, s) => format_dhms(d, h, m, s),
    }
}

fn time_as_ms(duration: &Duration) -> String {
    time_as_ms_with_precision(duration, 2)
}

pub fn elapsed_time(duration: &Duration) -> String {
    time_as_seconds(duration)
}

pub fn eta_time(duration: &Duration) -> String {
    time_as_seconds(duration)
}

pub fn compilation_time(duration: &Duration) -> String {
    time_as_ms(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_as_seconds() {
        fn assert(left: &str, right: Duration) {
            assert_eq!(left, time_as_seconds(&right))
        }
        assert(" 0 s", Duration::from_secs(0));
        assert("59 s", Duration::from_secs(59));
        assert(" 1 m 12 s", Duration::from_secs(60 + 12));
        assert("59 m  0 s", Duration::from_secs(60 * 59));
        assert(" 5 h  1 m  2 s", Duration::from_secs(60 * 60 * 5 + 60 + 2));
    }

    #[test]
    fn test_time_as_ms_with_precision_1() {
        fn assert(left: &str, right: Duration) {
            assert_eq!(left, time_as_ms_with_precision(&right, 1))
        }
        assert("123.5 ms", Duration::from_micros(123456));
        assert("123.5 ms", Duration::from_micros(123455));
        assert("123.4 ms", Duration::from_micros(123445));
        assert("123.4 ms", Duration::from_micros(123440));
        assert("123.1 ms", Duration::from_micros(123100));
        assert("123 ms", Duration::from_micros(123000));
    }

    #[test]
    fn test_time_as_ms_with_precision_2() {
        fn assert(left: &str, right: Duration) {
            assert_eq!(left, time_as_ms_with_precision(&right, 2))
        }
        assert("123.46 ms", Duration::from_micros(123456));
        assert("123.46 ms", Duration::from_micros(123455));
        assert("123.45 ms", Duration::from_micros(123454));
        assert("123.45 ms", Duration::from_micros(123450));
        assert("123.1 ms", Duration::from_micros(123100));
        assert("123 ms", Duration::from_micros(123000));
        // assert(
        //     format!("{:?}", Duration::from_micros(123000)).as_str(),
        //     Duration::from_micros(123000),
        // );
    }

    #[test]
    fn test_time_as_ms_with_precision_3() {
        fn assert(left: &str, right: Duration) {
            assert_eq!(left, time_as_ms_with_precision(&right, 3))
        }
        assert("123.456 ms", Duration::from_micros(123456));
        assert("123.455 ms", Duration::from_micros(123455));
        assert("123.454 ms", Duration::from_micros(123454));
        assert("123.45 ms", Duration::from_micros(123450));
        assert("123.1 ms", Duration::from_micros(123100));
        assert("123 ms", Duration::from_micros(123000));
    }
}
