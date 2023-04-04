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
        let result = self.0.format(&format).map_err(|_| "couldn't parse date format")?;
        Ok(result.into())
    }

    pub fn add(&self, duration: &Duration) -> Result<Self, &'static str> {
        Ok(Self(self.0.checked_add(duration.0).ok_or(Date::err_msg())?))
    }

    pub fn sub(&self, duration: &Duration) -> Result<Self, &'static str> {
        Ok(Self(self.0.checked_sub(duration.0).ok_or(Date::err_msg())?))
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

    fn err_msg() -> &'static str {
        return "resulting date is too large";
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
        duration = duration
            .checked_add(time::Duration::weeks(weeks))
            .ok_or(Duration::err_msg())?;
        duration = duration
            .checked_add(time::Duration::days(days))
            .ok_or(Duration::err_msg())?;
        Ok(Self(duration))
    }

    pub fn add(&self, duration: &Duration) -> Result<Self, &'static str> {
        self.0
            .checked_add(duration.0)
            .map_or(Err(Duration::err_msg()), |v| Ok(Duration(v)))
    }

    pub fn sub(&self, duration: &Duration) -> Result<Self, &'static str> {
        self.0
            .checked_sub(duration.0)
            .map_or(Err(Duration::err_msg()), |v| Ok(Duration(v)))
    }

    pub fn weeks(&self) -> i64 {
        self.0.whole_weeks()
    }

    pub fn days(&self) -> i64 {
        self.0.whole_days()
    }

    fn err_msg() -> &'static str {
        return "resulting duration is too large";
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
