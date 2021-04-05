//! Font handling.

use std::fmt::{self, Display, Formatter};

use fontdock::FaceFromVec;

use crate::geom::Length;

/// An owned font face.
pub struct FaceBuf {
    data: Box<[u8]>,
    index: u32,
    ttf: ttf_parser::Face<'static>,
    buzz: rustybuzz::Face<'static>,
    units_per_em: f64,
    ascender: f64,
    cap_height: f64,
    x_height: f64,
    descender: f64,
}

impl FaceBuf {
    /// The raw face data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// The collection index.
    pub fn index(&self) -> u32 {
        self.index
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

    /// Look up a vertical metric at a given font size.
    pub fn vertical_metric(&self, size: Length, metric: VerticalFontMetric) -> Length {
        self.convert(size, match metric {
            VerticalFontMetric::Ascender => self.ascender,
            VerticalFontMetric::CapHeight => self.cap_height,
            VerticalFontMetric::XHeight => self.x_height,
            VerticalFontMetric::Baseline => 0.0,
            VerticalFontMetric::Descender => self.descender,
        })
    }

    /// Convert from font units to a length at a given font size.
    pub fn convert(&self, size: Length, units: impl Into<f64>) -> Length {
        units.into() / self.units_per_em * size
    }
}

impl FaceFromVec for FaceBuf {
    fn from_vec(vec: Vec<u8>, index: u32) -> Option<Self> {
        let data = vec.into_boxed_slice();

        // SAFETY: The slices's location is stable in memory since we don't
        //         touch it and it can't be touched from outside this type.
        let slice: &'static [u8] =
            unsafe { std::slice::from_raw_parts(data.as_ptr(), data.len()) };

        let ttf = ttf_parser::Face::from_slice(slice, index).ok()?;
        let buzz = rustybuzz::Face::from_slice(slice, index)?;

        // Look up some metrics we may need often.
        let units_per_em = ttf.units_per_em().unwrap_or(1000);
        let ascender = ttf.typographic_ascender().unwrap_or(ttf.ascender());
        let cap_height = ttf.capital_height().filter(|&h| h > 0).unwrap_or(ascender);
        let x_height = ttf.x_height().filter(|&h| h > 0).unwrap_or(ascender);
        let descender = ttf.typographic_descender().unwrap_or(ttf.descender());

        Some(Self {
            data,
            index,
            ttf,
            buzz,
            units_per_em: f64::from(units_per_em),
            ascender: f64::from(ascender),
            cap_height: f64::from(cap_height),
            x_height: f64::from(x_height),
            descender: f64::from(descender),
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
