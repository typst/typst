//! Super-basic text shaping.
//!
//! This is really only suited for simple Latin text. It picks the most suitable
//! font for each individual character. When the direction is right-to-left, the
//! word is spelled backwards. Vertical shaping is not supported.

use std::fmt::{self, Debug, Formatter};

use fontdock::{FaceId, FaceQuery, FallbackTree, FontVariant};
use ttf_parser::{Face, GlyphId};

use crate::font::FontLoader;
use crate::geom::{Dir, Length, Point, Size};
use crate::layout::{Element, Frame};

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
}

impl Shaped {
    /// Create a new shape run with empty `text`, `glyphs` and `offsets`.
    pub fn new(face: FaceId, font_size: Length) -> Self {
        Self {
            text: String::new(),
            face,
            glyphs: vec![],
            offsets: vec![],
            font_size,
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
        write!(f, "Shaped({})", self.text)
    }
}

/// Shape text into a frame containing [`Shaped`] runs.
pub fn shape(
    text: &str,
    dir: Dir,
    font_size: Length,
    loader: &mut FontLoader,
    fallback: &FallbackTree,
    variant: FontVariant,
) -> Frame {
    let mut frame = Frame::new(Size::new(Length::ZERO, font_size));
    let mut shaped = Shaped::new(FaceId::MAX, font_size);
    let mut offset = Length::ZERO;
    let mut ascender = Length::ZERO;
    let mut descender = Length::ZERO;

    // Create an iterator with conditional direction.
    let mut forwards = text.chars();
    let mut backwards = text.chars().rev();
    let chars: &mut dyn Iterator<Item = char> = if dir.is_positive() {
        &mut forwards
    } else {
        &mut backwards
    };

    for c in chars {
        let query = FaceQuery { fallback: fallback.iter(), variant, c };
        if let Some(id) = loader.query(query) {
            let face = loader.face(id).get();
            let (glyph, width) = match lookup_glyph(face, c) {
                Some(v) => v,
                None => continue,
            };

            let units_per_em = f64::from(face.units_per_em().unwrap_or(1000));
            let convert = |units| units / units_per_em * font_size;

            // Flush the buffer and reset the metrics if we use a new font face.
            if shaped.face != id {
                place(&mut frame, shaped, offset, ascender, descender);

                shaped = Shaped::new(id, font_size);
                offset = Length::ZERO;
                ascender = convert(f64::from(face.ascender()));
                descender = convert(f64::from(face.descender()));
            }

            shaped.text.push(c);
            shaped.glyphs.push(glyph);
            shaped.offsets.push(offset);
            offset += convert(f64::from(width));
        }
    }

    // Flush the last buffered parts of the word.
    place(&mut frame, shaped, offset, ascender, descender);

    frame
}

/// Look up the glyph for `c` and returns its index alongside its advance width.
fn lookup_glyph(face: &Face, c: char) -> Option<(GlyphId, u16)> {
    let glyph = face.glyph_index(c)?;
    let width = face.glyph_hor_advance(glyph)?;
    Some((glyph, width))
}

/// Place shaped text into a frame.
fn place(
    frame: &mut Frame,
    shaped: Shaped,
    offset: Length,
    ascender: Length,
    descender: Length,
) {
    if !shaped.text.is_empty() {
        let pos = Point::new(frame.size.width, ascender);
        frame.push(pos, Element::Text(shaped));
        frame.size.width += offset;
        frame.size.height = frame.size.height.max(ascender - descender);
    }
}
