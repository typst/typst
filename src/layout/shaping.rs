//! Super-basic text shaping.
//!
//! This is really only suited for simple Latin text. It picks the most suitable
//! font for each individual character. When the direction is right-to-left, the
//! word is spelled backwards. Vertical shaping is not supported.

use std::fmt::{self, Debug, Display, Formatter};

use fontdock::{FaceId, FontVariant};
use ttf_parser::{Face, GlyphId};

use crate::env::FontLoader;
use crate::exec::FamilyMap;
use crate::geom::{Dir, Length, Point, Size};
use crate::layout::{Element, Fill, Frame};

/// A shaped run of text.
#[derive(Clone, PartialEq)]
pub struct Shaped {
    /// The shaped text.
    pub text: String,
    /// The font face the text was shaped with.
    pub face: FaceId,
    /// The shaped glyphs.
    pub glyphs: Vec<GlyphId>,
    /// The horizontal offsets of the glyphs. This is indexed parallel to
    /// `glyphs`. Vertical offsets are not yet supported.
    pub offsets: Vec<Length>,
    /// The font size.
    pub font_size: Length,
    /// The glyph fill color / texture.
    pub color: Fill,
}

impl Shaped {
    /// Create a new shape run with empty `text`, `glyphs` and `offsets`.
    pub fn new(face: FaceId, font_size: Length, color: Fill) -> Self {
        Self {
            text: String::new(),
            face,
            glyphs: vec![],
            offsets: vec![],
            font_size,
            color,
        }
    }

    /// Encode the glyph ids into a big-endian byte buffer.
    pub fn encode_glyphs_be(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(2 * self.glyphs.len());
        for &GlyphId(g) in &self.glyphs {
            bytes.push((g >> 8) as u8);
            bytes.push((g & 0xff) as u8);
        }
        bytes
    }
}

impl Debug for Shaped {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.text, f)
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

/// Shape text into a frame containing [`Shaped`] runs.
pub fn shape(
    text: &str,
    dir: Dir,
    families: &FamilyMap,
    variant: FontVariant,
    font_size: Length,
    top_edge: VerticalFontMetric,
    bottom_edge: VerticalFontMetric,
    color: Fill,
    loader: &mut FontLoader,
) -> Frame {
    let mut frame = Frame::new(Size::new(Length::ZERO, Length::ZERO));
    let mut shaped = Shaped::new(FaceId::MAX, font_size, color);
    let mut width = Length::ZERO;
    let mut top = Length::ZERO;
    let mut bottom = Length::ZERO;

    // Create an iterator with conditional direction.
    let mut forwards = text.chars();
    let mut backwards = text.chars().rev();
    let chars: &mut dyn Iterator<Item = char> = if dir.is_positive() {
        &mut forwards
    } else {
        &mut backwards
    };

    for c in chars {
        for family in families.iter() {
            if let Some(id) = loader.query(family, variant) {
                let face = loader.face(id).get();
                let (glyph, glyph_width) = match lookup_glyph(face, c) {
                    Some(v) => v,
                    None => continue,
                };

                let units_per_em = f64::from(face.units_per_em().unwrap_or(1000));
                let convert = |units| units / units_per_em * font_size;

                // Flush the buffer and reset the metrics if we use a new font face.
                if shaped.face != id {
                    place(&mut frame, shaped, width, top, bottom);

                    shaped = Shaped::new(id, font_size, color);
                    width = Length::ZERO;
                    top = convert(f64::from(lookup_metric(face, top_edge)));
                    bottom = convert(f64::from(lookup_metric(face, bottom_edge)));
                }

                shaped.text.push(c);
                shaped.glyphs.push(glyph);
                shaped.offsets.push(width);
                width += convert(f64::from(glyph_width));
                break;
            }
        }
    }

    place(&mut frame, shaped, width, top, bottom);

    frame
}

/// Place shaped text into a frame.
fn place(frame: &mut Frame, shaped: Shaped, width: Length, top: Length, bottom: Length) {
    if !shaped.text.is_empty() {
        frame.push(Point::new(frame.size.width, top), Element::Text(shaped));
        frame.size.width += width;
        frame.size.height = frame.size.height.max(top - bottom);
    }
}

/// Look up the glyph for `c` and returns its index alongside its advance width.
fn lookup_glyph(face: &Face, c: char) -> Option<(GlyphId, u16)> {
    let glyph = face.glyph_index(c)?;
    let width = face.glyph_hor_advance(glyph)?;
    Some((glyph, width))
}

/// Look up a vertical metric.
fn lookup_metric(face: &Face, metric: VerticalFontMetric) -> i16 {
    match metric {
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

/// The ascender of the face.
fn lookup_ascender(face: &Face) -> i16 {
    // We prefer the typographic ascender over the Windows ascender because
    // it can be overly large if the font has large glyphs.
    face.typographic_ascender().unwrap_or_else(|| face.ascender())
}

/// The descender of the face.
fn lookup_descender(face: &Face) -> i16 {
    // See `lookup_ascender` for reason.
    face.typographic_descender().unwrap_or_else(|| face.descender())
}
