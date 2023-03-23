use super::TextElem;
use crate::prelude::*;
use chrono::{Local, Locale};

/// Create a text element with the current date and time, with a format.
///
/// Formats the current date and time with the specified format string. The crate `chrono` is used to get the current date and time, and formats the
/// value. See the [`chrono::format::strftime`](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) module on the supported escape sequences.
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
    #[default("%d.%m.%Y".to_string())]
    format: String,
}

impl Show for DateElem {
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let date = Local::now();
        let lang: Lang = TextElem::lang_in(styles);

        let locale = match lang {
            Lang::GERMAN => Locale::de_DE,
            Lang::ENGLISH | _ => Locale::en_US,
        };

        let format = &self.format(styles);

        let text = date.format_localized(format, locale);
        let content = TextElem::packed(text.to_string());
        Ok(content)
    }
}
