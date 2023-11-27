use std::fmt::{self, Debug, Formatter};
use std::ops::Range;

use ecow::EcoString;

use crate::layout::{Abs, Em};
use crate::syntax::Span;
use crate::text::{Font, Lang};
use crate::visualize::Paint;

/// A run of shaped text.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct TextItem {
    /// The font the glyphs are contained in.
    pub font: Font,
    /// The font size.
    pub size: Abs,
    /// Glyph color.
    pub fill: Paint,
    /// The natural language of the text.
    pub lang: Lang,
    /// The item's plain text.
    pub text: EcoString,
    /// The glyphs.
    pub glyphs: Vec<Glyph>,
}

impl TextItem {
    /// The width of the text run.
    pub fn width(&self) -> Abs {
        self.glyphs.iter().map(|g| g.x_advance).sum::<Em>().at(self.size)
    }
}

impl Debug for TextItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Text(")?;
        self.text.fmt(f)?;
        f.write_str(")")
    }
}

/// A glyph in a run of shaped text.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Glyph {
    /// The glyph's index in the font.
    pub id: u16,
    /// The advance width of the glyph.
    pub x_advance: Em,
    /// The horizontal offset of the glyph.
    pub x_offset: Em,
    /// The range of the glyph in its item's text.
    pub range: Range<u16>,
    /// The source code location of the text.
    pub span: (Span, u16),
}

impl Glyph {
    /// The range of the glyph in its item's text.
    pub fn range(&self) -> Range<usize> {
        usize::from(self.range.start)..usize::from(self.range.end)
    }
}
