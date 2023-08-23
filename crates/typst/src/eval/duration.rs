use crate::util::pretty_array_like;
use ecow::{eco_format, EcoVec};
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::{Add, Div, Mul, Neg, Sub};
use time::ext::NumericalDuration;
use typst_macros::cast;

/// A duration object that represents either a positive or negative span of time.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Duration(time::Duration);
impl Duration {
    pub fn seconds(&self) -> f64 {
        self.0.as_seconds_f64()
    }

    pub fn minutes(&self) -> f64 {
        self.0.as_seconds_f64() / 60.0
    }

    pub fn hours(&self) -> f64 {
        self.0.as_seconds_f64() / 3_600.0
    }

    pub fn days(&self) -> f64 {
        self.0.as_seconds_f64() / 86_400.0
    }

    pub fn weeks(&self) -> f64 {
        self.0.as_seconds_f64() / 604_800.0
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl From<time::Duration> for Duration {
    fn from(value: time::Duration) -> Self {
        Self(value)
    }
}

impl Debug for Duration {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut tmp = self.0;
        let mut vec = EcoVec::new();

        let weeks = tmp.whole_seconds() / 604_800.0 as i64;
        if weeks != 0 {
            vec.push(eco_format!("weeks: {weeks}"));
        }
        tmp -= weeks.weeks();

        let days = tmp.whole_days();
        if days != 0 {
            vec.push(eco_format!("days: {days}"));
        }
        tmp -= days.days();

        let hours = tmp.whole_hours();
        if hours != 0 {
            vec.push(eco_format!("hours: {hours}"));
        }
        tmp -= hours.hours();

        let minutes = tmp.whole_minutes();
        if minutes != 0 {
            vec.push(eco_format!("minutes: {minutes}"));
        }
        tmp -= minutes.minutes();

        let seconds = tmp.whole_seconds();
        if seconds != 0 {
            vec.push(eco_format!("seconds: {seconds}"));
        }

        write!(f, "duration{}", &pretty_array_like(&vec, false))
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

cast! {
    type Duration: "duration",
}
