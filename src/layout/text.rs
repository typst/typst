//! The text layouter layouts continous pieces of text into boxes.
//!
//! The layouter picks the most suitable font for each individual character.
//! When the primary layouting axis horizontally inversed, the word is spelled
//! backwards. Vertical word layout is not yet supported.

use fontdock::{FaceId, FaceQuery, FontStyle};
use crate::font::SharedFontLoader;
use crate::geom::Size;
use crate::style::TextStyle;
use super::*;

/// Performs the text layouting.
#[derive(Debug)]
struct TextLayouter<'a> {
    ctx: TextContext<'a>,
    text: &'a str,
    actions: LayoutActions,
    buffer: String,
    active_font: FaceId,
    width: f64,
}

/// The context for text layouting.
#[derive(Debug, Copy, Clone)]
pub struct TextContext<'a> {
    /// The font loader to retrieve fonts from when typesetting text
    /// using [`layout_text`].
    pub loader: &'a SharedFontLoader,
    /// The style for text: Font selection with classes, weights and variants,
    /// font sizes, spacing and so on.
    pub style: &'a TextStyle,
    /// The axes along which the word is laid out. For now, only
    /// primary-horizontal layouting is supported.
    pub axes: LayoutAxes,
    /// The alignment of the finished layout.
    pub alignment: LayoutAlignment,
}

/// Layouts text into a box.
pub async fn layout_text(text: &str, ctx: TextContext<'_>) -> Layout {
    TextLayouter::new(text, ctx).layout().await
}

impl<'a> TextLayouter<'a> {
    /// Create a new text layouter.
    fn new(text: &'a str, ctx: TextContext<'a>) -> TextLayouter<'a> {
        TextLayouter {
            ctx,
            text,
            actions: LayoutActions::new(),
            buffer: String::new(),
            active_font: FaceId::MAX,
            width: 0.0,
        }
    }

    /// Do the layouting.
    async fn layout(mut self) -> Layout {
        // If the primary axis is negative, we layout the characters reversed.
        if self.ctx.axes.primary.is_positive() {
            for c in self.text.chars() {
                self.layout_char(c).await;
            }
        } else {
            for c in self.text.chars().rev() {
                self.layout_char(c).await;
            }
        }

        // Flush the last buffered parts of the word.
        if !self.buffer.is_empty() {
            self.actions.add(LayoutAction::WriteText(self.buffer));
        }

        Layout {
            dimensions: Size::new(self.width, self.ctx.style.font_size()),
            alignment: self.ctx.alignment,
            actions: self.actions.into_vec(),
        }
    }

    /// Layout an individual character.
    async fn layout_char(&mut self, c: char) {
        let (index, char_width) = match self.select_font(c).await {
            Some(selected) => selected,
            // TODO: Issue warning about missing character.
            None => return,
        };

        self.width += char_width;

        // Flush the buffer and issue a font setting action if the font differs
        // from the last character's one.
        if self.active_font != index {
            if !self.buffer.is_empty() {
                let text = std::mem::replace(&mut self.buffer, String::new());
                self.actions.add(LayoutAction::WriteText(text));
            }

            self.actions.add(LayoutAction::SetFont(index, self.ctx.style.font_size()));
            self.active_font = index;
        }

        self.buffer.push(c);
    }

    /// Select the best font for a character and return its index along with
    /// the width of the char in the font.
    async fn select_font(&mut self, c: char) -> Option<(FaceId, f64)> {
        let mut loader = self.ctx.loader.borrow_mut();

        let mut variant = self.ctx.style.variant;

        if self.ctx.style.bolder {
            variant.weight.0 += 300;
        }

        if self.ctx.style.italic {
            variant.style = match variant.style {
                FontStyle::Normal => FontStyle::Italic,
                FontStyle::Italic => FontStyle::Normal,
                FontStyle::Oblique => FontStyle::Normal,
            }
        }

        let query = FaceQuery {
            fallback: self.ctx.style.fallback.iter(),
            variant,
            c,
        };

        if let Some((id, face)) = loader.query(query).await {
            // Determine the width of the char.
            let units_per_em = face.units_per_em().unwrap_or(1000) as f64;
            let ratio = 1.0 / units_per_em;
            let to_raw = |x| ratio * x as f64;

            let glyph = face.glyph_index(c)?;
            let glyph_width = face.glyph_hor_advance(glyph)?;
            let char_width = to_raw(glyph_width) * self.ctx.style.font_size();

            Some((id, char_width))
        } else {
            None
        }
    }
}
