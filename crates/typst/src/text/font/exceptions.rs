use serde::Deserialize;

use super::{FontStretch, FontStyle, FontWeight};

#[derive(Debug, Default, Deserialize)]
pub struct OverrideEntry {
    pub family: Option<&'static str>,
    pub style: Option<FontStyle>,
    pub weight: Option<FontWeight>,
    pub stretch: Option<FontStretch>,
}

pub fn override_entry(postscript_name: &str) -> Option<&'static OverrideEntry> {
    OVERRIDE_MAP.get(postscript_name)
}

/// A map which keys are PostScript name and values are override entries.
static OVERRIDE_MAP: phf::Map<&'static str, OverrideEntry> = phf::phf_map! {
    "NewCM08-Book" => OverrideEntry {
        family: Some("New Computer Modern 08"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCM08-BookItalic" => OverrideEntry {
        family: Some("New Computer Modern 08"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCM08-Italic" => OverrideEntry {
        family: Some("New Computer Modern 08"),
        ..Default::default()
    },
    "NewCM08-Regular" => OverrideEntry {
        family: Some("New Computer Modern 08"),
        ..Default::default()
    },
    "NewCM10-Bold" => OverrideEntry {
        family: Some("New Computer Modern"),
        ..Default::default()
    },
    "NewCM10-BoldItalic" => OverrideEntry {
        family: Some("New Computer Modern"),
        ..Default::default()
    },
    "NewCM10-Book" => OverrideEntry {
        family: Some("New Computer Modern"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCM10-BookItalic" => OverrideEntry {
        family: Some("New Computer Modern"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCM10-Italic" => OverrideEntry {
        family: Some("New Computer Modern"),
        ..Default::default()
    },
    "NewCM10-Regular" => OverrideEntry {
        family: Some("New Computer Modern"),
        ..Default::default()
    },
    "NewCMMath-Book" => OverrideEntry {
        family: Some("New Computer Modern Math"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCMMath-Regular" => OverrideEntry {
        family: Some("New Computer Modern Math"),
        ..Default::default()
    },
    "NewCMMono10-Bold" => OverrideEntry {
        family: Some("New Computer Modern Mono"),
        ..Default::default()
    },
    "NewCMMono10-BoldOblique" => OverrideEntry {
        family: Some("New Computer Modern Mono"),
        ..Default::default()
    },
    "NewCMMono10-Book" => OverrideEntry {
        family: Some("New Computer Modern Mono"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCMMono10-BookItalic" => OverrideEntry {
        family: Some("New Computer Modern Mono"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCMMono10-Italic" => OverrideEntry {
        family: Some("New Computer Modern Mono"),
        ..Default::default()
    },
    "NewCMMono10-Regular" => OverrideEntry {
        family: Some("New Computer Modern Mono"),
        ..Default::default()
    },
    "NewCMSans08-Book" => OverrideEntry {
        family: Some("New Computer Modern Sans 08"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCMSans08-BookOblique" => OverrideEntry {
        family: Some("New Computer Modern Sans 08"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCMSans08-Oblique" => OverrideEntry {
        family: Some("New Computer Modern Sans 08"),
        ..Default::default()
    },
    "NewCMSans08-Regular" => OverrideEntry {
        family: Some("New Computer Modern Sans 08"),
        ..Default::default()
    },
    "NewCMSans10-Bold" => OverrideEntry {
        family: Some("New Computer Modern Sans"),
        ..Default::default()
    },
    "NewCMSans10-BoldOblique" => OverrideEntry {
        family: Some("New Computer Modern Sans"),
        ..Default::default()
    },
    "NewCMSans10-Book" => OverrideEntry {
        family: Some("New Computer Modern Sans"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCMSans10-BookOblique" => OverrideEntry {
        family: Some("New Computer Modern Sans"),
        weight: Some(FontWeight::from_number(450)),
        style: Some(FontStyle::Oblique),
        ..Default::default()
    },
    "NewCMSans10-Oblique" => OverrideEntry {
        family: Some("New Computer Modern Sans"),
        style: Some(FontStyle::Oblique),
        ..Default::default()
    },
    "NewCMSans10-Regular" => OverrideEntry {
        family: Some("New Computer Modern Sans"),
        ..Default::default()
    },
    "NewCMUncial08-Bold" => OverrideEntry {
        family: Some("New Computer Modern Uncial 08"),
        ..Default::default()
    },
    "NewCMUncial08-Book" => OverrideEntry {
        family: Some("New Computer Modern Uncial 08"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCMUncial08-Regular" => OverrideEntry {
        family: Some("New Computer Modern Uncial 08"),
        ..Default::default()
    },
    "NewCMUncial10-Bold" => OverrideEntry {
        family: Some("New Computer Modern Uncial"),
        ..Default::default()
    },
    "NewCMUncial10-Book" => OverrideEntry {
        family: Some("New Computer Modern Uncial"),
        weight: Some(FontWeight::from_number(450)),
        ..Default::default()
    },
    "NewCMUncial10-Regular" => OverrideEntry {
        family: Some("New Computer Modern Uncial"),
        ..Default::default()
    },
};
