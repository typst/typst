//! Font handling.

use std::fmt::{self, Display, Formatter};

use fontdock::FaceFromVec;

/// An owned font face.
pub struct FaceBuf {
    data: Box<[u8]>,
    ttf: ttf_parser::Face<'static>,
    buzz: rustybuzz::Face<'static>,
}

impl FaceBuf {
    /// The raw face data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get a reference to the underlying ttf-parser face.
    pub fn ttf(&self) -> &ttf_parser::Face<'_> {
        // We can't implement Deref because that would leak the internal 'static
        // lifetime.
        &self.ttf
    }

    /// Get a reference to the underlying rustybuzz face.
    pub fn buzz(&self) -> &rustybuzz::Face<'_> {
        // We can't implement Deref because that would leak the internal 'static
        // lifetime.
        &self.buzz
    }
}

impl FaceFromVec for FaceBuf {
    fn from_vec(vec: Vec<u8>, i: u32) -> Option<Self> {
        let data = vec.into_boxed_slice();

        // SAFETY: The slices's location is stable in memory since we don't
        //         touch it and it can't be touched from outside this type.
        let slice: &'static [u8] =
            unsafe { std::slice::from_raw_parts(data.as_ptr(), data.len()) };

        Some(Self {
            data,
            ttf: ttf_parser::Face::from_slice(slice, i).ok()?,
            buzz: rustybuzz::Face::from_slice(slice, i)?,
        })
    }
}

/// Identifies a vertical metric of a font.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum VerticalFontMetric {
    /// The distance from the baseline to the typographic ascender.
    ///
    /// Corresponds to the typographic ascender from the `OS/2` table if present
    /// and falls back to the ascender from the `hhea` table otherwise.
    Ascender,
    /// The approximate height of uppercase letters.
    CapHeight,
    /// The approximate height of non-ascending lowercase letters.
    XHeight,
    /// The baseline on which the letters rest.
    Baseline,
    /// The distance from the baseline to the typographic descender.
    ///
    /// Corresponds to the typographic descender from the `OS/2` table if
    /// present and falls back to the descender from the `hhea` table otherwise.
    Descender,
}

impl VerticalFontMetric {
    /// Look up the metric in the given font face.
    pub fn lookup(self, face: &ttf_parser::Face) -> i16 {
        match self {
            VerticalFontMetric::Ascender => lookup_ascender(face),
            VerticalFontMetric::CapHeight => face
                .capital_height()
                .filter(|&h| h > 0)
                .unwrap_or_else(|| lookup_ascender(face)),
            VerticalFontMetric::XHeight => face
                .x_height()
                .filter(|&h| h > 0)
                .unwrap_or_else(|| lookup_ascender(face)),
            VerticalFontMetric::Baseline => 0,
            VerticalFontMetric::Descender => lookup_descender(face),
        }
    }
}

/// The ascender of the face.
fn lookup_ascender(face: &ttf_parser::Face) -> i16 {
    // We prefer the typographic ascender over the Windows ascender because
    // it can be overly large if the font has large glyphs.
    face.typographic_ascender().unwrap_or_else(|| face.ascender())
}

/// The descender of the face.
fn lookup_descender(face: &ttf_parser::Face) -> i16 {
    // See `lookup_ascender` for reason.
    face.typographic_descender().unwrap_or_else(|| face.descender())
}

impl Display for VerticalFontMetric {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Ascender => "ascender",
            Self::CapHeight => "cap-height",
            Self::XHeight => "x-height",
            Self::Baseline => "baseline",
            Self::Descender => "descender",
        })
    }
}
