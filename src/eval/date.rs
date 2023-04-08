use std::fmt;
use std::fmt::{Debug, Formatter};
use time::format_description;
use typst::eval::Str;
use typst_macros::cast_from_value;

#[derive(Clone, Copy, PartialEq, Hash)]
pub struct Date(pub time::Date);

impl Date {
    pub fn display(&self, pattern: Option<Str>) -> Result<Str, &str> {
        let pattern = pattern.unwrap_or(Str::from("[year]-[month]-[day]"));
        let format = format_description::parse(pattern.as_str())
            .map_err(|_| "invalid date format")?;
        let result = self.0.format(&format).map_err(|_| "couldn't parse date")?;
        Ok(result.into())
    }

    pub fn add(&self, duration: &Duration) -> Self {
        Self(self.0.saturating_add(duration.0))
    }

    pub fn sub(&self, duration: &Duration) -> Self {
        Self(self.0.saturating_sub(duration.0))
    }

    pub fn year(&self) -> i32 {
        self.0.year()
    }

    pub fn month(&self) -> u8 {
        self.0.month() as u8
    }

    pub fn day(&self) -> u8 {
        self.0.day()
    }
}

impl Debug for Date {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "date({})", self.0)
    }
}

cast_from_value! {
    Date: "date",
}

#[derive(Clone, Copy, PartialEq, Hash)]
pub struct Duration(pub time::Duration);

impl Duration {
    pub fn new(weeks: i64, days: i64) -> Result<Self, &'static str> {
        let mut duration = time::Duration::days(0);
        duration = duration.saturating_add(time::Duration::weeks(weeks));
        duration = duration.saturating_add(time::Duration::days(days));
        Ok(Self(duration))
    }

    pub fn add(&self, duration: &Duration) -> Self {
        Self(self.0.saturating_add(duration.0))
    }

    pub fn sub(&self, duration: &Duration) -> Self {
        Self(self.0.saturating_sub(duration.0))
    }

    pub fn weeks(&self) -> i64 {
        self.0.whole_weeks()
    }

    pub fn days(&self) -> i64 {
        self.0.whole_days()
    }
}

impl Debug for Duration {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "duration({} days)", self.0.whole_days())
    }
}

cast_from_value! {
    Duration: "duration",
}
