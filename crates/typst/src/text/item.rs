use std::fmt::{self, Debug, Formatter};
use std::ops::Range;

use ecow::EcoString;

use crate::layout::{Abs, Em};
use crate::syntax::Span;
use crate::text::{is_default_ignorable, Font, Lang, Region};
use crate::visualize::{FixedStroke, Paint};

/// A run of shaped text.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct TextItem {
    /// The font the glyphs are contained in.
    pub font: Font,
    /// The font size.
    pub size: Abs,
    /// Glyph color.
    pub fill: Paint,
    /// Glyph stroke.
    pub stroke: Option<FixedStroke>,
    /// The natural language of the text.
    pub lang: Lang,
    /// The region of the text.
    pub region: Option<Region>,
    /// The item's plain text.
    pub text: EcoString,
    /// The glyphs. The number of glyphs may be different from the number of
    /// characters in the plain text due to e.g. ligatures.
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
    /// The range of the glyph in its item's text. The range's length may
    /// be more than one due to multi-byte UTF-8 encoding or ligatures.
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

/// A slice of a [`TextItem`].
pub struct TextItemView<'a> {
    /// The whole item this is a part of
    pub item: &'a TextItem,
    /// The glyphs of this slice
    pub glyph_range: Range<usize>,
}

impl<'a> TextItemView<'a> {
    /// Build a TextItemView for the whole contents of a TextItem.
    pub fn full(text: &'a TextItem) -> Self {
        Self::from_glyph_range(text, 0..text.glyphs.len())
    }

    /// Build a new [`TextItemView`] from a [`TextItem`] and a range of glyphs.
    pub fn from_glyph_range(text: &'a TextItem, glyph_range: Range<usize>) -> Self {
        TextItemView { item: text, glyph_range }
    }

    /// Returns an iterator over the glyphs of the slice.
    ///
    /// The range of text that each glyph represents is remapped to be relative
    /// to the start of the slice.
    pub fn glyphs(&self) -> impl Iterator<Item = Glyph> + '_ {
        let first = self.item.glyphs[self.glyph_range.start].range();
        let last = self.item.glyphs[self.glyph_range.end - 1].range();
        let base = first.start.min(last.start) as u16;
        (0..self.glyph_range.len()).map(move |index| {
            let g = &self.item.glyphs[self.glyph_range.start + index];
            Glyph {
                range: g.range.start - base..g.range.end - base,
                ..*g
            }
        })
    }

    /// The plain text for the given glyph. This is an approximation since
    /// glyphs do not correspond 1-1 with codepoints.
    pub fn glyph_text(&self, glyph: &Glyph) -> EcoString {
        self.item.text[glyph.range()]
            .chars()
            .filter(|&c| !is_default_ignorable(c))
            .collect()
    }

    /// The total width of this text slice
    pub fn width(&self) -> Abs {
        self.item.glyphs[self.glyph_range.clone()]
            .iter()
            .map(|g| g.x_advance)
            .sum::<Em>()
            .at(self.item.size)
    }
}
