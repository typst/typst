use serde::Deserialize;

use super::{FontStretch, FontStyle, FontWeight};

#[derive(Debug, Default, Deserialize)]
pub struct Exception {
    pub family: Option<&'static str>,
    pub style: Option<FontStyle>,
    pub weight: Option<FontWeight>,
    pub stretch: Option<FontStretch>,
}

impl Exception {
    pub const fn new() -> Self {
        Self {
            family: None,
            style: None,
            weight: None,
            stretch: None,
        }
    }

    const fn family(self, family: &'static str) -> Self {
        Self { family: Some(family), ..self }
    }

    const fn style(self, style: FontStyle) -> Self {
        Self { style: Some(style), ..self }
    }

    const fn weight(self, weight: u16) -> Self {
        Self { weight: Some(FontWeight(weight)), ..self }
    }

    #[allow(unused)] // left for future use
    const fn stretch(self, stretch: u16) -> Self {
        Self { stretch: Some(FontStretch(stretch)), ..self }
    }
}

pub fn find_exception(postscript_name: &str) -> Option<&'static Exception> {
    EXCEPTION_MAP.get(postscript_name)
}

/// A map which keys are PostScript name and values are override entries.
static EXCEPTION_MAP: phf::Map<&'static str, Exception> = phf::phf_map! {
    "NewCM08-Book" => Exception::new()
        .family("New Computer Modern 08")
        .weight(450),
    "NewCM08-BookItalic" => Exception::new()
        .family("New Computer Modern 08")
        .weight(450),
    "NewCM08-Italic" => Exception::new()
        .family("New Computer Modern 08"),
    "NewCM08-Regular" => Exception::new()
        .family("New Computer Modern 08"),
    "NewCM10-Bold" => Exception::new()
        .family("New Computer Modern"),
    "NewCM10-BoldItalic" => Exception::new()
        .family("New Computer Modern"),
    "NewCM10-Book" => Exception::new()
        .family("New Computer Modern")
        .weight(450),
    "NewCM10-BookItalic" => Exception::new()
        .family("New Computer Modern")
        .weight(450),
    "NewCM10-Italic" => Exception::new()
        .family("New Computer Modern"),
    "NewCM10-Regular" => Exception::new()
        .family("New Computer Modern"),
    "NewCMMath-Book" => Exception::new()
        .family("New Computer Modern Math")
        .weight(450),
    "NewCMMath-Regular" => Exception::new()
        .family("New Computer Modern Math"),
    "NewCMMono10-Bold" => Exception::new()
        .family("New Computer Modern Mono"),
    "NewCMMono10-BoldOblique" => Exception::new()
        .family("New Computer Modern Mono"),
    "NewCMMono10-Book" => Exception::new()
        .family("New Computer Modern Mono")
        .weight(450),
    "NewCMMono10-BookItalic" => Exception::new()
        .family("New Computer Modern Mono")
        .weight(450),
    "NewCMMono10-Italic" => Exception::new()
        .family("New Computer Modern Mono"),
    "NewCMMono10-Regular" => Exception::new()
        .family("New Computer Modern Mono"),
    "NewCMSans08-Book" => Exception::new()
        .family("New Computer Modern Sans 08")
        .weight(450),
    "NewCMSans08-BookOblique" => Exception::new()
        .family("New Computer Modern Sans 08")
        .weight(450),
    "NewCMSans08-Oblique" => Exception::new()
        .family("New Computer Modern Sans 08"),
    "NewCMSans08-Regular" => Exception::new()
        .family("New Computer Modern Sans 08"),
    "NewCMSans10-Bold" => Exception::new()
        .family("New Computer Modern Sans"),
    "NewCMSans10-BoldOblique" => Exception::new()
        .family("New Computer Modern Sans"),
    "NewCMSans10-Book" => Exception::new()
        .family("New Computer Modern Sans")
        .weight(450),
    "NewCMSans10-BookOblique" => Exception::new()
        .family("New Computer Modern Sans")
        .weight(450)
        .style(FontStyle::Oblique),
    "NewCMSans10-Oblique" => Exception::new()
        .family("New Computer Modern Sans")
        .style(FontStyle::Oblique),
    "NewCMSans10-Regular" => Exception::new()
        .family("New Computer Modern Sans"),
    "NewCMUncial08-Bold" => Exception::new()
        .family("New Computer Modern Uncial 08"),
    "NewCMUncial08-Book" => Exception::new()
        .family("New Computer Modern Uncial 08")
        .weight(450),
    "NewCMUncial08-Regular" => Exception::new()
        .family("New Computer Modern Uncial 08"),
    "NewCMUncial10-Bold" => Exception::new()
        .family("New Computer Modern Uncial"),
    "NewCMUncial10-Book" => Exception::new()
        .family("New Computer Modern Uncial")
        .weight(450),
    "NewCMUncial10-Regular" => Exception::new()
        .family("New Computer Modern Uncial"),
};
