use either::Either;
use typst_library::layout::{Dir, Em};
use typst_library::text::TextElem;
use unicode_bidi::{BidiInfo, Level as BidiLevel};

use super::*;

/// A representation in which children are already layouted and text is already
/// preshaped.
///
/// In many cases, we can directly reuse these results when constructing a line.
/// Only when a line break falls onto a text index that is not safe-to-break per
/// rustybuzz, we have to reshape that portion.
pub struct Preparation<'a> {
    /// The full text.
    pub text: &'a str,
    /// Configuration for inline layout.
    pub config: &'a Config,
    /// Bidirectional text embedding levels.
    ///
    /// This is `None` if all text directions are uniform (all the base
    /// direction).
    pub bidi: Option<BidiInfo<'a>>,
    /// Text runs, spacing and layouted elements.
    pub items: Vec<(Range, Item<'a>)>,
    /// Maps from byte indices to item indices.
    pub indices: Vec<usize>,
    /// The span mapper.
    pub spans: SpanMapper,
}

impl<'a> Preparation<'a> {
    /// Get the item that contains the given `text_offset`.
    pub fn get(&self, offset: usize) -> &(Range, Item<'a>) {
        let idx = self.indices.get(offset).copied().unwrap_or(0);
        &self.items[idx]
    }

    /// Iterate over the items that intersect the given `sliced` range alongside
    /// their indices in `self.items` and their ranges in the paragraph's text.
    pub fn slice(
        &self,
        sliced: Range,
    ) -> impl Iterator<Item = (usize, &(Range, Item<'a>))> {
        // Usually, we don't want empty-range items at the start of the line
        // (because they will be part of the previous line), but for the first
        // line, we need to keep them.
        let start = match sliced.start {
            0 => 0,
            n => self.indices.get(n).copied().unwrap_or(0),
        };
        self.items
            .iter()
            .enumerate()
            .skip(start)
            .take_while(move |(_, (range, _))| {
                range.start < sliced.end || range.end <= sliced.end
            })
    }
}

/// Performs BiDi analysis and then prepares further layout by building a
/// representation on which we can do line breaking without layouting each and
/// every line from scratch.
#[typst_macros::time]
pub fn prepare<'a>(
    engine: &mut Engine,
    config: &'a Config,
    text: &'a str,
    segments: Vec<Segment<'a>>,
    spans: SpanMapper,
) -> SourceResult<Preparation<'a>> {
    let default_level = match config.dir {
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

    if config.cjk_latin_spacing {
        add_cjk_latin_spacing(&mut items);
    }

    Ok(Preparation {
        config,
        text,
        bidi: is_bidi.then_some(bidi),
        items,
        indices,
        spans,
    })
}

/// Add some spacing between Han characters and western characters. See
/// Requirements for Chinese Text Layout, Section 3.2.2 Mixed Text Composition
/// in Horizontal Written Mode
fn add_cjk_latin_spacing(items: &mut [(Range, Item)]) {
    let mut iter = items
        .iter_mut()
        .filter(|(_, item)| !matches!(item, Item::Tag(_)))
        .flat_map(|(_, item)| match item {
            Item::Text(text) => Either::Left({
                // Check whether the text is normal, sub- or superscript. At
                // boundaries between these three, we do not want to insert
                // CJK-Latin-Spacing.
                let shift =
                    text.styles.get_ref(TextElem::shift_settings).map(|shift| shift.kind);

                // Since we only call this function in [`prepare`], we can
                // assume that the Cow is owned, and `to_mut` can be called
                // without overhead.
                text.glyphs.to_mut().iter_mut().map(move |g| Some((g, shift)))
            }),
            _ => Either::Right(std::iter::once(None)),
        })
        .peekable();

    let mut prev: Option<(&mut ShapedGlyph, _)> = None;
    while let Some(mut item) = iter.next() {
        if let Some((glyph, shift)) = &mut item {
            // Case 1: CJ followed by a Latin character
            if glyph.is_cj_script()
                && let Some(Some((next_glyph, next_shift))) = iter.peek()
                && next_glyph.is_letter_or_number()
                && *shift == *next_shift
            {
                // The spacing defaults to 1/4 em, and can be shrunk to 1/8 em.
                glyph.x_advance += Em::new(0.25);
                glyph.adjustability.shrinkability.1 += Em::new(0.125);
            }

            // Case 2: Latin followed by a CJ character
            if glyph.is_cj_script()
                && let Some((prev_glyph, prev_shift)) = prev
                && prev_glyph.is_letter_or_number()
                && *shift == prev_shift
            {
                glyph.x_advance += Em::new(0.25);
                glyph.x_offset += Em::new(0.25);
                glyph.adjustability.shrinkability.0 += Em::new(0.125);
            }
        }
        prev = item;
    }
}
