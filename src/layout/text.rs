use toddle::query::{SharedFontLoader, FontQuery, FontIndex};
use toddle::tables::{CharMap, Header, HorizontalMetrics};

use crate::size::{Size, Size2D};
use crate::style::TextStyle;
use super::*;

/// The context for text layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Copy, Clone)]
pub struct TextContext<'a, 'p> {
    pub loader: &'a SharedFontLoader<'p>,
    pub style: &'a TextStyle,
    pub axes: LayoutAxes,
    pub alignment: LayoutAlignment,
}

/// Layouts text into a box.
///
/// There is no complex layout involved. The text is simply laid out left-
/// to-right using the correct font for each character.
pub fn layout_text(text: &str, ctx: TextContext) -> LayoutResult<Layout> {
    TextLayouter::new(text, ctx).layout()
}

/// Layouts text into boxes.
struct TextLayouter<'a, 'p> {
    ctx: TextContext<'a, 'p>,
    text: &'a str,
    actions: LayoutActions,
    buffer: String,
    active_font: FontIndex,
    width: Size,
}

impl<'a, 'p> TextLayouter<'a, 'p> {
    /// Create a new text layouter.
    fn new(text: &'a str, ctx: TextContext<'a, 'p>) -> TextLayouter<'a, 'p> {
        TextLayouter {
            ctx,
            text,
            actions: LayoutActions::new(),
            buffer: String::new(),
            active_font: FontIndex::MAX,
            width: Size::ZERO,
        }
    }

    /// Layout the text
    fn layout(mut self) -> LayoutResult<Layout> {
        if self.ctx.axes.primary.is_positive() {
            for c in self.text.chars() {
                self.layout_char(c)?;
            }
        } else {
            for c in self.text.chars().rev() {
                self.layout_char(c)?;
            }
        }

        if !self.buffer.is_empty() {
            self.actions.add(LayoutAction::WriteText(self.buffer));
        }

        Ok(Layout {
            dimensions: Size2D::new(self.width, self.ctx.style.font_size()),
            alignment: self.ctx.alignment,
            actions: self.actions.to_vec(),
        })
    }

    /// Layout an individual character.
    fn layout_char(&mut self, c: char) -> LayoutResult<()> {
        let (index, char_width) = self.select_font(c)?;

        self.width += char_width;

        if self.active_font != index {
            if !self.buffer.is_empty() {
                let text = std::mem::replace(&mut self.buffer, String::new());
                self.actions.add(LayoutAction::WriteText(text));
            }

            self.actions.add(LayoutAction::SetFont(index, self.ctx.style.font_size()));
            self.active_font = index;
        }

        self.buffer.push(c);

        Ok(())
    }

    /// Select the best font for a character and return its index along with
    /// the width of the char in the font.
    fn select_font(&mut self, c: char) -> LayoutResult<(FontIndex, Size)> {
        let mut loader = self.ctx.loader.borrow_mut();

        let query = FontQuery {
            fallback: &self.ctx.style.fallback,
            variant: self.ctx.style.variant,
            c,
        };

        if let Some((font, index)) = loader.get(query) {
            let font_unit_ratio = 1.0 / (font.read_table::<Header>()?.units_per_em as f32);
            let font_unit_to_size = |x| Size::pt(font_unit_ratio * x);

            let glyph = font
                .read_table::<CharMap>()?
                .get(c)
                .expect("select_font: font should have char");

            let glyph_width = font
                .read_table::<HorizontalMetrics>()?
                .get(glyph)
                .expect("select_font: font should have glyph")
                .advance_width as f32;

            let char_width = font_unit_to_size(glyph_width)
                * self.ctx.style.font_size().to_pt();

            return Ok((index, char_width));
        }

        error!("no suitable font for character `{}`", c);
    }
}
