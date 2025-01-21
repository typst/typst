use serde::Deserialize;

use super::{FontStretch, FontStyle, FontWeight};

pub fn find_exception(postscript_name: &str) -> Option<&'static Exception> {
    EXCEPTION_MAP.get(postscript_name)
}

#[derive(Debug, Default, Deserialize)]
pub struct Exception {
    pub family: Option<&'static str>,
    pub style: Option<FontStyle>,
    pub weight: Option<FontWeight>,
    pub stretch: Option<FontStretch>,
}

impl Exception {
    const fn new() -> Self {
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

/// A map which keys are PostScript name and values are override entries.
static EXCEPTION_MAP: phf::Map<&'static str, Exception> = phf::phf_map! {
    // The old version of Arial-Black, published by Microsoft in 1996 in their
    // "core fonts for the web" project, has a wrong weight of 400.
    // See https://corefonts.sourceforge.net/.
    "Arial-Black" => Exception::new()
        .weight(900),
    // Archivo Narrow is different from Archivo and Archivo Black. Since Archivo Black
    // seems identical to Archivo weight 900, only differentiate between Archivo and
    // Archivo Narrow.
    "ArchivoNarrow-Regular" => Exception::new()
        .family("Archivo Narrow"),
    "ArchivoNarrow-Italic" => Exception::new()
        .family("Archivo Narrow"),
    "ArchivoNarrow-Bold" => Exception::new()
        .family("Archivo Narrow"),
    "ArchivoNarrow-BoldItalic" => Exception::new()
        .family("Archivo Narrow"),
    // Fandol fonts designed for Chinese typesetting.
    // See https://ctan.org/tex-archive/fonts/fandol/.
    "FandolHei-Bold" => Exception::new()
        .weight(700),
    "FandolSong-Bold" => Exception::new()
        .weight(700),
    // Noto fonts
    "NotoNaskhArabicUISemi-Bold" => Exception::new()
        .family("Noto Naskh Arabic UI")
        .weight(600),
    "NotoSansSoraSompengSemi-Bold" => Exception::new()
        .family("Noto Sans Sora Sompeng")
        .weight(600),
    "NotoSans-DisplayBlackItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayCondensedBlackItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayCondensedBold" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayCondensedBoldItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayCondensedExtraBoldItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayCondensedExtraLightItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayCondensedItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayCondensedLightItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayCondensedMediumItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayCondensedSemiBoldItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayCondensedThinItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraBoldItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedBlackItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedBold" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedBoldItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedExtraBoldItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedExtraLightItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedLightItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedMediumItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedSemiBoldItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedThinItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayExtraLightItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayLightItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayMediumItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplaySemiBoldItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedBlackItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedBold" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedBoldItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedExtraBoldItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedExtraLightItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedLightItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedMediumItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedSemiBoldItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedThinItalic" => Exception::new()
        .family("Noto Sans Display"),
    "NotoSans-DisplayThinItalic" => Exception::new()
        .family("Noto Sans Display"),
    // The following three postscript names are only used in the version 2.007
    // of the Noto Sans font. Other versions, while have different postscript
    // name, happen to have correct metadata.
    "NotoSerif-DisplayCondensedBold" => Exception::new()
        .family("Noto Serif Display"),
    "NotoSerif-DisplayExtraCondensedBold" => Exception::new()
        .family("Noto Serif Display"),
    "NotoSerif-DisplaySemiCondensedBold" => Exception::new()
        .family("Noto Serif Display"),
    // New Computer Modern
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
    "NewCMMath-Bold" => Exception::new()
        .family("New Computer Modern Math"),
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
    "NewCMSansMath-Regular" => Exception::new()
        .family("New Computer Modern Sans Math"),
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
    // Latin Modern
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
    // STKaiti is a set of Kai fonts. Their weight values need to be corrected
    // according to their PostScript names.
    "STKaitiSC-Regular" => Exception::new().weight(400),
    "STKaitiTC-Regular" => Exception::new().weight(400),
    "STKaitiSC-Bold" => Exception::new().weight(700),
    "STKaitiTC-Bold" => Exception::new().weight(700),
    "STKaitiSC-Black" => Exception::new().weight(900),
    "STKaitiTC-Black" => Exception::new().weight(900),
};
