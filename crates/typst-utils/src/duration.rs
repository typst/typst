use std::fmt::{self, Display, Formatter, Write};
use std::time::Duration;

use super::round_with_precision;

/// Formats a duration with a precision suitable for human display.
pub fn format_duration(duration: Duration) -> impl Display {
    DurationDisplay(duration)
}

/// Displays a `Duration`.
struct DurationDisplay(Duration);

impl Display for DurationDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut space = false;
        macro_rules! piece {
            ($($tts:tt)*) => {
                if std::mem::replace(&mut space, true) {
                    f.write_char(' ')?;
                }
                write!(f, $($tts)*)?;
            };
        }

        let secs = self.0.as_secs();
        let (mins, secs) = (secs / 60, (secs % 60));
        let (hours, mins) = (mins / 60, (mins % 60));
        let (days, hours) = ((hours / 24), (hours % 24));

        if days > 0 {
            piece!("{days} d");
        }

        if hours > 0 {
            piece!("{hours} h");
        }

        if mins > 0 {
            piece!("{mins} min");
        }

        // No need to display anything more than minutes at this point.
        if days > 0 || hours > 0 {
            return Ok(());
        }

        let order = |exp| 1000u64.pow(exp);
        let nanos = secs * order(3) + self.0.subsec_nanos() as u64;
        let fract = |exp| round_with_precision(nanos as f64 / order(exp) as f64, 2);

        if nanos == 0 || self.0 > Duration::from_secs(1) {
            // For durations > 5 min, we drop the fractional part.
            if self.0 > Duration::from_secs(300) {
                piece!("{secs} s");
            } else {
                piece!("{} s", fract(3));
            }
        } else if self.0 > Duration::from_millis(1) {
            piece!("{} ms", fract(2));
        } else if self.0 > Duration::from_micros(1) {
            piece!("{} µs", fract(1));
        } else {
            piece!("{} ns", fract(0));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn test(duration: Duration, expected: &str) {
        assert_eq!(format_duration(duration).to_string(), expected);
    }

    #[test]
    fn test_format_duration() {
        test(Duration::from_secs(1000000), "11 d 13 h 46 min");
        test(Duration::from_secs(3600 * 24), "1 d");
        test(Duration::from_secs(3600), "1 h");
        test(Duration::from_secs(3600 + 240), "1 h 4 min");
        test(Duration::from_secs_f64(364.77), "6 min 4 s");
        test(Duration::from_secs_f64(264.776), "4 min 24.78 s");
        test(Duration::from_secs(3), "3 s");
        test(Duration::from_secs_f64(2.8492), "2.85 s");
        test(Duration::from_micros(734), "734 µs");
        test(Duration::from_micros(294816), "294.82 ms");
        test(Duration::from_nanos(1), "1 ns");
    }
}
