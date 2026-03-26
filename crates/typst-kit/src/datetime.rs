//! Date and time manipulation.
//!
//! In particular, this module provides the necessary building pieces for
//! [`typst::World::today`].

#![cfg(feature = "datetime")]

use std::sync::OnceLock;

use chrono::{DateTime, Datelike, FixedOffset, Local, Utc};
use typst_library::foundations::{Datetime, Duration};

/// The current date and time.
pub enum Now {
    /// The date and time if the environment `SOURCE_DATE_EPOCH` is set.
    /// Used for reproducible builds.
    Fixed(DateTime<Utc>),
    /// The current date and time if the time is not externally fixed.
    System(OnceLock<DateTime<Utc>>),
}

impl Now {
    /// The current date.
    ///
    /// A timezone offset can be given to obtain the current date in this
    /// timezone.
    pub fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let now = match self {
            Now::Fixed(time) => time.fixed_offset(),
            Now::System(time) => {
                let now_utc = time.get_or_init(Utc::now);
                if offset.is_some() {
                    // Actual offset will be applied later.
                    now_utc.fixed_offset()
                } else {
                    now_utc.with_timezone(&Local).fixed_offset()
                }
            }
        };

        // The time with the specified UTC offset.
        let with_offset = match offset {
            None => now,
            Some(offset) => {
                let seconds = offset.seconds().trunc();
                // Check whether we can convert seconds from f64 to i32
                if !seconds.is_finite()
                    || seconds < f64::from(i32::MIN)
                    || seconds > f64::from(i32::MAX)
                {
                    return None;
                }
                now.with_timezone(&FixedOffset::east_opt(seconds as i32)?)
            }
        };

        Datetime::from_ymd(
            with_offset.year(),
            with_offset.month().try_into().ok()?,
            with_offset.day().try_into().ok()?,
        )
    }

    /// Fetch the current time again from the system, if it was not fixed.
    pub fn reset(&mut self) {
        if let Now::System(time_lock) = self {
            time_lock.take();
        }
    }
}
