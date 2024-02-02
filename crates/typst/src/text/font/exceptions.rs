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
    // The old version of Arial-Black, published by Microsoft in 1996 in their "core fonts for the web" project, has a wrong weight of 400.
    // See https://corefonts.sourceforge.net/.
    "Arial-Black" => Exception::new()
        .weight(900),
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
    "LMMono8-Regular" => Exception::new()
        .family("Latin Modern Mono 8"),
    "LMMono9-Regular" => Exception::new()
        .family("Latin Modern Mono 9"),
    "LMMono12-Regular" => Exception::new()
        .family("Latin Modern Mono 12"),
    "LMMonoLt10-BoldOblique" => Exception::new()
        .style(FontStyle::Oblique),
    "LMMonoLt10-Regular" => Exception::new()
        .weight(300),
    "LMMonoLt10-Oblique" => Exception::new()
        .weight(300)
        .style(FontStyle::Oblique),
    "LMMonoLtCond10-Regular" => Exception::new()
        .weight(300)
        .stretch(666),
    "LMMonoLtCond10-Oblique" => Exception::new()
        .weight(300)
        .style(FontStyle::Oblique)
        .stretch(666),
    "LMMonoPropLt10-Regular" => Exception::new()
        .weight(300),
    "LMMonoPropLt10-Oblique" => Exception::new()
        .weight(300),
    "LMRoman5-Regular" => Exception::new()
        .family("Latin Modern Roman 5"),
    "LMRoman6-Regular" => Exception::new()
        .family("Latin Modern Roman 6"),
    "LMRoman7-Regular" => Exception::new()
        .family("Latin Modern Roman 7"),
    "LMRoman8-Regular" => Exception::new()
        .family("Latin Modern Roman 8"),
    "LMRoman9-Regular" => Exception::new()
        .family("Latin Modern Roman 9"),
    "LMRoman12-Regular" => Exception::new()
        .family("Latin Modern Roman 12"),
    "LMRoman17-Regular" => Exception::new()
        .family("Latin Modern Roman 17"),
    "LMRoman7-Italic" => Exception::new()
        .family("Latin Modern Roman 7"),
    "LMRoman8-Italic" => Exception::new()
        .family("Latin Modern Roman 8"),
    "LMRoman9-Italic" => Exception::new()
        .family("Latin Modern Roman 9"),
    "LMRoman12-Italic" => Exception::new()
        .family("Latin Modern Roman 12"),
    "LMRoman5-Bold" => Exception::new()
        .family("Latin Modern Roman 5"),
    "LMRoman6-Bold" => Exception::new()
        .family("Latin Modern Roman 6"),
    "LMRoman7-Bold" => Exception::new()
        .family("Latin Modern Roman 7"),
    "LMRoman8-Bold" => Exception::new()
        .family("Latin Modern Roman 8"),
    "LMRoman9-Bold" => Exception::new()
        .family("Latin Modern Roman 9"),
    "LMRoman12-Bold" => Exception::new()
        .family("Latin Modern Roman 12"),
    "LMRomanSlant8-Regular" => Exception::new()
        .family("Latin Modern Roman 8"),
    "LMRomanSlant9-Regular" => Exception::new()
        .family("Latin Modern Roman 9"),
    "LMRomanSlant12-Regular" => Exception::new()
        .family("Latin Modern Roman 12"),
    "LMRomanSlant17-Regular" => Exception::new()
        .family("Latin Modern Roman 17"),
    "LMSans8-Regular" => Exception::new()
        .family("Latin Modern Sans 8"),
    "LMSans9-Regular" => Exception::new()
        .family("Latin Modern Sans 9"),
    "LMSans12-Regular" => Exception::new()
        .family("Latin Modern Sans 12"),
    "LMSans17-Regular" => Exception::new()
        .family("Latin Modern Sans 17"),
    "LMSans8-Oblique" => Exception::new()
        .family("Latin Modern Sans 8"),
    "LMSans9-Oblique" => Exception::new()
        .family("Latin Modern Sans 9"),
    "LMSans12-Oblique" => Exception::new()
        .family("Latin Modern Sans 12"),
    "LMSans17-Oblique" => Exception::new()
        .family("Latin Modern Sans 17"),
};
