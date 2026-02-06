use typst_library::diag::warning;
use typst_library::foundations::{Packed, Resolve};
use typst_library::introspection::{SplitLocator, Tag, TagElem};
use typst_library::layout::{
    Abs, BoxElem, Dir, Fr, Frame, HElem, InlineElem, InlineItem, Sizing, Spacing,
};
use typst_library::routines::Pair;
use typst_library::text::{
    LinebreakElem, SmartQuoteElem, SmartQuoter, SmartQuotes, SpaceElem, TextElem,
    is_default_ignorable,
};
use typst_syntax::Span;
use typst_utils::Numeric;

use super::*;
use crate::modifiers::{FrameModifiers, FrameModify, layout_and_modify};

// The characters by which spacing, inline content and pins are replaced in the
// full text.
const SPACING_REPLACE: &str = " "; // Space
const OBJ_REPLACE: &str = "\u{FFFC}"; // Object Replacement Character

// Unicode BiDi control characters.
const LTR_EMBEDDING: &str = "\u{202A}";
const RTL_EMBEDDING: &str = "\u{202B}";
const POP_EMBEDDING: &str = "\u{202C}";
const LTR_ISOLATE: &str = "\u{2066}";
const POP_ISOLATE: &str = "\u{2069}";

/// A prepared item in a inline layout.
#[derive(Debug)]
pub enum Item<'a> {
    /// A shaped text run with consistent style and direction.
    Text(ShapedText<'a>),
    /// Absolute spacing between other items, and whether it is weak.
    Absolute(Abs, bool),
    /// Fractional spacing between other items.
    Fractional(Fr, Option<(&'a Packed<BoxElem>, Locator<'a>, StyleChain<'a>)>),
    /// Layouted inline-level content.
    Frame(Frame),
    /// A tag.
    Tag(&'a Tag),
    /// An item that is invisible and needs to be skipped, e.g. a Unicode
    /// isolate.
    Skip(&'static str),
}

impl<'a> Item<'a> {
    /// Whether this is a tag item.
    pub fn is_tag(&self) -> bool {
        matches!(self, Self::Tag(_))
    }

    /// If this a text item, return it.
    pub fn text(&self) -> Option<&ShapedText<'a>> {
        match self {
            Self::Text(shaped) => Some(shaped),
            _ => None,
        }
    }

    /// If this a text item, return it mutably.
    pub fn text_mut(&mut self) -> Option<&mut ShapedText<'a>> {
        match self {
            Self::Text(shaped) => Some(shaped),
            _ => None,
        }
    }

    /// Return the textual representation of this item: Either just itself (for
    /// a text item) or a replacement string (for any other item).
    pub fn textual(&self) -> &str {
        match self {
            Self::Text(shaped) => shaped.text,
            Self::Absolute(_, _) | Self::Fractional(_, _) => SPACING_REPLACE,
            Self::Frame(_) => OBJ_REPLACE,
            Self::Tag(_) => "",
            Self::Skip(s) => s,
        }
    }

    /// The text length of the item.
    pub fn textual_len(&self) -> usize {
        self.textual().len()
    }

    /// The natural layouted width of the item.
    pub fn natural_width(&self) -> Abs {
        match self {
            Self::Text(shaped) => shaped.width(),
            Self::Absolute(v, _) => *v,
            Self::Frame(frame) => frame.width(),
            Self::Fractional(_, _) | Self::Tag(_) => Abs::zero(),
            Self::Skip(_) => Abs::zero(),
        }
    }
}

/// An item or not-yet shaped text. We can't shape text until we have collected
/// all items because only then we can compute BiDi, and we need to split shape
/// runs at level boundaries.
#[derive(Debug)]
pub enum Segment<'a> {
    /// One or multiple collapsed text children. Stores how long the segment is
    /// (in bytes of the full text string).
    Text(usize, StyleChain<'a>),
    /// An already prepared item.
    Item(Item<'a>),
}

impl Segment<'_> {
    /// The text length of the item.
    pub fn textual_len(&self) -> usize {
        match self {
            Self::Text(len, _) => *len,
            Self::Item(item) => item.textual_len(),
        }
    }
}

/// Collects all text into one string and a collection of segments that
/// correspond to pieces of that string. This also performs string-level
/// preprocessing like case transformations.
#[typst_macros::time]
pub fn collect<'a>(
    children: &[Pair<'a>],
    engine: &mut Engine<'_>,
    locator: &mut SplitLocator<'a>,
    config: &Config,
    region: Size,
) -> SourceResult<(String, Vec<Segment<'a>>, SpanMapper)> {
    let mut collector = Collector::new(2 + children.len());
    let mut quoter = SmartQuoter::new();

    if !config.first_line_indent.is_zero() {
        collector.push_item(Item::Absolute(config.first_line_indent, false));
        collector.spans.push(1, Span::detached());
    }

    if !config.hanging_indent.is_zero() {
        collector.push_item(Item::Absolute(-config.hanging_indent, false));
        collector.spans.push(1, Span::detached());
    }

    for &(child, styles) in children {
        let prev_len = collector.full.len();

        if child.is::<SpaceElem>() {
            collector.push_text(" ", styles);
        } else if let Some(elem) = child.to_packed::<TextElem>() {
            collector.build_text(styles, |full| {
                let dir = styles.resolve(TextElem::dir);
                if dir != config.dir {
                    // Insert "Explicit Directional Embedding".
                    match dir {
                        Dir::LTR => full.push_str(LTR_EMBEDDING),
                        Dir::RTL => full.push_str(RTL_EMBEDDING),
                        _ => {}
                    }
                }

                if let Some(case) = styles.get(TextElem::case) {
                    full.push_str(&case.apply(&elem.text));
                } else {
                    full.push_str(&elem.text);
                }

                if dir != config.dir {
                    // Insert "Pop Directional Formatting".
                    full.push_str(POP_EMBEDDING);
                }
            });
        } else if let Some(elem) = child.to_packed::<HElem>() {
            if elem.amount.is_zero() {
                continue;
            }

            collector.push_item(match elem.amount {
                Spacing::Fr(fr) => Item::Fractional(fr, None),
                Spacing::Rel(rel) => Item::Absolute(
                    rel.resolve(styles).relative_to(region.x),
                    elem.weak.get(styles),
                ),
            });
        } else if let Some(elem) = child.to_packed::<LinebreakElem>() {
            collector.push_text(
                if elem.justify.get(styles) { "\u{2028}" } else { "\n" },
                styles,
            );
        } else if let Some(elem) = child.to_packed::<SmartQuoteElem>() {
            let double = elem.double.get(styles);
            if elem.enabled.get(styles) {
                let quotes = SmartQuotes::get(
                    elem.quotes.get_ref(styles),
                    styles.get(TextElem::lang),
                    styles.get(TextElem::region),
                    elem.alternative.get(styles),
                );
                let before =
                    collector.full.chars().rev().find(|&c| !is_default_ignorable(c));
                let quote = quoter.quote(before, &quotes, double);
                collector.push_text(quote, styles);
            } else {
                collector.push_text(SmartQuotes::fallback(double), styles);
            }
        } else if let Some(elem) = child.to_packed::<InlineElem>() {
            collector.push_item(Item::Skip(LTR_ISOLATE));

            for item in elem.layout(engine, locator.next(&elem.span()), styles, region)? {
                match item {
                    InlineItem::Space(space, weak) => {
                        collector.push_item(Item::Absolute(space, weak));
                    }
                    InlineItem::Frame(mut frame) => {
                        frame.modify(&FrameModifiers::get_in(styles));
                        apply_shift(&engine.world, &mut frame, styles);
                        collector.push_item(Item::Frame(frame));
                    }
                }
            }

            collector.push_item(Item::Skip(POP_ISOLATE));
        } else if let Some(elem) = child.to_packed::<BoxElem>() {
            let loc = locator.next(&elem.span());
            if let Sizing::Fr(v) = elem.width.get(styles) {
                collector.push_item(Item::Fractional(v, Some((elem, loc, styles))));
            } else {
                let mut frame = layout_and_modify(styles, |styles| {
                    layout_box(elem, engine, loc, styles, region)
                })?;
                apply_shift(&engine.world, &mut frame, styles);
                collector.push_item(Item::Frame(frame));
            }
        } else if let Some(elem) = child.to_packed::<TagElem>() {
            collector.push_item(Item::Tag(&elem.tag));
        } else {
            // Non-paragraph inline layout should never trigger this since it
            // only won't be triggered if we see any non-inline content.
            engine.sink.warn(warning!(
                child.span(),
                "{} may not occur inside of a paragraph and was ignored",
                child.func().name(),
            ));
        };

        let len = collector.full.len() - prev_len;
        collector.spans.push(len, child.span());
    }

    Ok((collector.full, collector.segments, collector.spans))
}

/// Collects segments.
struct Collector<'a> {
    full: String,
    segments: Vec<Segment<'a>>,
    spans: SpanMapper,
}

impl<'a> Collector<'a> {
    fn new(capacity: usize) -> Self {
        Self {
            full: String::new(),
            segments: Vec::with_capacity(capacity),
            spans: SpanMapper::new(),
        }
    }

    fn push_text(&mut self, text: &str, styles: StyleChain<'a>) {
        self.build_text(styles, |full| full.push_str(text));
    }

    fn build_text<F>(&mut self, styles: StyleChain<'a>, f: F)
    where
        F: FnOnce(&mut String),
    {
        let prev = self.full.len();
        f(&mut self.full);
        let segment_len = self.full.len() - prev;

        // Merge adjacent text segments with the same styles.
        if let Some(Segment::Text(last_len, last_styles)) = self.segments.last_mut()
            && *last_styles == styles
        {
            *last_len += segment_len;
            return;
        }

        self.segments.push(Segment::Text(segment_len, styles));
    }

    fn push_item(&mut self, item: Item<'a>) {
        match (self.segments.last_mut(), &item) {
            // Merge adjacent weak spacing by taking the maximum.
            (
                Some(Segment::Item(Item::Absolute(prev_amount, true))),
                Item::Absolute(amount, true),
            ) => {
                *prev_amount = (*prev_amount).max(*amount);
            }

            _ => {
                self.full.push_str(item.textual());
                self.segments.push(Segment::Item(item));
            }
        }
    }
}

/// Maps byte offsets back to spans.
#[derive(Default)]
pub struct SpanMapper(Vec<(usize, Span)>);

impl SpanMapper {
    /// Create a new span mapper.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a span for a segment with the given length.
    pub fn push(&mut self, len: usize, span: Span) {
        self.0.push((len, span));
    }

    /// Determine the span at the given byte offset.
    ///
    /// May return a detached span.
    pub fn span_at(&self, offset: usize) -> (Span, u16) {
        let mut cursor = 0;
        for &(len, span) in &self.0 {
            if (cursor..cursor + len).contains(&offset) {
                return (span, u16::try_from(offset - cursor).unwrap_or(0));
            }
            cursor += len;
        }
        (Span::detached(), 0)
    }
}
