use std::fmt;
use std::fmt::{Debug, Formatter};
use time::error::{Format, InvalidFormatDescription};
use time::format_description;
use typst::eval::Str;
use typst_macros::cast_from_value;

#[derive(Clone, Copy, PartialEq, Hash)]
pub struct Date(pub time::Date);

impl Date {
    pub fn display(&self, pattern: Option<Str>) -> Result<Str, String> {
        let pattern = pattern.unwrap_or(Str::from("[year]-[month]-[day]"));
        let format =
            format_description::parse(pattern.as_str()).map_err(|e| match e {
                InvalidFormatDescription::UnclosedOpeningBracket { .. } => {
                    "found unclosed bracket".to_string()
                }
                InvalidFormatDescription::InvalidComponentName { name, .. } => {
                    format!("{} is not a valid component.", name)
                }
                InvalidFormatDescription::InvalidModifier { value, .. } => {
                    format!("modifier {} is invalid.", value)
                }
                InvalidFormatDescription::Expected { what, .. } => {
                    format!("expected {}", what)
                }
                InvalidFormatDescription::MissingComponentName { .. } => {
                    format!("found missing component name",)
                }
                InvalidFormatDescription::MissingRequiredModifier { name, .. } => {
                    format!("missing required modifier {}", name)
                }
                InvalidFormatDescription::NotSupported { context, what, .. } => {
                    format!("{} is not supported in {}", what, context)
                }
                _ => "invalid date format".to_string(),
            })?;
        let result = self.0.format(&format).map_err(|e| match e {
            Format::InvalidComponent(name) => format!("found invalid component {}", name),
            _ => "couldn't parse the date".to_string(),
        })?;
        Ok(result.into())
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
