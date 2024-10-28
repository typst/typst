use typst_library::foundations::{Resolve, Smart};
use typst_library::layout::{Abs, AlignElem, Dir, Em, FixedAlignment};
use typst_library::model::Linebreaks;
use typst_library::text::{Costs, Lang, TextElem};
use unicode_bidi::{BidiInfo, Level as BidiLevel};

use super::*;

/// A paragraph representation in which children are already layouted and text
/// is already preshaped.
///
/// In many cases, we can directly reuse these results when constructing a line.
/// Only when a line break falls onto a text index that is not safe-to-break per
/// rustybuzz, we have to reshape that portion.
pub struct Preparation<'a> {
    /// The paragraph's full text.
    pub text: &'a str,
    /// Bidirectional text embedding levels for the paragraph.
    ///
    /// This is `None` if the paragraph is BiDi-uniform (all the base direction).
    pub bidi: Option<BidiInfo<'a>>,
    /// Text runs, spacing and layouted elements.
    pub items: Vec<(Range, Item<'a>)>,
    /// Maps from byte indices to item indices.
    pub indices: Vec<usize>,
    /// The span mapper.
    pub spans: SpanMapper,
    /// Whether to hyphenate if it's the same for all children.
    pub hyphenate: Option<bool>,
    /// Costs for various layout decisions.
    pub costs: Costs,
    /// The dominant direction.
    pub dir: Dir,
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
    /// How to determine line breaks.
    pub linebreaks: Smart<Linebreaks>,
    /// The text size.
    pub size: Abs,
}

impl<'a> Preparation<'a> {
    /// Get the item that contains the given `text_offset`.
    pub fn get(&self, offset: usize) -> &(Range, Item<'a>) {
        let idx = self.indices.get(offset).copied().unwrap_or(0);
        &self.items[idx]
    }

    /// Iterate over the items that intersect the given `sliced` range.
    pub fn slice(&self, sliced: Range) -> impl Iterator<Item = &(Range, Item<'a>)> {
        // Usually, we don't want empty-range items at the start of the line
        // (because they will be part of the previous line), but for the first
        // line, we need to keep them.
        let start = match sliced.start {
            0 => 0,
            n => self.indices.get(n).copied().unwrap_or(0),
        };
        self.items[start..].iter().take_while(move |(range, _)| {
            range.start < sliced.end || range.end <= sliced.end
        })
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
    let dir = TextElem::dir_in(styles);
    let default_level = match dir {
        Dir::RTL => BidiLevel::rtl(),
        _ => BidiLevel::ltr(),
    };

    let bidi = BidiInfo::new(text, Some(default_level));
    let is_bidi = bidi
        .levels
        .iter()
        .any(|level| level.is_ltr() != default_level.is_ltr());

    let mut cursor = 0;
    let mut items = Vec::with_capacity(segments.len());

    // Shape the text to finalize the items.
    for segment in segments {
        let len = segment.textual_len();
        let end = cursor + len;
        let range = cursor..end;

        match segment {
            Segment::Text(_, styles) => {
                shape_range(&mut items, engine, text, &bidi, range, styles);
            }
            Segment::Item(item) => items.push((range, item)),
        }

        cursor = end;
    }

    // Build the mapping from byte to item indices.
    let mut indices = Vec::with_capacity(text.len());
    for (i, (range, _)) in items.iter().enumerate() {
        indices.extend(range.clone().map(|_| i));
    }

    let cjk_latin_spacing = TextElem::cjk_latin_spacing_in(styles).is_auto();
    if cjk_latin_spacing {
        add_cjk_latin_spacing(&mut items);
    }

    Ok(Preparation {
        text,
        bidi: is_bidi.then_some(bidi),
        items,
        indices,
        spans,
        hyphenate: children.shared_get(styles, TextElem::hyphenate_in),
        costs: TextElem::costs_in(styles),
        dir,
        lang: children.shared_get(styles, TextElem::lang_in),
        align: AlignElem::alignment_in(styles).resolve(styles).x,
        justify: ParElem::justify_in(styles),
        hang: ParElem::hanging_indent_in(styles),
        cjk_latin_spacing,
        fallback: TextElem::fallback_in(styles),
        linebreaks: ParElem::linebreaks_in(styles),
        size: TextElem::size_in(styles),
    })
}

/// Add some spacing between Han characters and western characters. See
/// Requirements for Chinese Text Layout, Section 3.2.2 Mixed Text Composition
/// in Horizontal Written Mode
fn add_cjk_latin_spacing(items: &mut [(Range, Item)]) {
    let mut items = items
        .iter_mut()
        .filter(|(_, x)| !matches!(x, Item::Tag(_)))
        .peekable();

    let mut prev: Option<&ShapedGlyph> = None;
    while let Some((_, item)) = items.next() {
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
                    .and_then(|(_, i)| i.text())
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
