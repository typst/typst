//! Layouting of text.

use std::cell::Ref;
use std::mem;

use smallvec::SmallVec;

use crate::doc::TextAction;
use crate::font::{Font, FontQuery};
use super::{Layouter, Layout, LayoutError, LayoutContext, LayoutResult, Size, Position};


/// Layouts text within the constraints of a layouting context.
#[derive(Debug)]
pub struct TextLayouter<'a, 'p> {
    ctx: &'a LayoutContext<'a, 'p>,
    units: Vec<Unit>,
    italic: bool,
    bold: bool,
}

/// A units that is arranged by the text layouter.
#[derive(Debug, Clone)]
enum Unit {
    /// A paragraph.
    Paragraph,
    /// A space with its font index and width.
    Space(usize, Size),
    /// One logical tex  unit.
    Text(TextUnit),
}

/// A logical unit of text (a word, syllable or a similar construct).
#[derive(Debug, Clone)]
struct TextUnit {
    /// Contains pairs of (characters, font_index, char_width) for each character of the text.
    chars_with_widths: SmallVec<[(char, usize, Size); 12]>,
    /// The total width of the unit.
    width: Size,
}

impl<'a, 'p> TextLayouter<'a, 'p> {
    /// Create a new text layouter.
    pub fn new(ctx: &'a LayoutContext<'a, 'p>) -> TextLayouter<'a, 'p> {
        TextLayouter {
            ctx,
            italic: false,
            bold: false,
            units: vec![],
        }
    }

    /// Add more text to the layout.
    pub fn add_text(&mut self, text: &str) -> LayoutResult<()> {
        let mut chars_with_widths = SmallVec::<[(char, usize, Size); 12]>::new();

        // Find out which font to use for each character in the text and meanwhile calculate the
        // width of the text.
        let mut text_width = Size::zero();
        for c in text.chars() {
            // Find out the width and add it to the total width.
            let (index, font) = self.get_font_for(c)?;
            let char_width = self.width_of(c, &font);
            text_width += char_width;

            chars_with_widths.push((c, index, char_width));
        }

        self.units.push(Unit::Text(TextUnit {
            chars_with_widths,
            width: text_width,
        }));

        Ok(())
    }

    /// Add a single space character.
    pub fn add_space(&mut self) -> LayoutResult<()> {
        let (index, font) = self.get_font_for(' ')?;
        let width = self.width_of(' ', &font);
        drop(font);
        Ok(self.units.push(Unit::Space(index, width)))
    }

    /// Start a new paragraph.
    pub fn add_paragraph(&mut self) -> LayoutResult<()> {
        Ok(self.units.push(Unit::Paragraph))
    }

    /// Enable or disable italics.
    pub fn set_italic(&mut self, italic: bool) {
        self.italic = italic;
    }

    /// Enable or disable boldface.
    pub fn set_bold(&mut self, bold: bool) {
        self.bold = bold;
    }

    /// Load a font that has the character we need.
    fn get_font_for(&self, character: char) -> LayoutResult<(usize, Ref<Font>)> {
        self.ctx.loader.get(FontQuery {
            families: self.ctx.text_style.font_families.clone(),
            italic: self.italic,
            bold: self.bold,
            character,
        }).ok_or_else(|| LayoutError::NoSuitableFont)
    }

    /// The width of a char in a specific font.
    fn width_of(&self, character: char, font: &Font) -> Size {
        font.widths[font.map(character) as usize] * self.ctx.text_style.font_size
    }
}

impl Layouter for TextLayouter<'_, '_> {
    fn finish(self) -> LayoutResult<Layout> {
        TextFinisher {
            actions: vec![],
            buffered_text: String::new(),
            current_width: Size::zero(),
            active_font: std::usize::MAX,
            max_width: self.ctx.max_extent.width,
            layouter: self,
        }.finish()
    }
}

/// Finishes a text layout by converting the text units into a stream of text actions.
#[derive(Debug)]
struct TextFinisher<'a, 'p> {
    layouter: TextLayouter<'a, 'p>,
    actions: Vec<TextAction>,
    buffered_text: String,
    current_width: Size,
    active_font: usize,
    max_width: Size,
}

impl<'a, 'p> TextFinisher<'a, 'p> {
    /// Finish the layout.
    fn finish(mut self) -> LayoutResult<Layout> {
        // Move the units out of the layouter leaving an empty vector in place. This is needed to
        // move the units out into the for loop while keeping the borrow checker happy.
        let mut units = Vec::new();
        mem::swap(&mut self.layouter.units, &mut units);

        // Move to the top-left corner of the layout space.
        self.move_start();

        for unit in units {
            match unit {
                Unit::Paragraph => self.write_paragraph(),
                Unit::Space(index, width) => self.write_space(index, width),
                Unit::Text(text) => self.write_text_unit(text),
            }
        }

        self.write_buffered_text();

        Ok(Layout {
            extent: self.layouter.ctx.max_extent.clone(),
            actions: self.actions,
        })
    }

    /// Add a paragraph to the output.
    fn write_paragraph(&mut self) {
        self.write_buffered_text();
        self.move_newline(self.layouter.ctx.text_style.paragraph_spacing);
    }

    /// Add a single space to the output if it is not eaten by a line break.
    fn write_space(&mut self, font: usize, width: Size) {
        if self.would_overflow(width) {
            self.write_buffered_text();
            self.move_newline(1.0);
        } else if self.current_width > Size::zero() {
            if font != self.active_font {
                self.write_buffered_text();
                self.set_font(font);
            }

            self.buffered_text.push(' ');
            self.current_width += width;
        }
    }

    /// Add a single unit of text without breaking it apart.
    fn write_text_unit(&mut self, text: TextUnit) {
        if self.would_overflow(text.width) {
            self.write_buffered_text();
            self.move_newline(1.0);
        }

        // Finally write the word.
        for (c, font, width) in text.chars_with_widths {
            if font != self.active_font {
                // If we will change the font, first write the remaining things.
                self.write_buffered_text();
                self.set_font(font);
            }

            self.buffered_text.push(c);
            self.current_width += width;
        }
    }

    /// Move to the top-left corner of the layout space.
    fn move_start(&mut self) {
        self.actions.push(TextAction::MoveNewline(Position {
            x: Size::zero(),
            y: self.layouter.ctx.max_extent.height
                - Size::from_points(self.layouter.ctx.text_style.font_size)
        }));
    }

    /// Move to the next line. A factor of 1.0 uses the default line spacing.
    fn move_newline(&mut self, factor: f32) {
        if self.active_font != std::usize::MAX {
            let vertical = Size::from_points(self.layouter.ctx.text_style.font_size)
                * self.layouter.ctx.text_style.line_spacing
                * factor;

            self.actions.push(TextAction::MoveNewline(Position {
                x: Size::zero(),
                y: -vertical
            }));

            self.current_width = Size::zero();
        }
    }

    /// Output a text action containing the buffered text and reset the buffer.
    fn write_buffered_text(&mut self) {
        if !self.buffered_text.is_empty() {
            let mut buffered = String::new();
            mem::swap(&mut self.buffered_text, &mut buffered);
            self.actions.push(TextAction::WriteText(buffered));
        }
    }

    /// Output an action setting a new font and update the active font.
    fn set_font(&mut self, index: usize) {
        self.active_font = index;
        self.actions.push(TextAction::SetFont(index, self.layouter.ctx.text_style.font_size));
    }

    /// Check whether additional text with the given width would overflow the current line.
    fn would_overflow(&self, width: Size) -> bool {
        self.current_width + width > self.max_width
    }
}
