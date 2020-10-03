//! Super-basic text shaping.
//!
//! The layouter picks the most suitable font for each individual character. When the
//! direction is right-to-left, the word is spelled backwards. Vertical shaping is not yet
//! supported.

use fontdock::{FaceId, FaceQuery, FontStyle};
use ttf_parser::GlyphId;

use super::elements::{LayoutElement, Shaped};
use super::BoxLayout as Layout;
use super::*;
use crate::font::FontLoader;
use crate::geom::Size;
use crate::style::TextStyle;

/// Layouts text into a box.
pub async fn shape(text: &str, ctx: ShapeOptions<'_>) -> BoxLayout {
    Shaper::new(text, ctx).layout().await
}

/// Options for text shaping.
#[derive(Debug)]
pub struct ShapeOptions<'a> {
    /// The font loader to retrieve fonts from.
    pub loader: &'a mut FontLoader,
    /// The style for text: Font selection with classes, weights and variants,
    /// font sizes, spacing and so on.
    pub style: &'a TextStyle,
    /// The direction into which the text is laid out. Currently, only horizontal
    /// directions are supported.
    pub dir: Dir,
    /// The alignment of the _resulting_ layout. This does not effect the line
    /// layouting itself, but rather how the finished layout will be positioned
    /// in a parent layout.
    pub align: LayoutAlign,
}

/// Performs super-basic text shaping.
struct Shaper<'a> {
    opts: ShapeOptions<'a>,
    text: &'a str,
    shaped: Shaped,
    layout: Layout,
    offset: f64,
}

impl<'a> Shaper<'a> {
    fn new(text: &'a str, opts: ShapeOptions<'a>) -> Self {
        Self {
            text,
            shaped: Shaped::new(FaceId::MAX, opts.style.font_size()),
            layout: BoxLayout {
                size: Size::new(0.0, opts.style.font_size()),
                align: opts.align,
                elements: LayoutElements::new(),
            },
            offset: 0.0,
            opts,
        }
    }

    async fn layout(mut self) -> Layout {
        // If the primary axis is negative, we layout the characters reversed.
        if self.opts.dir.is_positive() {
            for c in self.text.chars() {
                self.layout_char(c).await;
            }
        } else {
            for c in self.text.chars().rev() {
                self.layout_char(c).await;
            }
        }

        // Flush the last buffered parts of the word.
        if !self.shaped.text.is_empty() {
            let pos = Size::new(self.offset, 0.0);
            self.layout.elements.push(pos, LayoutElement::Text(self.shaped));
        }

        self.layout
    }

    async fn layout_char(&mut self, c: char) {
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
                    Shaped::new(FaceId::MAX, self.opts.style.font_size()),
                );

                let pos = Size::new(self.offset, 0.0);
                self.layout.elements.push(pos, LayoutElement::Text(shaped));
                self.offset = self.layout.size.x;
            }

            self.shaped.face = index;
        }

        self.shaped.text.push(c);
        self.shaped.glyphs.push(glyph);
        self.shaped.offsets.push(self.layout.size.x - self.offset);

        self.layout.size.x += char_width;
    }

    async fn select_font(&mut self, c: char) -> Option<(FaceId, GlyphId, f64)> {
        let mut variant = self.opts.style.variant;

        if self.opts.style.bolder {
            variant.weight = variant.weight.thicken(300);
        }

        if self.opts.style.italic {
            variant.style = match variant.style {
                FontStyle::Normal => FontStyle::Italic,
                FontStyle::Italic => FontStyle::Normal,
                FontStyle::Oblique => FontStyle::Normal,
            }
        }

        let query = FaceQuery {
            fallback: self.opts.style.fallback.iter(),
            variant,
            c,
        };

        if let Some((id, owned_face)) = self.opts.loader.query(query).await {
            let face = owned_face.get();
            let font_size = self.opts.style.font_size();

            let units_per_em = face.units_per_em().unwrap_or(1000) as f64;
            let ratio = 1.0 / units_per_em;
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
