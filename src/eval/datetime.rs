use ecow::{eco_vec, EcoVec};
use std::fmt;
use std::fmt::{Debug, Formatter};
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
    pub fn display(&self, pattern: Option<String>) -> Result<String, String> {
        let pattern = pattern.unwrap_or(match self {
            Datetime::Date(_) => String::from("[year]-[month]-[day]"),
            Datetime::Time(_) => String::from("[hour]:[minute]:[second]"),
            Datetime::Datetime(_) => {
                String::from("[year]-[month]-[day] [hour]:[minute]:[second]")
            }
        });

        let format =
            format_description::parse(pattern.as_str()).map_err(|e| match e {
                InvalidFormatDescription::UnclosedOpeningBracket { .. } => {
                    "found unclosed bracket".to_string()
                }
                InvalidFormatDescription::InvalidComponentName { name, .. } => {
                    format!("{} is not a valid component", name)
                }
                InvalidFormatDescription::InvalidModifier { value, .. } => {
                    format!("modifier {} is invalid", value)
                }
                InvalidFormatDescription::Expected { what, .. } => {
                    format!("expected {}", what)
                }
                InvalidFormatDescription::MissingComponentName { .. } => {
                    "a component name is missing".to_string()
                }
                InvalidFormatDescription::MissingRequiredModifier { name, .. } => {
                    format!("missing required modifier {}", name)
                }
                InvalidFormatDescription::NotSupported { context, what, .. } => {
                    format!("{} is not supported in {}", what, context)
                }
                _ => "unable to parse datetime format".to_string(),
            })?;

        let formatted_result = match self {
            Datetime::Date(date) => date.format(&format),
            Datetime::Time(time) => time.format(&format),
            Datetime::Datetime(datetime) => datetime.format(&format),
        };

        let unwrapped_result = formatted_result.map_err(|e| match e {
            Format::InvalidComponent(name) => format!("found invalid component {}", name),
            _ => "unable to format datetime in the requested format".to_string(),
        })?;

        Ok(unwrapped_result)
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
