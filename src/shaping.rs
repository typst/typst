//! Super-basic text shaping.
//!
//! This is really only suited for simple Latin text. It picks the most suitable
//! font for each individual character. When the direction is right-to-left, the
//! word is spelled backwards. Vertical shaping is not supported.

use std::fmt::{self, Debug, Formatter};

use fontdock::{FaceId, FaceQuery, FallbackTree, FontVariant};
use ttf_parser::GlyphId;

use crate::font::FontLoader;
use crate::geom::{Point, Size};
use crate::layout::{BoxLayout, Dir, LayoutElement};

/// Shape text into a box containing shaped runs.
pub async fn shape(
    text: &str,
    dir: Dir,
    size: f64,
    variant: FontVariant,
    fallback: &FallbackTree,
    loader: &mut FontLoader,
) -> BoxLayout {
    Shaper::new(text, dir, size, variant, fallback, loader).shape().await
}

/// A shaped run of text.
#[derive(Clone, PartialEq)]
pub struct Shaped {
    /// The shaped text.
    pub text: String,
    /// The font face the text was shaped with.
    pub face: FaceId,
    /// The shaped glyphs.
    pub glyphs: Vec<GlyphId>,
    /// The horizontal offsets of the glyphs. This is indexed parallel to `glyphs`.
    /// Vertical offets are not yet supported.
    pub offsets: Vec<f64>,
    /// The font size.
    pub size: f64,
}

impl Shaped {
    /// Create a new shape run with empty `text`, `glyphs` and `offsets`.
    pub fn new(face: FaceId, size: f64) -> Self {
        Self {
            text: String::new(),
            face,
            glyphs: vec![],
            offsets: vec![],
            size,
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

/// Performs super-basic text shaping.
struct Shaper<'a> {
    text: &'a str,
    dir: Dir,
    variant: FontVariant,
    fallback: &'a FallbackTree,
    loader: &'a mut FontLoader,
    shaped: Shaped,
    layout: BoxLayout,
    offset: f64,
}

impl<'a> Shaper<'a> {
    fn new(
        text: &'a str,
        dir: Dir,
        size: f64,
        variant: FontVariant,
        fallback: &'a FallbackTree,
        loader: &'a mut FontLoader,
    ) -> Self {
        Self {
            text,
            dir,
            variant,
            fallback,
            loader,
            shaped: Shaped::new(FaceId::MAX, size),
            layout: BoxLayout::new(Size::new(0.0, size)),
            offset: 0.0,
        }
    }

    async fn shape(mut self) -> BoxLayout {
        // If the primary axis is negative, we layout the characters reversed.
        if self.dir.is_positive() {
            for c in self.text.chars() {
                self.shape_char(c).await;
            }
        } else {
            for c in self.text.chars().rev() {
                self.shape_char(c).await;
            }
        }

        // Flush the last buffered parts of the word.
        if !self.shaped.text.is_empty() {
            let pos = Point::new(self.offset, 0.0);
            self.layout.push(pos, LayoutElement::Text(self.shaped));
        }

        self.layout
    }

    async fn shape_char(&mut self, c: char) {
        let (index, glyph, char_width) = match self.select_font(c).await {
            Some(selected) => selected,
            // TODO: Issue warning about missing character.
            None => return,
        };

        // Flush the buffer and issue a font setting action if the font differs
        // from the last character's one.
        if self.shaped.face != index {
            if !self.shaped.text.is_empty() {
                let shaped = std::mem::replace(
                    &mut self.shaped,
                    Shaped::new(FaceId::MAX, self.layout.size.height),
                );

                let pos = Point::new(self.offset, 0.0);
                self.layout.push(pos, LayoutElement::Text(shaped));
                self.offset = self.layout.size.width;
            }

            self.shaped.face = index;
        }

        self.shaped.text.push(c);
        self.shaped.glyphs.push(glyph);
        self.shaped.offsets.push(self.layout.size.width - self.offset);

        self.layout.size.width += char_width;
    }

    async fn select_font(&mut self, c: char) -> Option<(FaceId, GlyphId, f64)> {
        let query = FaceQuery {
            fallback: self.fallback.iter(),
            variant: self.variant,
            c,
        };

        if let Some((id, owned_face)) = self.loader.query(query).await {
            let face = owned_face.get();

            let units_per_em = face.units_per_em().unwrap_or(1000) as f64;
            let ratio = 1.0 / units_per_em;
            let font_size = self.layout.size.height;
            let to_raw = |x| ratio * x as f64 * font_size;

            // Determine the width of the char.
            let glyph = face.glyph_index(c)?;
            let glyph_width = to_raw(face.glyph_hor_advance(glyph)? as i32);

            Some((id, glyph, glyph_width))
        } else {
            None
        }
    }
}
