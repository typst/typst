use unicode_bidi::{BidiInfo, Level as BidiLevel};

use super::*;
use crate::foundations::{Resolve, Smart};
use crate::layout::{Abs, AlignElem, Dir, Em, FixedAlignment};
use crate::model::Linebreaks;
use crate::text::{Costs, Lang, TextElem};

/// A paragraph representation in which children are already layouted and text
/// is already preshaped.
///
/// In many cases, we can directly reuse these results when constructing a line.
/// Only when a line break falls onto a text index that is not safe-to-break per
/// rustybuzz, we have to reshape that portion.
pub struct Preparation<'a> {
    /// Bidirectional text embedding levels for the paragraph.
    pub bidi: BidiInfo<'a>,
    /// Text runs, spacing and layouted elements.
    pub items: Vec<Item<'a>>,
    /// The span mapper.
    pub spans: SpanMapper,
    /// Whether to hyphenate if it's the same for all children.
    pub hyphenate: Option<bool>,
    /// Costs for various layout decisions.
    pub costs: Costs,
    /// The text language if it's the same for all children.
    pub lang: Option<Lang>,
    /// The paragraph's resolved horizontal alignment.
    pub align: FixedAlignment,
    /// Whether to justify the paragraph.
    pub justify: bool,
    /// The paragraph's hanging indent.
    pub hang: Abs,
    /// Whether to add spacing between CJK and Latin characters.
    pub cjk_latin_spacing: bool,
    /// Whether font fallback is enabled for this paragraph.
    pub fallback: bool,
    /// The leading of the paragraph.
    pub leading: Abs,
    /// How to determine line breaks.
    pub linebreaks: Smart<Linebreaks>,
    /// The text size.
    pub size: Abs,
}

impl<'a> Preparation<'a> {
    /// Find the item that contains the given `text_offset`.
    pub fn find(&self, text_offset: usize) -> Option<&Item<'a>> {
        let mut cursor = 0;
        for item in &self.items {
            let end = cursor + item.textual_len();
            if (cursor..end).contains(&text_offset) {
                return Some(item);
            }
            cursor = end;
        }
        None
    }

    /// Return the items that intersect the given `text_range`.
    ///
    /// Returns the expanded range around the items and the items.
    pub fn slice(&self, text_range: Range) -> (Range, &[Item<'a>]) {
        let mut cursor = 0;
        let mut start = 0;
        let mut end = 0;
        let mut expanded = text_range.clone();

        for (i, item) in self.items.iter().enumerate() {
            if cursor <= text_range.start {
                start = i;
                expanded.start = cursor;
            }

            let len = item.textual_len();
            if cursor < text_range.end || cursor + len <= text_range.end {
                end = i + 1;
                expanded.end = cursor + len;
            } else {
                break;
            }

            cursor += len;
        }

        (expanded, &self.items[start..end])
    }
}

/// Performs BiDi analysis and then prepares paragraph layout by building a
/// representation on which we can do line breaking without layouting each and
/// every line from scratch.
#[typst_macros::time]
pub fn prepare<'a>(
    engine: &mut Engine,
    children: &'a StyleVec,
    text: &'a str,
    segments: Vec<Segment<'a>>,
    spans: SpanMapper,
    styles: StyleChain<'a>,
) -> SourceResult<Preparation<'a>> {
    let bidi = BidiInfo::new(
        text,
        match TextElem::dir_in(styles) {
            Dir::LTR => Some(BidiLevel::ltr()),
            Dir::RTL => Some(BidiLevel::rtl()),
            _ => None,
        },
    );

    let mut cursor = 0;
    let mut items = Vec::with_capacity(segments.len());

    // Shape the text to finalize the items.
    for segment in segments {
        let end = cursor + segment.textual_len();
        match segment {
            Segment::Text(_, styles) => {
                shape_range(&mut items, engine, &bidi, cursor..end, &spans, styles);
            }
            Segment::Item(item) => items.push(item),
        }

        cursor = end;
    }

    let cjk_latin_spacing = TextElem::cjk_latin_spacing_in(styles).is_auto();
    if cjk_latin_spacing {
        add_cjk_latin_spacing(&mut items);
    }

    Ok(Preparation {
        bidi,
        items,
        spans,
        hyphenate: children.shared_get(styles, TextElem::hyphenate_in),
        costs: TextElem::costs_in(styles),
        lang: children.shared_get(styles, TextElem::lang_in),
        align: AlignElem::alignment_in(styles).resolve(styles).x,
        justify: ParElem::justify_in(styles),
        hang: ParElem::hanging_indent_in(styles),
        cjk_latin_spacing,
        fallback: TextElem::fallback_in(styles),
        leading: ParElem::leading_in(styles),
        linebreaks: ParElem::linebreaks_in(styles),
        size: TextElem::size_in(styles),
    })
}

/// Add some spacing between Han characters and western characters. See
/// Requirements for Chinese Text Layout, Section 3.2.2 Mixed Text Composition
/// in Horizontal Written Mode
fn add_cjk_latin_spacing(items: &mut [Item]) {
    let mut items = items.iter_mut().filter(|x| !matches!(x, Item::Tag(_))).peekable();
    let mut prev: Option<&ShapedGlyph> = None;
    while let Some(item) = items.next() {
        let Some(text) = item.text_mut() else {
            prev = None;
            continue;
        };

        // Since we only call this function in [`prepare`], we can assume that
        // the Cow is owned, and `to_mut` can be called without overhead.
        debug_assert!(matches!(text.glyphs, std::borrow::Cow::Owned(_)));
        let mut glyphs = text.glyphs.to_mut().iter_mut().peekable();

        while let Some(glyph) = glyphs.next() {
            let next = glyphs.peek().map(|n| n as _).or_else(|| {
                items
                    .peek()
                    .and_then(|i| i.text())
                    .and_then(|shaped| shaped.glyphs.first())
            });

            // Case 1: CJ followed by a Latin character
            if glyph.is_cj_script() && next.is_some_and(|g| g.is_letter_or_number()) {
                // The spacing is default to 1/4 em, and can be shrunk to 1/8 em.
                glyph.x_advance += Em::new(0.25);
                glyph.adjustability.shrinkability.1 += Em::new(0.125);
                text.width += Em::new(0.25).at(text.size);
            }

            // Case 2: Latin followed by a CJ character
            if glyph.is_cj_script() && prev.is_some_and(|g| g.is_letter_or_number()) {
                glyph.x_advance += Em::new(0.25);
                glyph.x_offset += Em::new(0.25);
                glyph.adjustability.shrinkability.0 += Em::new(0.125);
                text.width += Em::new(0.25).at(text.size);
            }

            prev = Some(glyph);
        }
    }
}
