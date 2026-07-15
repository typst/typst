//! Date and time manipulation.
//!
//! In particular, this module provides the necessary building pieces for
//! [`World::today`](typst_library::World::today).

#![cfg(feature = "datetime")]

use std::sync::OnceLock;

use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveTime, Timelike, Utc};
use chrono::{NaiveDate, NaiveDateTime};

use typst_library::diag::{StrResult, bail};
use typst_library::foundations::{Datetime, Duration};

/// The current date and time.
pub struct Time(TimeInner);

/// The internal representation of a [`Time`].
enum TimeInner {
    /// A fixed date and time.
    Fixed(DateTime<Utc>),
    /// The current date and time if the time is not externally fixed.
    System(OnceLock<DateTime<Utc>>),
}

impl Time {
    /// Use a predefined fixed date and time to provide the current date. Used
    /// for reproducible builds.
    ///
    /// Returns an error if `datetime` is only a time.
    pub fn fixed(datetime: Datetime) -> StrResult<Self> {
        let date = match datetime {
            Datetime::Date(d) => d,
            Datetime::Datetime(dt) => dt.date(),
            Datetime::Time(_) => bail!("fixed datetime must specify a date"),
        };

        Ok(Time(TimeInner::Fixed(DateTime::from_naive_utc_and_offset(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(
                    date.year(),
                    date.month() as u32,
                    date.day() as u32,
                )
                .ok_or("provided fixed date is invalid")?,
                NaiveTime::from_hms_opt(
                    datetime.hour().unwrap_or(0) as u32,
                    datetime.minute().unwrap_or(0) as u32,
                    datetime.second().unwrap_or(0) as u32,
                )
                .ok_or("provided fixed time is invalid")?,
            ),
            Utc,
        ))))
    }

    /// Use a fixed timestamp to provide the current date. Used for reproducible
    /// builds.
    ///
    /// This timestamp is usually provided using the `SOURCE_DATE_EPOCH`
    /// environment variable.
    ///
    /// Returns an error if the timestamp is out of range.
    pub fn fixed_timestamp(timestamp: i64) -> StrResult<Self> {
        Ok(Time(TimeInner::Fixed(
            DateTime::from_timestamp(timestamp, 0).ok_or("timestamp is out of range")?,
        )))
    }

    /// Rely on the system to provide the current date.
    pub fn system() -> Self {
        Time(TimeInner::System(OnceLock::new()))
    }

    /// The time with the given UTC offset applied.
    fn with_offset(&self, offset: Option<Duration>) -> Option<DateTime<FixedOffset>> {
        let now = match &self.0 {
            TimeInner::Fixed(time) => time.fixed_offset(),
            TimeInner::System(time) => {
                let now_utc = time.get_or_init(Utc::now);
                if offset.is_some() {
                    // Actual offset will be applied later.
                    now_utc.fixed_offset()
                } else {
                    now_utc.with_timezone(&Local).fixed_offset()
                }
            }
        };

        match offset {
            None => Some(now),
            Some(offset) => {
                let seconds = offset.seconds().trunc();
                // Check whether we can convert seconds from f64 to i32
                if !seconds.is_finite()
                    || seconds < f64::from(i32::MIN)
                    || seconds > f64::from(i32::MAX)
                {
                    return None;
                }
                Some(now.with_timezone(&FixedOffset::east_opt(seconds as i32)?))
            }
        }
    }

    /// The current date.
    ///
    /// A timezone offset can be given to obtain the current date in this
    /// timezone.
    ///
    /// This can directly be used to implement
    /// [`World::today`](typst_library::World::today).
    pub fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let with_offset = self.with_offset(offset)?;
        Datetime::from_ymd(
            with_offset.year(),
            with_offset.month().try_into().ok()?,
            with_offset.day().try_into().ok()?,
        )
    }

    /// The current date and time, including the time of day.
    ///
    /// Unlike [`today`](Self::today), this only succeeds if the time is
    /// [fixed](Self::fixed) or [fixed to a timestamp](Self::fixed_timestamp),
    /// returning `None` for the live system clock. See [`Datetime::today`] for
    /// more details.
    ///
    /// This can directly be used to implement
    /// [`World::today_with_time`](typst_library::World::today_with_time).
    pub fn today_with_time(&self, offset: Option<Duration>) -> Option<Datetime> {
        if !matches!(self.0, TimeInner::Fixed(_)) {
            return None;
        }
        let with_offset = self.with_offset(offset)?;
        Datetime::from_ymd_hms(
            with_offset.year(),
            with_offset.month().try_into().ok()?,
            with_offset.day().try_into().ok()?,
            with_offset.hour().try_into().ok()?,
            with_offset.minute().try_into().ok()?,
            with_offset.second().try_into().ok()?,
        )
    }

    /// If not a fixed time, resets the memoized time fetched from the system.
    ///
    /// It will be fetched again the next time [`today`](Self::today) is called.
    /// This is usually called in between compilations.
    pub fn reset(&mut self) {
        if let TimeInner::System(ref mut time_lock) = self.0 {
            time_lock.take();
        }
    }
}

#[cfg(test)]
mod tests {
    use typst_library::foundations::Datetime;

    use super::Time;

    #[test]
    fn today_with_time_succeeds_when_fixed() {
        let datetime = Datetime::from_ymd_hms(2024, 4, 3, 10, 39, 30).unwrap();
        let time = Time::fixed(datetime).unwrap();
        assert_eq!(time.today_with_time(None), Some(datetime));
    }

    #[test]
    fn today_with_time_succeeds_when_fixed_timestamp() {
        let datetime = Datetime::from_ymd_hms(2024, 4, 3, 10, 39, 30).unwrap();
        let time = Time::fixed_timestamp(1712140770).unwrap();
        assert_eq!(time.today_with_time(None), Some(datetime));
    }

    #[test]
    fn today_with_time_fails_when_system() {
        let time = Time::system();
        assert_eq!(time.today_with_time(None), None);
    }
}
