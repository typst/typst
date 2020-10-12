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
use crate::layout::{BoxLayout, LayoutElement};

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
    /// `glyphs`. Vertical offets are not yet supported.
    pub offsets: Vec<Length>,
    /// The font size.
    pub font_size: Length,
}

impl Shaped {
    /// Create a new shape run with empty `text`, `glyphs` and `offsets`.
    pub fn new(face: FaceId, size: Length) -> Self {
        Self {
            text: String::new(),
            face,
            glyphs: vec![],
            offsets: vec![],
            font_size: size,
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

/// Shape text into a box containing [`Shaped`] runs.
///
/// [`Shaped`]: struct.Shaped.html
pub fn shape(
    loader: &mut FontLoader,
    text: &str,
    font_size: Length,
    dir: Dir,
    fallback: &FallbackTree,
    variant: FontVariant,
) -> BoxLayout {
    let mut layout = BoxLayout::new(Size::new(Length::ZERO, font_size));
    let mut shaped = Shaped::new(FaceId::MAX, font_size);
    let mut offset = Length::ZERO;

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
        if let Some((id, owned_face)) = loader.query(query) {
            let face = owned_face.get();
            let (glyph, width) = match lookup_glyph(face, c, font_size) {
                Some(v) => v,
                None => continue,
            };

            // Flush the buffer if we change the font face.
            if shaped.face != id && !shaped.text.is_empty() {
                let pos = Point::new(layout.size.width, Length::ZERO);
                layout.push(pos, LayoutElement::Text(shaped));
                layout.size.width += offset;
                shaped = Shaped::new(FaceId::MAX, font_size);
                offset = Length::ZERO;
            }

            shaped.face = id;
            shaped.text.push(c);
            shaped.glyphs.push(glyph);
            shaped.offsets.push(offset);
            offset += width;
        }
    }

    // Flush the last buffered parts of the word.
    if !shaped.text.is_empty() {
        let pos = Point::new(layout.size.width, Length::ZERO);
        layout.push(pos, LayoutElement::Text(shaped));
        layout.size.width += offset;
    }

    layout
}

/// Looks up the glyph for `c` and returns its index alongside its width at the
/// given `size`.
fn lookup_glyph(face: &Face, c: char, size: Length) -> Option<(GlyphId, Length)> {
    let glyph = face.glyph_index(c)?;

    // Determine the width of the char.
    let units_per_em = face.units_per_em().unwrap_or(1000) as f64;
    let width_units = face.glyph_hor_advance(glyph)? as f64;
    let width = width_units / units_per_em * size;

    Some((glyph, width))
}
