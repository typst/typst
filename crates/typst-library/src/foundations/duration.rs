use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, Div, Mul, Neg, Sub};

use ecow::{eco_format, EcoString};
use time::ext::NumericalDuration;

use crate::foundations::{func, repr, scope, ty, Repr};

/// Represents a positive or negative span of time.
#[ty(scope, cast)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Duration(time::Duration);

impl Duration {
    /// Whether the duration is empty / zero.
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Decomposes the time into whole weeks, days, hours, minutes, and seconds.
    pub fn decompose(&self) -> [i64; 5] {
        let mut tmp = self.0;
        let weeks = tmp.whole_weeks();
        tmp -= weeks.weeks();
        let days = tmp.whole_days();
        tmp -= days.days();
        let hours = tmp.whole_hours();
        tmp -= hours.hours();
        let minutes = tmp.whole_minutes();
        tmp -= minutes.minutes();
        let seconds = tmp.whole_seconds();
        [weeks, days, hours, minutes, seconds]
    }
}

#[scope]
impl Duration {
    /// Creates a new duration.
    ///
    /// You can specify the [duration] using weeks, days, hours, minutes and
    /// seconds. You can also get a duration by subtracting two
    /// [datetimes]($datetime).
    ///
    /// ```example
    /// #duration(
    ///   days: 3,
    ///   hours: 12,
    /// ).hours()
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// The number of seconds.
        #[named]
        #[default(0)]
        seconds: i64,
        /// The number of minutes.
        #[named]
        #[default(0)]
        minutes: i64,
        /// The number of hours.
        #[named]
        #[default(0)]
        hours: i64,
        /// The number of days.
        #[named]
        #[default(0)]
        days: i64,
        /// The number of weeks.
        #[named]
        #[default(0)]
        weeks: i64,
    ) -> Duration {
        Duration::from(
            time::Duration::seconds(seconds)
                + time::Duration::minutes(minutes)
                + time::Duration::hours(hours)
                + time::Duration::days(days)
                + time::Duration::weeks(weeks),
        )
    }

    /// The duration expressed in seconds.
    ///
    /// This function returns the total duration represented in seconds as a
    /// floating-point number rather than the second component of the duration.
    #[func]
    pub fn seconds(&self) -> f64 {
        self.0.as_seconds_f64()
    }

    /// The duration expressed in minutes.
    ///
    /// This function returns the total duration represented in minutes as a
    /// floating-point number rather than the second component of the duration.
    #[func]
    pub fn minutes(&self) -> f64 {
        self.seconds() / 60.0
    }

    /// The duration expressed in hours.
    ///
    /// This function returns the total duration represented in hours as a
    /// floating-point number rather than the second component of the duration.
    #[func]
    pub fn hours(&self) -> f64 {
        self.seconds() / 3_600.0
    }

    /// The duration expressed in days.
    ///
    /// This function returns the total duration represented in days as a
    /// floating-point number rather than the second component of the duration.
    #[func]
    pub fn days(&self) -> f64 {
        self.seconds() / 86_400.0
    }

    /// The duration expressed in weeks.
    ///
    /// This function returns the total duration represented in weeks as a
    /// floating-point number rather than the second component of the duration.
    #[func]
    pub fn weeks(&self) -> f64 {
        self.seconds() / 604_800.0
    }
}

impl Debug for Duration {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Repr for Duration {
    fn repr(&self) -> EcoString {
        let [weeks, days, hours, minutes, seconds] = self.decompose();
        let mut vec = Vec::with_capacity(5);

        if weeks != 0 {
            vec.push(eco_format!("weeks: {}", weeks.repr()));
        }

        if days != 0 {
            vec.push(eco_format!("days: {}", days.repr()));
        }

        if hours != 0 {
            vec.push(eco_format!("hours: {}", hours.repr()));
        }

        if minutes != 0 {
            vec.push(eco_format!("minutes: {}", minutes.repr()));
        }

        if seconds != 0 {
            vec.push(eco_format!("seconds: {}", seconds.repr()));
        }

        eco_format!("duration{}", &repr::pretty_array_like(&vec, false))
    }
}

impl From<time::Duration> for Duration {
    fn from(value: time::Duration) -> Self {
        Self(value)
    }
}

impl From<Duration> for time::Duration {
    fn from(value: Duration) -> Self {
        value.0
    }
}

impl Add for Duration {
    type Output = Duration;

    fn add(self, rhs: Self) -> Self::Output {
        Duration(self.0 + rhs.0)
    }
}

impl Sub for Duration {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        Duration(self.0 - rhs.0)
    }
}

impl Neg for Duration {
    type Output = Duration;

    fn neg(self) -> Self::Output {
        Duration(-self.0)
    }
}

impl Mul<f64> for Duration {
    type Output = Duration;

    fn mul(self, rhs: f64) -> Self::Output {
        Duration(self.0 * rhs)
    }
}

impl Div<f64> for Duration {
    type Output = Duration;

    fn div(self, rhs: f64) -> Self::Output {
        Duration(self.0 / rhs)
    }
}

impl Div for Duration {
    type Output = f64;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}
