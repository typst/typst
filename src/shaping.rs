//! Super-basic text shaping.
//!
//! This is really only suited for simple Latin text. It picks the most suitable
//! font for each individual character. When the direction is right-to-left, the
//! word is spelled backwards. Vertical shaping is not supported.

use fontdock::{FaceId, FaceQuery, FallbackTree, FontStyle, FontVariant};
use ttf_parser::GlyphId;

use crate::eval::TextState;
use crate::font::FontLoader;
use crate::geom::{Point, Size};
use crate::layout::{BoxLayout, Dir, LayoutAlign, LayoutElement, LayoutElements, Shaped};

/// Shape text into a box.
pub async fn shape(
    text: &str,
    dir: Dir,
    align: LayoutAlign,
    state: &TextState,
    loader: &mut FontLoader,
) -> BoxLayout {
    Shaper::new(text, dir, align, state, loader).shape().await
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
        align: LayoutAlign,
        state: &'a TextState,
        loader: &'a mut FontLoader,
    ) -> Self {
        let mut variant = state.variant;

        if state.strong {
            variant.weight = variant.weight.thicken(300);
        }

        if state.emph {
            variant.style = match variant.style {
                FontStyle::Normal => FontStyle::Italic,
                FontStyle::Italic => FontStyle::Normal,
                FontStyle::Oblique => FontStyle::Normal,
            }
        }

        Self {
            text,
            dir,
            variant,
            fallback: &state.fallback,
            loader,
            shaped: Shaped::new(FaceId::MAX, state.font_size()),
            layout: BoxLayout {
                size: Size::new(0.0, state.font_size()),
                align,
                elements: LayoutElements::new(),
            },
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
            self.layout.elements.push(pos, LayoutElement::Text(self.shaped));
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
                self.layout.elements.push(pos, LayoutElement::Text(shaped));
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
