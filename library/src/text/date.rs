use std::str::FromStr;

use super::TextElem;
use crate::prelude::*;
use time::{Month, OffsetDateTime, Weekday};
use time_fmt::format::*;
use typst::diag::SourceError;

/// Wrapper for the formate string so the string gets checked for possible
/// errors on initialization.
pub struct DateFormat(String);

impl FromStr for DateFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut was_percent = false;
        for c in s.chars() {
            if was_percent {
                // all possible specifier
                if ![
                    'a', 'A', 'b', 'B', 'c', 'C', 'd', 'D', 'e', 'F', 'g', 'G', 'h', 'H',
                    'I', 'j', 'k', 'l', 'm', 'M', 'n', 'p', 'P', 'r', 'R', 'S', 't', 'T',
                    'u', 'U', 'V', 'w', 'W', 'x', 'X', 'y', 'Y', 'z', 'Z', '%',
                ]
                .contains(&c)
                {
                    return Err(format!("Unknown specifier '%{}'", c));
                }
                was_percent = false;
            } else {
                was_percent = c == '%'
            }
        }

        Ok(Self(s.to_string()))
    }
}

impl Default for DateFormat {
    fn default() -> Self {
        Self("%d.%m.%Y".to_string())
    }
}

cast_from_value! {
    DateFormat,
    v: EcoString => DateFormat::from_str(&v)?,
}

cast_to_value! {
    v: DateFormat => v.0.into()
}

/// Create a text element with the current date and time, with a format.
///
/// Formats the current date and time with the specified format string. The
/// crate [`time-fmt`](https://github.com/MiSawa/time-fmt) is used to get the
/// current date and time, and formats the value.
///
/// ## Example
/// ```example
/// = Date
/// #date(format: "%d.%m.%Y")
/// ```
///
/// Display: Date
/// Category: text
#[element(Show)]
pub struct DateElem {
    /// How the date should be formatted.
    #[default]
    format: DateFormat,
}

impl Show for DateElem {
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let date_time = OffsetDateTime::now_utc();
        let lang: Lang = TextElem::lang_in(styles);

        let format: String = self.format(styles).0;

        //HACK Since `time-fmt` does not support languages, the escape characters in question are replaced here.
        let text = format
            .replace("%b", month_short_str(lang, date_time.month()))
            .replace("%B", month_long_str(lang, date_time.month()))
            .replace("%a", weekday_short_str(lang, date_time.weekday()))
            .replace("%A", weekday_long_str(lang, date_time.weekday()));

        let text = match format_offset_date_time(&text, date_time) {
            Ok(ok) => ok,
            Err(err) => {
                return SourceResult::Err(Box::new(vec![SourceError::new(
                    self.span(),
                    EcoString::from(err.to_string()),
                )]))
            }
        };

        let content = TextElem::packed(text.to_string());
        Ok(content)
    }
}

fn month_short_str(lang: Lang, month: Month) -> &'static str {
    match lang {
        Lang::GERMAN => match month {
            Month::January => "Jan",
            Month::February => "Feb",
            Month::March => "Mär",
            Month::April => "Apr",
            Month::May => "Mai",
            Month::June => "Jun",
            Month::July => "Jul",
            Month::August => "Aug",
            Month::September => "Sep",
            Month::October => "Okt",
            Month::November => "Nov",
            Month::December => "Dez",
        },
        Lang::ENGLISH | _ => match month {
            Month::January => "Jan",
            Month::February => "Feb",
            Month::March => "Mar",
            Month::April => "Apr",
            Month::May => "May",
            Month::June => "Jun",
            Month::July => "Jul",
            Month::August => "Aug",
            Month::September => "Sep",
            Month::October => "Oct",
            Month::November => "Nov",
            Month::December => "Dec",
        },
    }
}

fn month_long_str(lang: Lang, month: Month) -> &'static str {
    match lang {
        Lang::GERMAN => match month {
            Month::January => "Januar",
            Month::February => "Februar",
            Month::March => "März",
            Month::April => "April",
            Month::May => "Mai",
            Month::June => "Juni",
            Month::July => "Juli",
            Month::August => "August",
            Month::September => "September",
            Month::October => "Oktober",
            Month::November => "November",
            Month::December => "Dezember",
        },
        Lang::ENGLISH | _ => match month {
            Month::January => "January",
            Month::February => "February",
            Month::March => "March",
            Month::April => "April",
            Month::May => "May",
            Month::June => "June",
            Month::July => "July",
            Month::August => "August",
            Month::September => "September",
            Month::October => "October",
            Month::November => "November",
            Month::December => "December",
        },
    }
}

fn weekday_short_str(lang: Lang, day: Weekday) -> &'static str {
    match lang {
        Lang::GERMAN => match day {
            Weekday::Monday => "Mo",
            Weekday::Tuesday => "Di",
            Weekday::Wednesday => "Mi",
            Weekday::Thursday => "Do",
            Weekday::Friday => "Fr",
            Weekday::Saturday => "Sa",
            Weekday::Sunday => "So",
        },
        Lang::ENGLISH | _ => match day {
            Weekday::Monday => "Mon",
            Weekday::Tuesday => "Tue",
            Weekday::Wednesday => "Wed",
            Weekday::Thursday => "Thu",
            Weekday::Friday => "Fri",
            Weekday::Saturday => "Sat",
            Weekday::Sunday => "Sun",
        },
    }
}

fn weekday_long_str(lang: Lang, day: Weekday) -> &'static str {
    match lang {
        Lang::GERMAN => match day {
            Weekday::Monday => "Montag",
            Weekday::Tuesday => "Dienstag",
            Weekday::Wednesday => "Mittwoch",
            Weekday::Thursday => "Donnerstag",
            Weekday::Friday => "Freitag",
            Weekday::Saturday => "Samstag",
            Weekday::Sunday => "Sonntag",
        },
        Lang::ENGLISH | _ => match day {
            Weekday::Monday => "Monday",
            Weekday::Tuesday => "Tuesday",
            Weekday::Wednesday => "Wednesday",
            Weekday::Thursday => "Thursday",
            Weekday::Friday => "Friday",
            Weekday::Saturday => "Saturday",
            Weekday::Sunday => "Sunday",
        },
    }
}
