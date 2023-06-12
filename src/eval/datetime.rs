use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;

use ecow::{eco_format, EcoString, EcoVec};
use time::error::{Format, InvalidFormatDescription};
use time::{format_description, PrimitiveDateTime};

use crate::eval::cast;
use crate::util::pretty_array_like;

/// A datetime object that represents either a date, a time or a combination of
/// both.
#[derive(Clone, Copy, PartialEq, Hash)]
pub enum Datetime {
    /// Representation as a date.
    Date(time::Date),
    /// Representation as a time.
    Time(time::Time),
    /// Representation as a combination of date and time.
    Datetime(time::PrimitiveDateTime),
}

impl Datetime {
    /// Display the date and/or time in a certain format.
    pub fn display(&self, pattern: Option<EcoString>) -> Result<EcoString, EcoString> {
        let pattern = pattern.as_ref().map(EcoString::as_str).unwrap_or(match self {
            Datetime::Date(_) => "[year]-[month]-[day]",
            Datetime::Time(_) => "[hour]:[minute]:[second]",
            Datetime::Datetime(_) => "[year]-[month]-[day] [hour]:[minute]:[second]",
        });

        let format = format_description::parse(pattern)
            .map_err(format_time_invalid_format_description_error)?;

        let formatted_result = match self {
            Datetime::Date(date) => date.format(&format),
            Datetime::Time(time) => time.format(&format),
            Datetime::Datetime(datetime) => datetime.format(&format),
        }
        .map(EcoString::from);

        formatted_result.map_err(format_time_format_error)
    }

    /// Return the year of the datetime, if existing.
    pub fn year(&self) -> Option<i32> {
        match self {
            Datetime::Date(date) => Some(date.year()),
            Datetime::Time(_) => None,
            Datetime::Datetime(datetime) => Some(datetime.year()),
        }
    }

    /// Return the month of the datetime, if existing.
    pub fn month(&self) -> Option<u8> {
        match self {
            Datetime::Date(date) => Some(date.month().into()),
            Datetime::Time(_) => None,
            Datetime::Datetime(datetime) => Some(datetime.month().into()),
        }
    }

    /// Return the weekday of the datetime, if existing.
    pub fn weekday(&self) -> Option<u8> {
        match self {
            Datetime::Date(date) => Some(date.weekday().number_from_monday()),
            Datetime::Time(_) => None,
            Datetime::Datetime(datetime) => Some(datetime.weekday().number_from_monday()),
        }
    }

    /// Return the day of the datetime, if existing.
    pub fn day(&self) -> Option<u8> {
        match self {
            Datetime::Date(date) => Some(date.day()),
            Datetime::Time(_) => None,
            Datetime::Datetime(datetime) => Some(datetime.day()),
        }
    }

    /// Return the hour of the datetime, if existing.
    pub fn hour(&self) -> Option<u8> {
        match self {
            Datetime::Date(_) => None,
            Datetime::Time(time) => Some(time.hour()),
            Datetime::Datetime(datetime) => Some(datetime.hour()),
        }
    }

    /// Return the minute of the datetime, if existing.
    pub fn minute(&self) -> Option<u8> {
        match self {
            Datetime::Date(_) => None,
            Datetime::Time(time) => Some(time.minute()),
            Datetime::Datetime(datetime) => Some(datetime.minute()),
        }
    }

    /// Return the second of the datetime, if existing.
    pub fn second(&self) -> Option<u8> {
        match self {
            Datetime::Date(_) => None,
            Datetime::Time(time) => Some(time.second()),
            Datetime::Datetime(datetime) => Some(datetime.second()),
        }
    }

    /// Create a datetime from year, month, and day.
    pub fn from_ymd(year: i32, month: u8, day: u8) -> Option<Self> {
        Some(Datetime::Date(
            time::Date::from_calendar_date(year, time::Month::try_from(month).ok()?, day)
                .ok()?,
        ))
    }

    /// Create a datetime from hour, minute, and second.
    pub fn from_hms(hour: u8, minute: u8, second: u8) -> Option<Self> {
        Some(Datetime::Time(time::Time::from_hms(hour, minute, second).ok()?))
    }

    /// Create a datetime from day and time.
    pub fn from_ymd_hms(
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Option<Self> {
        let date =
            time::Date::from_calendar_date(year, time::Month::try_from(month).ok()?, day)
                .ok()?;
        let time = time::Time::from_hms(hour, minute, second).ok()?;
        Some(Datetime::Datetime(PrimitiveDateTime::new(date, time)))
    }
}

impl Debug for Datetime {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let year = self.year().map(|y| eco_format!("year: {y}"));
        let month = self.month().map(|m| eco_format!("month: {m}"));
        let day = self.day().map(|d| eco_format!("day: {d}"));
        let hour = self.hour().map(|h| eco_format!("hour: {h}"));
        let minute = self.minute().map(|m| eco_format!("minute: {m}"));
        let second = self.second().map(|s| eco_format!("second: {s}"));
        let filtered = [year, month, day, hour, minute, second]
            .into_iter()
            .flatten()
            .collect::<EcoVec<_>>();

        write!(f, "datetime{}", &pretty_array_like(&filtered, false))
    }
}

cast! {
    type Datetime: "datetime",
}

/// Format the `Format` error of the time crate in an appropriate way.
fn format_time_format_error(error: Format) -> EcoString {
    match error {
        Format::InvalidComponent(name) => eco_format!("invalid component '{}'", name),
        _ => "failed to format datetime in the requested format".into(),
    }
}

/// Format the `InvalidFormatDescription` error of the time crate in an
/// appropriate way.
fn format_time_invalid_format_description_error(
    error: InvalidFormatDescription,
) -> EcoString {
    match error {
        InvalidFormatDescription::UnclosedOpeningBracket { index, .. } => {
            eco_format!("missing closing bracket for bracket at index {}", index)
        }
        InvalidFormatDescription::InvalidComponentName { name, index, .. } => {
            eco_format!("invalid component name '{}' at index {}", name, index)
        }
        InvalidFormatDescription::InvalidModifier { value, index, .. } => {
            eco_format!("invalid modifier '{}' at index {}", value, index)
        }
        InvalidFormatDescription::Expected { what, index, .. } => {
            eco_format!("expected {} at index {}", what, index)
        }
        InvalidFormatDescription::MissingComponentName { index, .. } => {
            eco_format!("expected component name at index {}", index)
        }
        InvalidFormatDescription::MissingRequiredModifier { name, index, .. } => {
            eco_format!(
                "missing required modifier {} for component at index {}",
                name,
                index
            )
        }
        InvalidFormatDescription::NotSupported { context, what, index, .. } => {
            eco_format!("{} is not supported in {} at index {}", what, context, index)
        }
        _ => "failed to parse datetime format".into(),
    }
}
