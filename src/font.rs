//! Font handling.

use std::fmt::{self, Display, Formatter};

use fontdock::FaceFromVec;

use crate::geom::Length;

/// An owned font face.
pub struct FaceBuf {
    data: Box<[u8]>,
    index: u32,
    inner: rustybuzz::Face<'static>,
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

    /// Get a reference to the underlying ttf-parser/rustybuzz face.
    pub fn ttf(&self) -> &rustybuzz::Face<'_> {
        // We can't implement Deref because that would leak the internal 'static
        // lifetime.
        &self.inner
    }

    /// Look up a vertical metric.
    pub fn vertical_metric(&self, metric: VerticalFontMetric) -> EmLength {
        self.convert(match metric {
            VerticalFontMetric::Ascender => self.ascender,
            VerticalFontMetric::CapHeight => self.cap_height,
            VerticalFontMetric::XHeight => self.x_height,
            VerticalFontMetric::Baseline => 0.0,
            VerticalFontMetric::Descender => self.descender,
        })
    }

    /// Convert from font units to an em length length.
    pub fn convert(&self, units: impl Into<f64>) -> EmLength {
        EmLength(units.into() / self.units_per_em)
    }
}

impl FaceFromVec for FaceBuf {
    fn from_vec(vec: Vec<u8>, index: u32) -> Option<Self> {
        let data = vec.into_boxed_slice();

        // SAFETY: The slices's location is stable in memory since we don't
        //         touch it and it can't be touched from outside this type.
        let slice: &'static [u8] =
            unsafe { std::slice::from_raw_parts(data.as_ptr(), data.len()) };

        let inner = rustybuzz::Face::from_slice(slice, index)?;

        // Look up some metrics we may need often.
        let units_per_em = inner.units_per_em();
        let ascender = inner.typographic_ascender().unwrap_or(inner.ascender());
        let cap_height = inner.capital_height().filter(|&h| h > 0).unwrap_or(ascender);
        let x_height = inner.x_height().filter(|&h| h > 0).unwrap_or(ascender);
        let descender = inner.typographic_descender().unwrap_or(inner.descender());

        Some(Self {
            data,
            index,
            inner,
            units_per_em: f64::from(units_per_em),
            ascender: f64::from(ascender),
            cap_height: f64::from(cap_height),
            x_height: f64::from(x_height),
            descender: f64::from(descender),
        })
    }
}

/// A length in resolved em units.
#[derive(Default, Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct EmLength(f64);

impl EmLength {
    /// Convert to a length at the given font size.
    pub fn scale(self, size: Length) -> Length {
        self.0 * size
    }

    /// Get the number of em units.
    pub fn get(self) -> f64 {
        self.0
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
