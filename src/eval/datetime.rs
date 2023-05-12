use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;

use ecow::{eco_format, eco_vec, EcoString, EcoVec};
use time::error::{Format, InvalidFormatDescription};
use time::format_description;

use crate::util::pretty_array_like;
use typst_macros::cast_from_value;

/// A datetime object that represents either a date, a time or a combination of
/// both.
#[derive(Clone, Copy, PartialEq, Hash)]
pub enum Datetime {
    /// Representation as a date.
    Date(time::Date),
    /// Representation as a time.
    Datetime(time::PrimitiveDateTime),
    /// Representation as a combination of date and time.
    Time(time::Time),
}

impl Datetime {
    /// Display the date and/or time in a certain format.
    pub fn display(&self, pattern: Option<EcoString>) -> Result<EcoString, EcoString> {
        let pattern = pattern.unwrap_or(match self {
            Datetime::Date(_) => EcoString::from("[year]-[month]-[day]"),
            Datetime::Time(_) => EcoString::from("[hour]:[minute]:[second]"),
            Datetime::Datetime(_) => {
                EcoString::from("[year]-[month]-[day] [hour]:[minute]:[second]")
            }
        });

        let format = format_description::parse(pattern.as_str())
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
}

impl Debug for Datetime {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let year = self.year().map_or("".into(), |y| eco_format!("year:{}", y));
        let month = self.month().map_or("".into(), |m| eco_format!("month:{}", m));
        let day = self.day().map_or("".into(), |d| eco_format!("day:{}", d));
        let hour = self.hour().map_or("".into(), |h| eco_format!("hour:{}", h));
        let minute = self.minute().map_or("".into(), |m| eco_format!("minute:{}", m));
        let second = self.second().map_or("".into(), |s| eco_format!("second:{}", s));

        let filtered = eco_vec![year, month, day, hour, minute, second]
            .into_iter()
            .filter(|e| !e.is_empty())
            .collect::<EcoVec<EcoString>>();

        write!(f, "datetime{}", &pretty_array_like(&filtered, false))
    }
}

cast_from_value! {
    Datetime: "datetime",
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
