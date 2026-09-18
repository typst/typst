use serde::Deserialize;

use super::{FontStretch, FontStyle, FontWeight};

pub fn find_exception(postscript_name: &str) -> Option<&'static Exception> {
    EXCEPTION_MAP.get(postscript_name)
}

#[derive(Debug, Copy, Clone, Default, Deserialize)]
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

    const fn stretch(self, stretch: u16) -> Self {
        Self { stretch: Some(FontStretch(stretch)), ..self }
    }
}

const E: Exception = Exception::new();

/// A map which keys are PostScript name and values are override entries.
static EXCEPTION_MAP: phf::Map<&'static str, Exception> = phf::phf_map! {
    // The old version of Arial-Black, published by Microsoft in 1996 in their
    // "core fonts for the web" project, has a wrong weight of 400. See
    // https://corefonts.sourceforge.net/.
    "Arial-Black" => E.weight(900),

    // Archivo Narrow is different from Archivo and Archivo Black. Since Archivo
    // Black seems identical to Archivo weight 900, only differentiate between
    // Archivo and Archivo Narrow.
    "ArchivoNarrow-Regular" => E.family("Archivo Narrow"),
    "ArchivoNarrow-Italic" => E.family("Archivo Narrow"),
    "ArchivoNarrow-Bold" => E.family("Archivo Narrow"),
    "ArchivoNarrow-BoldItalic" => E.family("Archivo Narrow"),

    // Fandol fonts designed for Chinese typesetting.
    // See https://ctan.org/tex-archive/fonts/fandol/.
    "FandolHei-Bold" => E.weight(700),
    "FandolSong-Bold" => E.weight(700),

    // IBM Plex
    "IBMPlexMono-Medm" => E.family("IBM Plex Mono"),
    "IBMPlexMono-MedmItalic" => E.family("IBM Plex Mono"),
    "IBMPlexMono-SmBld" => E.family("IBM Plex Mono"),
    "IBMPlexMono-SmBldItalic" => E.family("IBM Plex Mono"),
    "IBMPlexMono-Text" => E.family("IBM Plex Mono"),
    "IBMPlexMono-TextItalic" => E.family("IBM Plex Mono"),
    "IBMPlexSans-Medm" => E.family("IBM Plex Sans"),
    "IBMPlexSans-MedmItalic" => E.family("IBM Plex Sans"),
    "IBMPlexSans-SmBld" => E.family("IBM Plex Sans"),
    "IBMPlexSans-SmBldItalic" => E.family("IBM Plex Sans"),
    "IBMPlexSans-Text" => E.family("IBM Plex Sans"),
    "IBMPlexSans-TextItalic" => E.family("IBM Plex Sans"),
    "IBMPlexSansArabic-Medium" => E.family("IBM Plex Sans Arabic"),
    "IBMPlexSansArabic-SemiBold" => E.family("IBM Plex Sans Arabic"),
    "IBMPlexSansArabic-Text" => E.family("IBM Plex Sans Arabic"),
    "IBMPlexSansCond-Medm" => E.family("IBM Plex Sans"),
    "IBMPlexSansCond-MedmItalic" => E.family("IBM Plex Sans"),
    "IBMPlexSansCond-SmBld" => E.family("IBM Plex Sans"),
    "IBMPlexSansCond-SmBldItalic" => E.family("IBM Plex Sans"),
    "IBMPlexSansCond-Text" => E.family("IBM Plex Sans"),
    "IBMPlexSansCond-TextItalic" => E.family("IBM Plex Sans"),
    "IBMPlexSansDevanagari-Medm" => E.family("IBM Plex Sans Devanagari"),
    "IBMPlexSansDevanagari-SmBld" => E.family("IBM Plex Sans Devanagari"),
    "IBMPlexSansDevanagari-Text" => E.family("IBM Plex Sans Devanagari"),
    "IBMPlexSansHebrew-Medm" => E.family("IBM Plex Sans Hebrew"),
    "IBMPlexSansHebrew-SmBld" => E.family("IBM Plex Sans Hebrew"),
    "IBMPlexSansHebrew-Text" => E.family("IBM Plex Sans Hebrew"),
    "IBMPlexSansJP-Medm" => E.family("IBM Plex Sans JP"),
    "IBMPlexSansJP-SmBld" => E.family("IBM Plex Sans JP"),
    "IBMPlexSansJP-Text" => E.family("IBM Plex Sans JP"),
    "IBMPlexSansKR-Medm" => E.family("IBM Plex Sans KR"),
    "IBMPlexSansKR-SmBld" => E.family("IBM Plex Sans KR"),
    "IBMPlexSansKR-Text" => E.family("IBM Plex Sans KR"),
    "IBMPlexSansSC-Medm" => E.family("IBM Plex Sans SC"),
    "IBMPlexSansSC-SmBld" => E.family("IBM Plex Sans SC"),
    "IBMPlexSansSC-Text" => E.family("IBM Plex Sans SC"),
    "IBMPlexSansTC-Medm" => E.family("IBM Plex Sans TC"),
    "IBMPlexSansTC-SmBld" => E.family("IBM Plex Sans TC"),
    "IBMPlexSansTC-Text" => E.family("IBM Plex Sans TC"),
    "IBMPlexSansThai-Medm" => E.family("IBM Plex Sans Thai"),
    "IBMPlexSansThai-SmBld" => E.family("IBM Plex Sans Thai"),
    "IBMPlexSansThai-Text" => E.family("IBM Plex Sans Thai"),
    "IBMPlexSansThaiLooped-Medm" => E.family("IBM Plex Sans Thai Looped"),
    "IBMPlexSansThaiLooped-SmBld" => E.family("IBM Plex Sans Thai Looped"),
    "IBMPlexSansThaiLooped-Text" => E.family("IBM Plex Sans Thai Looped"),
    "IBMPlexSerif-Medium" => E.family("IBM Plex Serif"),
    "IBMPlexSerif-MediumItalic" => E.family("IBM Plex Serif"),
    "IBMPlexSerif-SemiBold" => E.family("IBM Plex Serif"),
    "IBMPlexSerif-SemiBoldItalic" => E.family("IBM Plex Serif"),
    "IBMPlexSerif-Text" => E.family("IBM Plex Serif"),
    "IBMPlexSerif-TextItalic" => E.family("IBM Plex Serif"),

    // Latin Modern
    "LMMono8-Regular" => E.family("Latin Modern Mono 8"),
    "LMMono9-Regular" => E.family("Latin Modern Mono 9"),
    "LMMono12-Regular" => E.family("Latin Modern Mono 12"),
    "LMMonoLt10-BoldOblique" => E.style(FontStyle::Oblique),
    "LMMonoLt10-Regular" => E.weight(300),
    "LMMonoLt10-Oblique" => E.weight(300).style(FontStyle::Oblique),
    "LMMonoLtCond10-Regular" => E.weight(300).stretch(666),
    "LMMonoLtCond10-Oblique" => E.weight(300).style(FontStyle::Oblique).stretch(666),
    "LMMonoPropLt10-Regular" => E.weight(300),
    "LMMonoPropLt10-Oblique" => E.weight(300),
    "LMRoman5-Regular" => E.family("Latin Modern Roman 5"),
    "LMRoman6-Regular" => E.family("Latin Modern Roman 6"),
    "LMRoman7-Regular" => E.family("Latin Modern Roman 7"),
    "LMRoman8-Regular" => E.family("Latin Modern Roman 8"),
    "LMRoman9-Regular" => E.family("Latin Modern Roman 9"),
    "LMRoman12-Regular" => E.family("Latin Modern Roman 12"),
    "LMRoman17-Regular" => E.family("Latin Modern Roman 17"),
    "LMRoman7-Italic" => E.family("Latin Modern Roman 7"),
    "LMRoman8-Italic" => E.family("Latin Modern Roman 8"),
    "LMRoman9-Italic" => E.family("Latin Modern Roman 9"),
    "LMRoman12-Italic" => E.family("Latin Modern Roman 12"),
    "LMRoman5-Bold" => E.family("Latin Modern Roman 5"),
    "LMRoman6-Bold" => E.family("Latin Modern Roman 6"),
    "LMRoman7-Bold" => E.family("Latin Modern Roman 7"),
    "LMRoman8-Bold" => E.family("Latin Modern Roman 8"),
    "LMRoman9-Bold" => E.family("Latin Modern Roman 9"),
    "LMRoman12-Bold" => E.family("Latin Modern Roman 12"),
    "LMRomanSlant8-Regular" => E.family("Latin Modern Roman 8"),
    "LMRomanSlant9-Regular" => E.family("Latin Modern Roman 9"),
    "LMRomanSlant12-Regular" => E.family("Latin Modern Roman 12"),
    "LMRomanSlant17-Regular" => E.family("Latin Modern Roman 17"),
    "LMSans8-Regular" => E.family("Latin Modern Sans 8"),
    "LMSans9-Regular" => E.family("Latin Modern Sans 9"),
    "LMSans12-Regular" => E.family("Latin Modern Sans 12"),
    "LMSans17-Regular" => E.family("Latin Modern Sans 17"),
    "LMSans8-Oblique" => E.family("Latin Modern Sans 8"),
    "LMSans9-Oblique" => E.family("Latin Modern Sans 9"),
    "LMSans12-Oblique" => E.family("Latin Modern Sans 12"),
    "LMSans17-Oblique" => E.family("Latin Modern Sans 17"),

    // Noto
    "NotoNaskhArabicUISemi-Bold" => E.family("Noto Naskh Arabic UI").weight(600),
    "NotoSansSoraSompengSemi-Bold" => E.family("Noto Sans Sora Sompeng").weight(600),
    "NotoSans-DisplayBlackItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayCondensedBlackItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayCondensedBold" => E.family("Noto Sans Display"),
    "NotoSans-DisplayCondensedBoldItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayCondensedExtraBoldItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayCondensedExtraLightItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayCondensedItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayCondensedLightItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayCondensedMediumItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayCondensedSemiBoldItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayCondensedThinItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraBoldItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedBlackItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedBold" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedBoldItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedExtraBoldItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedExtraLightItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedLightItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedMediumItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedSemiBoldItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraCondensedThinItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayExtraLightItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayLightItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayMediumItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplaySemiBoldItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedBlackItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedBold" => E.family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedBoldItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedExtraBoldItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedExtraLightItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedLightItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedMediumItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedSemiBoldItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplaySemiCondensedThinItalic" => E.family("Noto Sans Display"),
    "NotoSans-DisplayThinItalic" => E.family("Noto Sans Display"),

    // The following three postscript names are only used in the version 2.007
    // of the Noto Sans font. Other versions, while have different postscript
    // name, happen to have correct metadata.
    "NotoSerif-DisplayCondensedBold" => E.family("Noto Serif Display"),
    "NotoSerif-DisplayExtraCondensedBold" => E.family("Noto Serif Display"),
    "NotoSerif-DisplaySemiCondensedBold" => E.family("Noto Serif Display"),

    // New Computer Modern
    "NewCM08-Book" => E.family("New Computer Modern 08").weight(450),
    "NewCM08-BookItalic" => E.family("New Computer Modern 08").weight(450),
    "NewCM08-Italic" => E.family("New Computer Modern 08"),
    "NewCM08-Regular" => E.family("New Computer Modern 08"),
    "NewCM10-Bold" => E.family("New Computer Modern"),
    "NewCM10-BoldItalic" => E.family("New Computer Modern"),
    "NewCM10-Book" => E.family("New Computer Modern").weight(450),
    "NewCM10-BookItalic" => E.family("New Computer Modern").weight(450),
    "NewCM10-Italic" => E.family("New Computer Modern"),
    "NewCM10-Regular" => E.family("New Computer Modern"),
    "NewCMMath-Bold" => E.family("New Computer Modern Math"),
    "NewCMMath-Book" => E.family("New Computer Modern Math").weight(450),
    "NewCMMath-Regular" => E.family("New Computer Modern Math"),
    "NewCMMono10-Bold" => E.family("New Computer Modern Mono"),
    "NewCMMono10-BoldOblique" => E.family("New Computer Modern Mono"),
    "NewCMMono10-Book" => E.family("New Computer Modern Mono").weight(450),
    "NewCMMono10-BookItalic" => E.family("New Computer Modern Mono").weight(450),
    "NewCMMono10-Italic" => E.family("New Computer Modern Mono"),
    "NewCMMono10-Regular" => E.family("New Computer Modern Mono"),
    "NewCMSans08-Book" => E.family("New Computer Modern Sans 08").weight(450),
    "NewCMSans08-BookOblique" => E.family("New Computer Modern Sans 08").weight(450),
    "NewCMSans08-Oblique" => E.family("New Computer Modern Sans 08"),
    "NewCMSans08-Regular" => E.family("New Computer Modern Sans 08"),
    "NewCMSans10-Bold" => E.family("New Computer Modern Sans"),
    "NewCMSans10-BoldOblique" => E.family("New Computer Modern Sans"),
    "NewCMSans10-Book" => E.family("New Computer Modern Sans").weight(450),
    "NewCMSans10-BookOblique" => E.family("New Computer Modern Sans").weight(450).style(FontStyle::Oblique),
    "NewCMSans10-Oblique" => E.family("New Computer Modern Sans").style(FontStyle::Oblique),
    "NewCMSans10-Regular" => E.family("New Computer Modern Sans"),
    "NewCMSansMath-Regular" => E.family("New Computer Modern Sans Math"),
    "NewCMUncial08-Bold" => E.family("New Computer Modern Uncial 08"),
    "NewCMUncial08-Book" => E.family("New Computer Modern Uncial 08").weight(450),
    "NewCMUncial08-Regular" => E.family("New Computer Modern Uncial 08"),
    "NewCMUncial10-Bold" => E.family("New Computer Modern Uncial"),
    "NewCMUncial10-Book" => E.family("New Computer Modern Uncial").weight(450),
    "NewCMUncial10-Regular" => E.family("New Computer Modern Uncial"),

    // SimSun-ExtB is a CJK Extension B font, not an "ExtraBold" variant of
    // SimSun. Without this exception, `typographic_family()` strips the "ExtB"
    // suffix and merges it with SimSun, causing wrong font selection.
    "SimSun-ExtB" => E.family("SimSun-ExtB"),

    // STKaiti is a set of Kai fonts. Their weight values need to be corrected
    // according to their PostScript names.
    "STKaitiSC-Regular" => E.weight(400),
    "STKaitiTC-Regular" => E.weight(400),
    "STKaitiSC-Bold" => E.weight(700),
    "STKaitiTC-Bold" => E.weight(700),
    "STKaitiSC-Black" => E.weight(900),
    "STKaitiTC-Black" => E.weight(900),
};
