//! The text layouter layouts continous pieces of text into boxes.
//!
//! The layouter picks the most suitable font for each individual character.
//! When the primary layouting axis horizontally inversed, the word is spelled
//! backwards. Vertical word layout is not yet supported.

use toddle::query::{FontQuery, FontIndex};
use toddle::tables::{CharMap, Header, HorizontalMetrics};

use crate::GlobalFontLoader;
use crate::size::{Size, Size2D};
use crate::style::TextStyle;
use super::*;


/// Performs the text layouting.
#[derive(Debug)]
struct TextLayouter<'a> {
    ctx: TextContext<'a>,
    text: &'a str,
    actions: LayoutActions,
    buffer: String,
    active_font: FontIndex,
    width: Size,
}

/// The context for text layouting.
#[derive(Debug, Copy, Clone)]
pub struct TextContext<'a> {
    /// The font loader to retrieve fonts from when typesetting text
    /// using [`layout_text`].
    pub loader: &'a GlobalFontLoader,
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
            active_font: FontIndex::MAX,
            width: Size::ZERO,
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
            dimensions: Size2D::new(self.width, self.ctx.style.font_size()),
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
    async fn select_font(&mut self, c: char) -> Option<(FontIndex, Size)> {
        let mut loader = self.ctx.loader.borrow_mut();

        let mut variant = self.ctx.style.variant;
        if self.ctx.style.bolder {
            variant.weight.0 += 300;
        }

        let queried = if self.ctx.style.monospace {
            loader.get(FontQuery {
                // FIXME: This is a hack.
                fallback: std::iter::once("source code pro")
                    .chain(self.ctx.style.fallback.iter()),
                variant,
                c,
            }).await
        } else {
            loader.get(FontQuery {
                fallback: self.ctx.style.fallback.iter(),
                variant,
                c,
            }).await
        };

        if let Some((font, index)) = queried {
            // Determine the width of the char.
            let header = font.read_table::<Header>().ok()?;
            let font_unit_ratio = 1.0 / (header.units_per_em as f32);
            let font_unit_to_size = |x| Size::pt(font_unit_ratio * x);

            let glyph = font
                .read_table::<CharMap>()
                .ok()?
                .get(c)?;

            let glyph_width = font
                .read_table::<HorizontalMetrics>()
                .ok()?
                .get(glyph)?
                .advance_width as f32;

            let char_width = font_unit_to_size(glyph_width)
                * self.ctx.style.font_size().to_pt();

            Some((index, char_width))
        } else {
            None
        }
    }
}
