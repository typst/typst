use serde::Deserialize;

use super::{FontStretch, FontStyle, FontWeight};

#[derive(Debug, Default, Deserialize)]
pub struct Exception {
    pub family: Option<&'static str>,
    pub style: Option<FontStyle>,
    pub weight: Option<FontWeight>,
    pub stretch: Option<FontStretch>,
}

pub fn find_exception(postscript_name: &str) -> Option<&'static Exception> {
    EXCEPTION_MAP.get(postscript_name)
}

/// A map which keys are PostScript name and values are override entries.
static EXCEPTION_MAP: phf::Map<&'static str, Exception> = phf::phf_map! {
    "NewCM08-Book" => Exception {
        family: Some("New Computer Modern 08"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCM08-BookItalic" => Exception {
        family: Some("New Computer Modern 08"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCM08-Italic" => Exception {
        family: Some("New Computer Modern 08"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCM08-Regular" => Exception {
        family: Some("New Computer Modern 08"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCM10-Bold" => Exception {
        family: Some("New Computer Modern"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCM10-BoldItalic" => Exception {
        family: Some("New Computer Modern"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCM10-Book" => Exception {
        family: Some("New Computer Modern"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCM10-BookItalic" => Exception {
        family: Some("New Computer Modern"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCM10-Italic" => Exception {
        family: Some("New Computer Modern"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCM10-Regular" => Exception {
        family: Some("New Computer Modern"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMMath-Book" => Exception {
        family: Some("New Computer Modern Math"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCMMath-Regular" => Exception {
        family: Some("New Computer Modern Math"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMMono10-Bold" => Exception {
        family: Some("New Computer Modern Mono"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMMono10-BoldOblique" => Exception {
        family: Some("New Computer Modern Mono"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMMono10-Book" => Exception {
        family: Some("New Computer Modern Mono"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCMMono10-BookItalic" => Exception {
        family: Some("New Computer Modern Mono"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCMMono10-Italic" => Exception {
        family: Some("New Computer Modern Mono"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMMono10-Regular" => Exception {
        family: Some("New Computer Modern Mono"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMSans08-Book" => Exception {
        family: Some("New Computer Modern Sans 08"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCMSans08-BookOblique" => Exception {
        family: Some("New Computer Modern Sans 08"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCMSans08-Oblique" => Exception {
        family: Some("New Computer Modern Sans 08"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMSans08-Regular" => Exception {
        family: Some("New Computer Modern Sans 08"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMSans10-Bold" => Exception {
        family: Some("New Computer Modern Sans"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMSans10-BoldOblique" => Exception {
        family: Some("New Computer Modern Sans"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMSans10-Book" => Exception {
        family: Some("New Computer Modern Sans"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCMSans10-BookOblique" => Exception {
        family: Some("New Computer Modern Sans"),
        weight: Some(FontWeight(450)),
        style: Some(FontStyle::Oblique),
        stretch: None,
    },
    "NewCMSans10-Oblique" => Exception {
        family: Some("New Computer Modern Sans"),
        weight: None,
        style: Some(FontStyle::Oblique),
        stretch: None,
    },
    "NewCMSans10-Regular" => Exception {
        family: Some("New Computer Modern Sans"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMUncial08-Bold" => Exception {
        family: Some("New Computer Modern Uncial 08"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMUncial08-Book" => Exception {
        family: Some("New Computer Modern Uncial 08"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCMUncial08-Regular" => Exception {
        family: Some("New Computer Modern Uncial 08"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMUncial10-Bold" => Exception {
        family: Some("New Computer Modern Uncial"),
        weight: None,
        style: None,
        stretch: None,
    },
    "NewCMUncial10-Book" => Exception {
        family: Some("New Computer Modern Uncial"),
        weight: Some(FontWeight(450)),
        style: None,
        stretch: None,
    },
    "NewCMUncial10-Regular" => Exception {
        family: Some("New Computer Modern Uncial"),
        weight: None,
        style: None,
        stretch: None,
    },
};
