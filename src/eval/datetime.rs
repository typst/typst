use std::fmt;
use std::fmt::{Debug, Formatter};

use ecow::{eco_format, eco_vec, EcoString, EcoVec};
use time::error::{Format, InvalidFormatDescription};
use time::format_description;

use typst_macros::cast_from_value;

#[derive(Clone, Copy, PartialEq, Hash)]
pub enum Datetime {
    Date(time::Date),
    Datetime(time::PrimitiveDateTime),
    Time(time::Time),
}

impl Datetime {
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

        Ok(formatted_result.map_err(format_time_format_error)?)
    }

    pub fn date(&self) -> Option<time::Date> {
        match self {
            Datetime::Date(date) => Some(*date),
            Datetime::Time(_) => None,
            Datetime::Datetime(datetime) => Some(datetime.date()),
        }
    }

    pub fn time(&self) -> Option<time::Time> {
        match self {
            Datetime::Date(_) => None,
            Datetime::Time(time) => Some(*time),
            Datetime::Datetime(datetime) => Some(datetime.time()),
        }
    }
}

impl Debug for Datetime {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let year = self.date().map_or("".to_string(), |d| format!("year: {}", d.year()));
        let month = self
            .date()
            .map_or("".to_string(), |d| format!("month: {}", d.month() as u8));
        let day = self.date().map_or("".to_string(), |d| format!("day: {}", d.day()));
        let hour = self.time().map_or("".to_string(), |d| format!("hour: {}", d.hour()));
        let minute = self
            .time()
            .map_or("".to_string(), |d| format!("minute: {}", d.minute()));
        let second = self
            .time()
            .map_or("".to_string(), |d| format!("second: {}", d.second()));
        write!(
            f,
            "datetime({})",
            eco_vec![year, month, day, hour, minute, second]
                .into_iter()
                .filter(|e| !e.is_empty())
                .collect::<EcoVec<String>>()
                .join(", ")
        )
    }
}

cast_from_value! {
    Datetime: "datetime",
}

fn format_time_format_error(error: Format) -> EcoString {
    match error {
        Format::InvalidComponent(name) => eco_format!("invalid component '{}'", name),
        _ => "failed to format datetime in the requested format".into(),
    }
}

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
