mod linebreak;
mod shaping;

use comemo::{Tracked, TrackedMut};
use unicode_bidi::{BidiInfo, Level as BidiLevel};
use unicode_script::{Script, UnicodeScript};

use self::linebreak::{breakpoints, Breakpoint};
use self::shaping::{
    cjk_punct_style, is_of_cj_script, shape, ShapedGlyph, ShapedText, BEGIN_PUNCT_PAT,
    END_PUNCT_PAT,
};
use crate::diag::{bail, SourceResult};
use crate::engine::{Engine, Route};
use crate::eval::Tracer;
use crate::foundations::{Packed, Resolve, Smart, StyleChain};
use crate::introspection::{Introspector, Locator, TagElem};
use crate::layout::{
    Abs, AlignElem, BoxElem, Dir, Em, FixedAlignment, Fr, Fragment, Frame, FrameItem,
    HElem, InlineElem, InlineItem, Point, Size, Sizing, Spacing,
};
use crate::model::{Linebreaks, ParElem};
use crate::realize::StyleVec;
use crate::syntax::Span;
use crate::text::{
    Costs, Lang, LinebreakElem, SmartQuoteElem, SmartQuoter, SmartQuotes, SpaceElem,
    TextElem,
};
use crate::utils::Numeric;
use crate::World;

/// Layouts content inline.
pub(crate) fn layout_inline(
    children: &StyleVec,
    engine: &mut Engine,
    styles: StyleChain,
    consecutive: bool,
    region: Size,
    expand: bool,
) -> SourceResult<Fragment> {
    #[comemo::memoize]
    #[allow(clippy::too_many_arguments)]
    fn cached(
        children: &StyleVec,
        world: Tracked<dyn World + '_>,
        introspector: Tracked<Introspector>,
        route: Tracked<Route>,
        locator: Tracked<Locator>,
        tracer: TrackedMut<Tracer>,
        styles: StyleChain,
        consecutive: bool,
        region: Size,
        expand: bool,
    ) -> SourceResult<Fragment> {
        let mut locator = Locator::chained(locator);
        let mut engine = Engine {
            world,
            introspector,
            route: Route::extend(route),
            locator: &mut locator,
            tracer,
        };

        // Collect all text into one string for BiDi analysis.
        let (text, segments, spans) =
            collect(children, &mut engine, &styles, region, consecutive)?;

        // Perform BiDi analysis and then prepare paragraph layout by building a
        // representation on which we can do line breaking without layouting
        // each and every line from scratch.
        let p = prepare(&mut engine, children, &text, segments, spans, styles)?;

        // Break the paragraph into lines.
        let lines = linebreak(&engine, &p, region.x - p.hang);

        // Stack the lines into one frame per region.
        let shrink = ParElem::shrink_in(styles);
        finalize(&mut engine, &p, &lines, region, expand, shrink)
    }

    let fragment = cached(
        children,
        engine.world,
        engine.introspector,
        engine.route.track(),
        engine.locator.track(),
        TrackedMut::reborrow_mut(&mut engine.tracer),
        styles,
        consecutive,
        region,
        expand,
    )?;

    engine.locator.visit_frames(&fragment);
    Ok(fragment)
}

/// Range of a substring of text.
type Range = std::ops::Range<usize>;

// The characters by which spacing, inline content and pins are replaced in the
// paragraph's full text.
const SPACING_REPLACE: &str = " "; // Space
const OBJ_REPLACE: &str = "\u{FFFC}"; // Object Replacement Character
const SPACING_REPLACE_CHAR: char = ' ';
const OBJ_REPLACE_CHAR: char = '\u{FFFC}';

// Unicode BiDi control characters.
const LTR_EMBEDDING: &str = "\u{202A}";
const RTL_EMBEDDING: &str = "\u{202B}";
const POP_EMBEDDING: &str = "\u{202C}";
const LTR_ISOLATE: &str = "\u{2066}";
const POP_ISOLATE: &str = "\u{2069}";

/// A paragraph representation in which children are already layouted and text
/// is already preshaped.
///
/// In many cases, we can directly reuse these results when constructing a line.
/// Only when a line break falls onto a text index that is not safe-to-break per
/// rustybuzz, we have to reshape that portion.
struct Preparation<'a> {
    /// Bidirectional text embedding levels for the paragraph.
    bidi: BidiInfo<'a>,
    /// Text runs, spacing and layouted elements.
    items: Vec<Item<'a>>,
    /// The span mapper.
    spans: SpanMapper,
    /// Whether to hyphenate if it's the same for all children.
    hyphenate: Option<bool>,
    /// Costs for various layout decisions.
    costs: Costs,
    /// The text language if it's the same for all children.
    lang: Option<Lang>,
    /// The paragraph's resolved horizontal alignment.
    align: FixedAlignment,
    /// Whether to justify the paragraph.
    justify: bool,
    /// The paragraph's hanging indent.
    hang: Abs,
    /// Whether to add spacing between CJK and Latin characters.
    cjk_latin_spacing: bool,
    /// Whether font fallback is enabled for this paragraph.
    fallback: bool,
    /// The leading of the paragraph.
    leading: Abs,
    /// How to determine line breaks.
    linebreaks: Smart<Linebreaks>,
    /// The text size.
    size: Abs,
}

impl<'a> Preparation<'a> {
    /// Find the item that contains the given `text_offset`.
    fn find(&self, text_offset: usize) -> Option<&Item<'a>> {
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
    fn slice(&self, text_range: Range) -> (Range, &[Item<'a>]) {
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

/// An item or not-yet shaped text. We can't shape text until we have collected
/// all items because only then we can compute BiDi, and we need to split shape
/// runs at level boundaries.
#[derive(Debug)]
enum Segment<'a> {
    /// One or multiple collapsed text children. Stores how long the segment is
    /// (in bytes of the full text string).
    Text(usize, StyleChain<'a>),
    /// An already prepared item.
    Item(Item<'a>),
}

impl Segment<'_> {
    /// The text length of the item.
    fn textual_len(&self) -> usize {
        match self {
            Self::Text(len, _) => *len,
            Self::Item(item) => item.textual_len(),
        }
    }
}

/// A prepared item in a paragraph layout.
#[derive(Debug)]
enum Item<'a> {
    /// A shaped text run with consistent style and direction.
    Text(ShapedText<'a>),
    /// Absolute spacing between other items, and whether it is weak.
    Absolute(Abs, bool),
    /// Fractional spacing between other items.
    Fractional(Fr, Option<(&'a Packed<BoxElem>, StyleChain<'a>)>),
    /// Layouted inline-level content.
    Frame(Frame, StyleChain<'a>),
    /// A tag.
    Tag(&'a Packed<TagElem>),
    /// An item that is invisible and needs to be skipped, e.g. a Unicode
    /// isolate.
    Skip(&'static str),
}

impl<'a> Item<'a> {
    /// If this a text item, return it.
    fn text(&self) -> Option<&ShapedText<'a>> {
        match self {
            Self::Text(shaped) => Some(shaped),
            _ => None,
        }
    }

    /// If this a text item, return it mutably.
    fn text_mut(&mut self) -> Option<&mut ShapedText<'a>> {
        match self {
            Self::Text(shaped) => Some(shaped),
            _ => None,
        }
    }

    /// Return the textual representation of this item: Either just itself (for
    /// a text item) or a replacement string (for any other item).
    fn textual(&self) -> &str {
        match self {
            Self::Text(shaped) => shaped.text,
            Self::Absolute(_, _) | Self::Fractional(_, _) => SPACING_REPLACE,
            Self::Frame(_, _) => OBJ_REPLACE,
            Self::Tag(_) => "",
            Self::Skip(s) => s,
        }
    }

    /// The text length of the item.
    fn textual_len(&self) -> usize {
        self.textual().len()
    }

    /// The natural layouted width of the item.
    fn width(&self) -> Abs {
        match self {
            Self::Text(shaped) => shaped.width,
            Self::Absolute(v, _) => *v,
            Self::Frame(frame, _) => frame.width(),
            Self::Fractional(_, _) | Self::Tag(_) => Abs::zero(),
            Self::Skip(_) => Abs::zero(),
        }
    }
}

/// Maps byte offsets back to spans.
#[derive(Default)]
struct SpanMapper(Vec<(usize, Span)>);

impl SpanMapper {
    /// Create a new span mapper.
    fn new() -> Self {
        Self::default()
    }

    /// Push a span for a segment with the given length.
    fn push(&mut self, len: usize, span: Span) {
        self.0.push((len, span));
    }

    /// Determine the span at the given byte offset.
    ///
    /// May return a detached span.
    fn span_at(&self, offset: usize) -> (Span, u16) {
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

/// A dash at the end of a line.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(super) enum Dash {
    /// A hyphen added to break a word.
    SoftHyphen,
    /// Regular hyphen, present in a compound word, e.g. beija-flor.
    HardHyphen,
    /// An em dash.
    Long,
    /// An en dash.
    Short,
}

/// A layouted line, consisting of a sequence of layouted paragraph items that
/// are mostly borrowed from the preparation phase. This type enables you to
/// measure the size of a line in a range before committing to building the
/// line's frame.
///
/// At most two paragraph items must be created individually for this line: The
/// first and last one since they may be broken apart by the start or end of the
/// line, respectively. But even those can partially reuse previous results when
/// the break index is safe-to-break per rustybuzz.
struct Line<'a> {
    /// Bidi information about the paragraph.
    bidi: &'a BidiInfo<'a>,
    /// The trimmed range the line spans in the paragraph.
    trimmed: Range,
    /// The untrimmed end where the line ends.
    end: usize,
    /// A reshaped text item if the line sliced up a text item at the start.
    first: Option<Item<'a>>,
    /// Inner items which don't need to be reprocessed.
    inner: &'a [Item<'a>],
    /// A reshaped text item if the line sliced up a text item at the end. If
    /// there is only one text item, this takes precedence over `first`.
    last: Option<Item<'a>>,
    /// The width of the line.
    width: Abs,
    /// Whether the line should be justified.
    justify: bool,
    /// Whether the line ends with a hyphen or dash, either naturally or through
    /// hyphenation.
    dash: Option<Dash>,
}

impl<'a> Line<'a> {
    /// Iterate over the line's items.
    fn items(&self) -> impl Iterator<Item = &Item<'a>> {
        self.first.iter().chain(self.inner).chain(&self.last)
    }

    /// Return items that intersect the given `text_range`.
    fn slice(&self, text_range: Range) -> impl Iterator<Item = &Item<'a>> {
        let mut cursor = self.trimmed.start;
        let mut start = 0;
        let mut end = 0;

        for (i, item) in self.items().enumerate() {
            if cursor <= text_range.start {
                start = i;
            }

            let len = item.textual_len();
            if cursor < text_range.end || cursor + len <= text_range.end {
                end = i + 1;
            } else {
                break;
            }

            cursor += len;
        }

        self.items().skip(start).take(end - start)
    }

    /// How many glyphs are in the text where we can insert additional
    /// space when encountering underfull lines.
    fn justifiables(&self) -> usize {
        let mut count = 0;
        for shaped in self.items().filter_map(Item::text) {
            count += shaped.justifiables();
        }
        // CJK character at line end should not be adjusted.
        if self
            .items()
            .last()
            .and_then(Item::text)
            .map(|s| s.cjk_justifiable_at_last())
            .unwrap_or(false)
        {
            count -= 1;
        }

        count
    }

    /// How much can the line stretch
    fn stretchability(&self) -> Abs {
        self.items().filter_map(Item::text).map(|s| s.stretchability()).sum()
    }

    /// How much can the line shrink
    fn shrinkability(&self) -> Abs {
        self.items().filter_map(Item::text).map(|s| s.shrinkability()).sum()
    }

    /// The sum of fractions in the line.
    fn fr(&self) -> Fr {
        self.items()
            .filter_map(|item| match item {
                Item::Fractional(fr, _) => Some(*fr),
                _ => None,
            })
            .sum()
    }
}

/// Collect all text of the paragraph into one string and layout equations. This
/// also performs string-level preprocessing like case transformations.
fn collect<'a>(
    children: &'a StyleVec,
    engine: &mut Engine<'_>,
    styles: &'a StyleChain<'a>,
    region: Size,
    consecutive: bool,
) -> SourceResult<(String, Vec<Segment<'a>>, SpanMapper)> {
    let mut collector = Collector::new(2 + children.len());
    let mut iter = children.chain(styles).peekable();

    let first_line_indent = ParElem::first_line_indent_in(*styles);
    if !first_line_indent.is_zero()
        && consecutive
        && AlignElem::alignment_in(*styles).resolve(*styles).x
            == TextElem::dir_in(*styles).start().into()
    {
        collector.push_item(Item::Absolute(first_line_indent.resolve(*styles), false));
        collector.spans.push(1, Span::detached());
    }

    let hang = ParElem::hanging_indent_in(*styles);
    if !hang.is_zero() {
        collector.push_item(Item::Absolute(-hang, false));
        collector.spans.push(1, Span::detached());
    }

    let outer_dir = TextElem::dir_in(*styles);

    while let Some((child, styles)) = iter.next() {
        let prev_len = collector.full.len();

        if child.is::<SpaceElem>() {
            collector.push_text(" ", styles);
        } else if let Some(elem) = child.to_packed::<TextElem>() {
            collector.build_text(styles, |full| {
                let dir = TextElem::dir_in(styles);
                if dir != outer_dir {
                    // Insert "Explicit Directional Embedding".
                    match dir {
                        Dir::LTR => full.push_str(LTR_EMBEDDING),
                        Dir::RTL => full.push_str(RTL_EMBEDDING),
                        _ => {}
                    }
                }

                if let Some(case) = TextElem::case_in(styles) {
                    full.push_str(&case.apply(elem.text()));
                } else {
                    full.push_str(elem.text());
                }

                if dir != outer_dir {
                    // Insert "Pop Directional Formatting".
                    full.push_str(POP_EMBEDDING);
                }
            });
        } else if let Some(elem) = child.to_packed::<HElem>() {
            let amount = elem.amount();
            if amount.is_zero() {
                continue;
            }

            collector.push_item(match amount {
                Spacing::Fr(fr) => Item::Fractional(*fr, None),
                Spacing::Rel(rel) => Item::Absolute(
                    rel.resolve(styles).relative_to(region.x),
                    elem.weak(styles),
                ),
            });
        } else if let Some(elem) = child.to_packed::<LinebreakElem>() {
            collector
                .push_text(if elem.justify(styles) { "\u{2028}" } else { "\n" }, styles);
        } else if let Some(elem) = child.to_packed::<SmartQuoteElem>() {
            let double = elem.double(styles);
            if elem.enabled(styles) {
                let quotes = SmartQuotes::new(
                    elem.quotes(styles),
                    TextElem::lang_in(styles),
                    TextElem::region_in(styles),
                    elem.alternative(styles),
                );
                let peeked = iter.peek().and_then(|(child, _)| {
                    if let Some(elem) = child.to_packed::<TextElem>() {
                        elem.text().chars().next()
                    } else if child.is::<SmartQuoteElem>() {
                        Some('"')
                    } else if child.is::<SpaceElem>()
                        || child.is::<HElem>()
                        || child.is::<LinebreakElem>()
                        // This is a temporary hack. We should rather skip these
                        // and peek at the next child.
                        || child.is::<TagElem>()
                    {
                        Some(SPACING_REPLACE_CHAR)
                    } else {
                        Some(OBJ_REPLACE_CHAR)
                    }
                });

                let quote = collector.quoter.quote(&quotes, double, peeked);
                collector.push_quote(quote, styles);
            } else {
                collector.push_text(if double { "\"" } else { "'" }, styles);
            }
        } else if let Some(elem) = child.to_packed::<InlineElem>() {
            collector.push_item(Item::Skip(LTR_ISOLATE));

            for item in elem.layout(engine, styles, region)? {
                match item {
                    InlineItem::Space(space, weak) => {
                        collector.push_item(Item::Absolute(space, weak));
                    }
                    InlineItem::Frame(frame) => {
                        collector.push_item(Item::Frame(frame, styles));
                    }
                }
            }

            collector.push_item(Item::Skip(POP_ISOLATE));
        } else if let Some(elem) = child.to_packed::<BoxElem>() {
            if let Sizing::Fr(v) = elem.width(styles) {
                collector.push_item(Item::Fractional(v, Some((elem, styles))));
            } else {
                let frame = elem.layout(engine, styles, region)?;
                collector.push_item(Item::Frame(frame, styles));
            }
        } else if let Some(elem) = child.to_packed::<TagElem>() {
            collector.push_item(Item::Tag(elem));
        } else {
            bail!(child.span(), "unexpected paragraph child");
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
    quoter: SmartQuoter,
}

impl<'a> Collector<'a> {
    fn new(capacity: usize) -> Self {
        Self {
            full: String::new(),
            segments: Vec::with_capacity(capacity),
            spans: SpanMapper::new(),
            quoter: SmartQuoter::new(),
        }
    }

    fn push_text(&mut self, text: &str, styles: StyleChain<'a>) {
        self.full.push_str(text);
        self.push_segment(Segment::Text(text.len(), styles), false);
    }

    fn build_text<F>(&mut self, styles: StyleChain<'a>, f: F)
    where
        F: FnOnce(&mut String),
    {
        let prev = self.full.len();
        f(&mut self.full);
        let len = self.full.len() - prev;
        self.push_segment(Segment::Text(len, styles), false);
    }

    fn push_quote(&mut self, quote: &str, styles: StyleChain<'a>) {
        self.full.push_str(quote);
        self.push_segment(Segment::Text(quote.len(), styles), true);
    }

    fn push_item(&mut self, item: Item<'a>) {
        self.full.push_str(item.textual());
        self.push_segment(Segment::Item(item), false);
    }

    fn push_segment(&mut self, segment: Segment<'a>, is_quote: bool) {
        if let Some(last) = self.full.chars().last() {
            self.quoter.last(last, is_quote);
        }

        if let (Some(Segment::Text(last_len, last_styles)), Segment::Text(len, styles)) =
            (self.segments.last_mut(), &segment)
        {
            if *last_styles == *styles {
                *last_len += *len;
                return;
            }
        }

        self.segments.push(segment);
    }
}

/// Prepare paragraph layout by shaping the whole paragraph.
fn prepare<'a>(
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

/// Add some spacing between Han characters and western characters.
/// See Requirements for Chinese Text Layout, Section 3.2.2 Mixed Text Composition in Horizontal
/// Written Mode
fn add_cjk_latin_spacing(items: &mut [Item]) {
    let mut items = items.iter_mut().filter(|x| !matches!(x, Item::Tag(_))).peekable();
    let mut prev: Option<&ShapedGlyph> = None;
    while let Some(item) = items.next() {
        let Some(text) = item.text_mut() else {
            prev = None;
            continue;
        };

        // Since we only call this function in [`prepare`], we can assume
        // that the Cow is owned, and `to_mut` can be called without overhead.
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

/// Group a range of text by BiDi level and script, shape the runs and generate
/// items for them.
fn shape_range<'a>(
    items: &mut Vec<Item<'a>>,
    engine: &Engine,
    bidi: &BidiInfo<'a>,
    range: Range,
    spans: &SpanMapper,
    styles: StyleChain<'a>,
) {
    let script = TextElem::script_in(styles);
    let lang = TextElem::lang_in(styles);
    let region = TextElem::region_in(styles);
    let mut process = |range: Range, level: BidiLevel| {
        let dir = if level.is_ltr() { Dir::LTR } else { Dir::RTL };
        let shaped = shape(
            engine,
            range.start,
            &bidi.text[range],
            spans,
            styles,
            dir,
            lang,
            region,
        );
        items.push(Item::Text(shaped));
    };

    let mut prev_level = BidiLevel::ltr();
    let mut prev_script = Script::Unknown;
    let mut cursor = range.start;

    // Group by embedding level and script.  If the text's script is explicitly
    // set (rather than inferred from the glyphs), we keep the script at an
    // unchanging `Script::Unknown` so that only level changes cause breaks.
    for i in range.clone() {
        if !bidi.text.is_char_boundary(i) {
            continue;
        }

        let level = bidi.levels[i];
        let curr_script = match script {
            Smart::Auto => {
                bidi.text[i..].chars().next().map_or(Script::Unknown, |c| c.script())
            }
            Smart::Custom(_) => Script::Unknown,
        };

        if level != prev_level || !is_compatible(curr_script, prev_script) {
            if cursor < i {
                process(cursor..i, prev_level);
            }
            cursor = i;
            prev_level = level;
            prev_script = curr_script;
        } else if is_generic_script(prev_script) {
            prev_script = curr_script;
        }
    }

    process(cursor..range.end, prev_level);
}

/// Whether this is not a specific script.
fn is_generic_script(script: Script) -> bool {
    matches!(script, Script::Unknown | Script::Common | Script::Inherited)
}

/// Whether these script can be part of the same shape run.
fn is_compatible(a: Script, b: Script) -> bool {
    is_generic_script(a) || is_generic_script(b) || a == b
}

/// Find suitable linebreaks.
fn linebreak<'a>(engine: &Engine, p: &'a Preparation<'a>, width: Abs) -> Vec<Line<'a>> {
    let linebreaks = p.linebreaks.unwrap_or_else(|| {
        if p.justify {
            Linebreaks::Optimized
        } else {
            Linebreaks::Simple
        }
    });

    match linebreaks {
        Linebreaks::Simple => linebreak_simple(engine, p, width),
        Linebreaks::Optimized => linebreak_optimized(engine, p, width),
    }
}

/// Perform line breaking in simple first-fit style. This means that we build
/// lines greedily, always taking the longest possible line. This may lead to
/// very unbalanced line, but is fast and simple.
fn linebreak_simple<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    width: Abs,
) -> Vec<Line<'a>> {
    let mut lines = Vec::with_capacity(16);
    let mut start = 0;
    let mut last = None;

    breakpoints(p, |end, breakpoint| {
        let prepend_hyphen = lines.last().map(should_repeat_hyphen).unwrap_or(false);

        // Compute the line and its size.
        let mut attempt = line(engine, p, start..end, breakpoint, prepend_hyphen);

        // If the line doesn't fit anymore, we push the last fitting attempt
        // into the stack and rebuild the line from the attempt's end. The
        // resulting line cannot be broken up further.
        if !width.fits(attempt.width) {
            if let Some((last_attempt, last_end)) = last.take() {
                lines.push(last_attempt);
                start = last_end;
                attempt = line(engine, p, start..end, breakpoint, prepend_hyphen);
            }
        }

        // Finish the current line if there is a mandatory line break (i.e.
        // due to "\n") or if the line doesn't fit horizontally already
        // since then no shorter line will be possible.
        if breakpoint == Breakpoint::Mandatory || !width.fits(attempt.width) {
            lines.push(attempt);
            start = end;
            last = None;
        } else {
            last = Some((attempt, end));
        }
    });

    if let Some((line, _)) = last {
        lines.push(line);
    }

    lines
}

/// Perform line breaking in optimized Knuth-Plass style. Here, we use more
/// context to determine the line breaks than in the simple first-fit style. For
/// example, we might choose to cut a line short even though there is still a
/// bit of space to improve the fit of one of the following lines. The
/// Knuth-Plass algorithm is based on the idea of "cost". A line which has a
/// very tight or very loose fit has a higher cost than one that is just right.
/// Ending a line with a hyphen incurs extra cost and endings two successive
/// lines with hyphens even more.
///
/// To find the layout with the minimal total cost the algorithm uses dynamic
/// programming: For each possible breakpoint it determines the optimal
/// paragraph layout _up to that point_. It walks over all possible start points
/// for a line ending at that point and finds the one for which the cost of the
/// line plus the cost of the optimal paragraph up to the start point (already
/// computed and stored in dynamic programming table) is minimal. The final
/// result is simply the layout determined for the last breakpoint at the end of
/// text.
fn linebreak_optimized<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    width: Abs,
) -> Vec<Line<'a>> {
    /// The cost of a line or paragraph layout.
    type Cost = f64;

    /// An entry in the dynamic programming table.
    struct Entry<'a> {
        pred: usize,
        total: Cost,
        line: Line<'a>,
    }

    // Cost parameters.
    const DEFAULT_HYPH_COST: Cost = 0.5;
    const DEFAULT_RUNT_COST: Cost = 0.5;
    const CONSECUTIVE_DASH_COST: Cost = 0.3;
    const MAX_COST: Cost = 1_000_000.0;
    const MIN_RATIO: f64 = -1.0;

    let hyph_cost = DEFAULT_HYPH_COST * p.costs.hyphenation().get();
    let runt_cost = DEFAULT_RUNT_COST * p.costs.runt().get();

    // Dynamic programming table.
    let mut active = 0;
    let mut table = vec![Entry {
        pred: 0,
        total: 0.0,
        line: line(engine, p, 0..0, Breakpoint::Mandatory, false),
    }];

    let em = p.size;
    let mut lines = Vec::with_capacity(16);
    breakpoints(p, |end, breakpoint| {
        let k = table.len();
        let is_end = end == p.bidi.text.len();
        let mut best: Option<Entry> = None;

        // Find the optimal predecessor.
        for (i, pred) in table.iter().enumerate().skip(active) {
            // Layout the line.
            let start = pred.line.end;
            let prepend_hyphen = should_repeat_hyphen(&pred.line);

            let attempt = line(engine, p, start..end, breakpoint, prepend_hyphen);

            // Determine how much the line's spaces would need to be stretched
            // to make it the desired width.
            let delta = width - attempt.width;
            // Determine how much stretch are permitted.
            let adjust = if delta >= Abs::zero() {
                attempt.stretchability()
            } else {
                attempt.shrinkability()
            };
            // Ideally, the ratio should between -1.0 and 1.0, but sometimes a value above 1.0
            // is possible, in which case the line is underfull.
            let mut ratio = delta / adjust;
            if ratio.is_nan() {
                // The line is not stretchable, but it just fits.
                // This often happens with monospace fonts and CJK texts.
                ratio = 0.0;
            }
            if ratio > 1.0 {
                // We should stretch the line above its stretchability. Now
                // calculate the extra amount. Also, don't divide by zero.
                let extra_stretch =
                    (delta - adjust) / attempt.justifiables().max(1) as f64;
                // Normalize the amount by half Em size.
                ratio = 1.0 + extra_stretch / (em / 2.0);
            }

            // Determine the cost of the line.
            let min_ratio = if p.justify { MIN_RATIO } else { 0.0 };
            let mut cost = if ratio < min_ratio {
                // The line is overfull. This is the case if
                // - justification is on, but we'd need to shrink too much
                // - justification is off and the line just doesn't fit
                //
                // If this is the earliest breakpoint in the active set
                // (active == i), remove it from the active set. If there is an
                // earlier one (active < i), then the logically shorter line was
                // in fact longer (can happen with negative spacing) and we
                // can't trim the active set just yet.
                if active == i {
                    active += 1;
                }
                MAX_COST
            } else if breakpoint == Breakpoint::Mandatory || is_end {
                // This is a mandatory break and the line is not overfull, so
                // all breakpoints before this one become inactive since no line
                // can span above the mandatory break.
                active = k;
                // If ratio > 0, we need to stretch the line only when justify is needed.
                // If ratio < 0, we always need to shrink the line.
                if (ratio > 0.0 && attempt.justify) || ratio < 0.0 {
                    ratio.powi(3).abs()
                } else {
                    0.0
                }
            } else {
                // Normal line with cost of |ratio^3|.
                ratio.powi(3).abs()
            };

            // Penalize runts.
            if k == i + 1 && is_end {
                cost += runt_cost;
            }

            // Penalize hyphens.
            if breakpoint == Breakpoint::Hyphen {
                cost += hyph_cost;
            }

            // In Knuth paper, cost = (1 + 100|r|^3 + p)^2 + a,
            // where r is the ratio, p=50 is the penalty, and a=3000 is consecutive the penalty.
            // We divide the whole formula by 10, resulting (0.01 + |r|^3 + p)^2 + a,
            // where p=0.5 and a=0.3
            cost = (0.01 + cost).powi(2);

            // Penalize two consecutive dashes (not necessarily hyphens) extra.
            if attempt.dash.is_some() && pred.line.dash.is_some() {
                cost += CONSECUTIVE_DASH_COST;
            }

            // The total cost of this line and its chain of predecessors.
            let total = pred.total + cost;

            // If this attempt is better than what we had before, take it!
            if best.as_ref().map_or(true, |best| best.total >= total) {
                best = Some(Entry { pred: i, total, line: attempt });
            }
        }

        table.push(best.unwrap());
    });

    // Retrace the best path.
    let mut idx = table.len() - 1;
    while idx != 0 {
        table.truncate(idx + 1);
        let entry = table.pop().unwrap();
        lines.push(entry.line);
        idx = entry.pred;
    }

    lines.reverse();
    lines
}

/// Create a line which spans the given range.
fn line<'a>(
    engine: &Engine,
    p: &'a Preparation,
    mut range: Range,
    breakpoint: Breakpoint,
    prepend_hyphen: bool,
) -> Line<'a> {
    let end = range.end;
    let mut justify =
        p.justify && end < p.bidi.text.len() && breakpoint != Breakpoint::Mandatory;

    if range.is_empty() {
        return Line {
            bidi: &p.bidi,
            end,
            trimmed: range,
            first: None,
            inner: &[],
            last: None,
            width: Abs::zero(),
            justify,
            dash: None,
        };
    }

    // Slice out the relevant items.
    let (mut expanded, mut inner) = p.slice(range.clone());
    let mut width = Abs::zero();

    // Weak space (Absolute(_, weak=true)) would be removed if at the end of the line
    while let Some((Item::Absolute(_, true), before)) = inner.split_last() {
        // apply it recursively to ensure the last one is not weak space
        inner = before;
        range.end -= 1;
        expanded.end -= 1;
    }
    // Weak space (Absolute(_, weak=true)) would be removed if at the beginning of the line
    while let Some((Item::Absolute(_, true), after)) = inner.split_first() {
        // apply it recursively to ensure the first one is not weak space
        inner = after;
        range.start += 1;
        expanded.end += 1;
    }

    // Reshape the last item if it's split in half or hyphenated.
    let mut last = None;
    let mut dash = None;
    if let Some((Item::Text(shaped), before)) = inner.split_last() {
        // Compute the range we want to shape, trimming whitespace at the
        // end of the line.
        let base = expanded.end - shaped.text.len();
        let start = range.start.max(base);
        let text = &p.bidi.text[start..range.end];
        // U+200B ZERO WIDTH SPACE is used to provide a line break opportunity,
        // we want to trim it too.
        let trimmed = text.trim_end().trim_end_matches('\u{200B}');
        range.end = start + trimmed.len();

        // Deal with hyphens, dashes and justification.
        let shy = trimmed.ends_with('\u{ad}');
        let hyphen = breakpoint == Breakpoint::Hyphen;
        dash = if hyphen || shy {
            Some(Dash::SoftHyphen)
        } else if trimmed.ends_with('-') {
            Some(Dash::HardHyphen)
        } else if trimmed.ends_with('–') {
            Some(Dash::Short)
        } else if trimmed.ends_with('—') {
            Some(Dash::Long)
        } else {
            None
        };
        justify |= text.ends_with('\u{2028}');

        // Deal with CJK punctuation at line ends.
        let gb_style = cjk_punct_style(shaped.lang, shaped.region);
        let maybe_adjust_last_glyph = trimmed.ends_with(END_PUNCT_PAT)
            || (p.cjk_latin_spacing && trimmed.ends_with(is_of_cj_script));

        // Usually, we don't want to shape an empty string because:
        // - We don't want the height of trimmed whitespace in a different
        //   font to be considered for the line height.
        // - Even if it's in the same font, its unnecessary.
        //
        // There is one exception though. When the whole line is empty, we
        // need the shaped empty string to make the line the appropriate
        // height. That is the case exactly if the string is empty and there
        // are no other items in the line.
        if hyphen
            || start + shaped.text.len() > range.end
            || maybe_adjust_last_glyph
            || prepend_hyphen
        {
            if hyphen || start < range.end || before.is_empty() {
                let mut reshaped = shaped.reshape(engine, &p.spans, start..range.end);
                if hyphen || shy {
                    reshaped.push_hyphen(engine, p.fallback);
                }

                if let Some(last_glyph) = reshaped.glyphs.last() {
                    if last_glyph.is_cjk_left_aligned_punctuation(gb_style) {
                        // If the last glyph is a CJK punctuation, we want to shrink it.
                        // See Requirements for Chinese Text Layout, Section 3.1.6.3
                        // Compression of punctuation marks at line start or line end
                        let shrink_amount = last_glyph.shrinkability().1;
                        let punct = reshaped.glyphs.to_mut().last_mut().unwrap();
                        punct.shrink_right(shrink_amount);
                        reshaped.width -= shrink_amount.at(reshaped.size);
                    } else if p.cjk_latin_spacing
                        && last_glyph.is_cj_script()
                        && (last_glyph.x_advance - last_glyph.x_offset) > Em::one()
                    {
                        // If the last glyph is a CJK character adjusted by [`add_cjk_latin_spacing`],
                        // restore the original width.
                        let shrink_amount =
                            last_glyph.x_advance - last_glyph.x_offset - Em::one();
                        let glyph = reshaped.glyphs.to_mut().last_mut().unwrap();
                        glyph.x_advance -= shrink_amount;
                        glyph.adjustability.shrinkability.1 = Em::zero();
                        reshaped.width -= shrink_amount.at(reshaped.size);
                    }
                }

                width += reshaped.width;
                last = Some(Item::Text(reshaped));
            }

            inner = before;
        }
    }

    // Deal with CJ characters at line starts.
    let text = &p.bidi.text[range.start..end];
    let maybe_adjust_first_glyph = text.starts_with(BEGIN_PUNCT_PAT)
        || (p.cjk_latin_spacing && text.starts_with(is_of_cj_script));

    // Reshape the start item if it's split in half.
    let mut first = None;
    if let Some((Item::Text(shaped), after)) = inner.split_first() {
        // Compute the range we want to shape.
        let base = expanded.start;
        let end = range.end.min(base + shaped.text.len());

        // Reshape if necessary.
        if range.start + shaped.text.len() > end
            || maybe_adjust_first_glyph
            || prepend_hyphen
        {
            // If the range is empty, we don't want to push an empty text item.
            if range.start < end {
                let reshaped = shaped.reshape(engine, &p.spans, range.start..end);
                width += reshaped.width;
                first = Some(Item::Text(reshaped));
            }

            inner = after;
        }
    }

    if prepend_hyphen {
        let reshaped = first.as_mut().or(last.as_mut()).and_then(Item::text_mut);
        if let Some(reshaped) = reshaped {
            let width_before = reshaped.width;
            reshaped.prepend_hyphen(engine, p.fallback);
            width += reshaped.width - width_before;
        }
    }

    if maybe_adjust_first_glyph {
        let reshaped = first.as_mut().or(last.as_mut()).and_then(Item::text_mut);
        if let Some(reshaped) = reshaped {
            if let Some(first_glyph) = reshaped.glyphs.first() {
                if first_glyph.is_cjk_right_aligned_punctuation() {
                    // If the first glyph is a CJK punctuation, we want to shrink it.
                    let shrink_amount = first_glyph.shrinkability().0;
                    let glyph = reshaped.glyphs.to_mut().first_mut().unwrap();
                    glyph.shrink_left(shrink_amount);
                    let amount_abs = shrink_amount.at(reshaped.size);
                    reshaped.width -= amount_abs;
                    width -= amount_abs;
                } else if p.cjk_latin_spacing
                    && first_glyph.is_cj_script()
                    && first_glyph.x_offset > Em::zero()
                {
                    // If the first glyph is a CJK character adjusted by [`add_cjk_latin_spacing`],
                    // restore the original width.
                    let shrink_amount = first_glyph.x_offset;
                    let glyph = reshaped.glyphs.to_mut().first_mut().unwrap();
                    glyph.x_advance -= shrink_amount;
                    glyph.x_offset = Em::zero();
                    glyph.adjustability.shrinkability.0 = Em::zero();
                    let amount_abs = shrink_amount.at(reshaped.size);
                    reshaped.width -= amount_abs;
                    width -= amount_abs;
                }
            }
        }
    }

    // Measure the inner items.
    for item in inner {
        width += item.width();
    }

    Line {
        bidi: &p.bidi,
        trimmed: range,
        end,
        first,
        inner,
        last,
        width,
        justify,
        dash,
    }
}

/// Combine layouted lines into one frame per region.
fn finalize(
    engine: &mut Engine,
    p: &Preparation,
    lines: &[Line],
    region: Size,
    expand: bool,
    shrink: bool,
) -> SourceResult<Fragment> {
    // Determine the paragraph's width: Full width of the region if we
    // should expand or there's fractional spacing, fit-to-width otherwise.
    let width = if !region.x.is_finite()
        || (!expand && lines.iter().all(|line| line.fr().is_zero()))
    {
        region
            .x
            .min(p.hang + lines.iter().map(|line| line.width).max().unwrap_or_default())
    } else {
        region.x
    };

    // Stack the lines into one frame per region.
    let mut frames: Vec<Frame> = lines
        .iter()
        .map(|line| commit(engine, p, line, width, region.y, shrink))
        .collect::<SourceResult<_>>()?;

    // Positive ratios enable prevention, while zero and negative ratios disable it.
    if p.costs.orphan().get() > 0.0 {
        // Prevent orphans.
        if frames.len() >= 2 && !frames[1].is_empty() {
            let second = frames.remove(1);
            let first = &mut frames[0];
            merge(first, second, p.leading);
        }
    }
    if p.costs.widow().get() > 0.0 {
        // Prevent widows.
        let len = frames.len();
        if len >= 2 && !frames[len - 2].is_empty() {
            let second = frames.pop().unwrap();
            let first = frames.last_mut().unwrap();
            merge(first, second, p.leading);
        }
    }

    Ok(Fragment::frames(frames))
}

/// Merge two line frames
fn merge(first: &mut Frame, second: Frame, leading: Abs) {
    let offset = first.height() + leading;
    let total = offset + second.height();
    first.push_frame(Point::with_y(offset), second);
    first.size_mut().y = total;
}

/// Commit to a line and build its frame.
fn commit(
    engine: &mut Engine,
    p: &Preparation,
    line: &Line,
    width: Abs,
    full: Abs,
    shrink: bool,
) -> SourceResult<Frame> {
    let mut remaining = width - line.width - p.hang;
    let mut offset = Abs::zero();

    // Reorder the line from logical to visual order.
    let (reordered, starts_rtl) = reorder(line);
    if !starts_rtl {
        offset += p.hang;
    }

    // Handle hanging punctuation to the left.
    if let Some(Item::Text(text)) = reordered.first() {
        if let Some(glyph) = text.glyphs.first() {
            if !text.dir.is_positive()
                && TextElem::overhang_in(text.styles)
                && (reordered.len() > 1 || text.glyphs.len() > 1)
            {
                let amount = overhang(glyph.c) * glyph.x_advance.at(text.size);
                offset -= amount;
                remaining += amount;
            }
        }
    }

    // Handle hanging punctuation to the right.
    if let Some(Item::Text(text)) = reordered.last() {
        if let Some(glyph) = text.glyphs.last() {
            if text.dir.is_positive()
                && TextElem::overhang_in(text.styles)
                && (reordered.len() > 1 || text.glyphs.len() > 1)
            {
                let amount = overhang(glyph.c) * glyph.x_advance.at(text.size);
                remaining += amount;
            }
        }
    }

    // Determine how much additional space is needed.
    // The justification_ratio is for the first step justification,
    // extra_justification is for the last step.
    // For more info on multi-step justification, see Procedures for Inter-
    // Character Space Expansion in W3C document Chinese Layout Requirements.
    let fr = line.fr();
    let mut justification_ratio = 0.0;
    let mut extra_justification = Abs::zero();

    let shrinkability = line.shrinkability();
    let stretch = line.stretchability();
    if remaining < Abs::zero() && shrinkability > Abs::zero() && shrink {
        // Attempt to reduce the length of the line, using shrinkability.
        justification_ratio = (remaining / shrinkability).max(-1.0);
        remaining = (remaining + shrinkability).min(Abs::zero());
    } else if line.justify && fr.is_zero() {
        // Attempt to increase the length of the line, using stretchability.
        if stretch > Abs::zero() {
            justification_ratio = (remaining / stretch).min(1.0);
            remaining = (remaining - stretch).max(Abs::zero());
        }

        let justifiables = line.justifiables();
        if justifiables > 0 && remaining > Abs::zero() {
            // Underfull line, distribute the extra space.
            extra_justification = remaining / justifiables as f64;
            remaining = Abs::zero();
        }
    }

    let mut top = Abs::zero();
    let mut bottom = Abs::zero();

    // Build the frames and determine the height and baseline.
    let mut frames = vec![];
    for item in reordered {
        let mut push = |offset: &mut Abs, frame: Frame| {
            let width = frame.width();
            top.set_max(frame.baseline());
            bottom.set_max(frame.size().y - frame.baseline());
            frames.push((*offset, frame));
            *offset += width;
        };

        match item {
            Item::Absolute(v, _) => {
                offset += *v;
            }
            Item::Fractional(v, elem) => {
                let amount = v.share(fr, remaining);
                if let Some((elem, styles)) = elem {
                    let region = Size::new(amount, full);
                    let mut frame = elem.layout(engine, *styles, region)?;
                    frame.post_process(*styles);
                    frame.translate(Point::with_y(TextElem::baseline_in(*styles)));
                    push(&mut offset, frame);
                } else {
                    offset += amount;
                }
            }
            Item::Text(shaped) => {
                let mut frame =
                    shaped.build(engine, justification_ratio, extra_justification);
                frame.post_process(shaped.styles);
                push(&mut offset, frame);
            }
            Item::Frame(frame, styles) => {
                let mut frame = frame.clone();
                frame.post_process(*styles);
                frame.translate(Point::with_y(TextElem::baseline_in(*styles)));
                push(&mut offset, frame);
            }
            Item::Tag(tag) => {
                let mut frame = Frame::soft(Size::zero());
                frame.push(Point::zero(), FrameItem::Tag(tag.elem.clone()));
                frames.push((offset, frame));
            }
            Item::Skip(_) => {}
        }
    }

    // Remaining space is distributed now.
    if !fr.is_zero() {
        remaining = Abs::zero();
    }

    let size = Size::new(width, top + bottom);
    let mut output = Frame::soft(size);
    output.set_baseline(top);

    // Construct the line's frame.
    for (offset, frame) in frames {
        let x = offset + p.align.position(remaining);
        let y = top - frame.baseline();
        output.push_frame(Point::new(x, y), frame);
    }

    Ok(output)
}

/// Return a line's items in visual order.
fn reorder<'a>(line: &'a Line<'a>) -> (Vec<&Item<'a>>, bool) {
    let mut reordered = vec![];

    // The bidi crate doesn't like empty lines.
    if line.trimmed.is_empty() {
        return (line.slice(line.trimmed.clone()).collect(), false);
    }

    // Find the paragraph that contains the line.
    let para = line
        .bidi
        .paragraphs
        .iter()
        .find(|para| para.range.contains(&line.trimmed.start))
        .unwrap();

    // Compute the reordered ranges in visual order (left to right).
    let (levels, runs) = line.bidi.visual_runs(para, line.trimmed.clone());
    let starts_rtl = levels.first().is_some_and(|level| level.is_rtl());

    // Collect the reordered items.
    for run in runs {
        // Skip reset L1 runs because handling them would require reshaping
        // again in some cases.
        if line.bidi.levels[run.start] != levels[run.start] {
            continue;
        }

        let prev = reordered.len();
        reordered.extend(line.slice(run.clone()));

        if levels[run.start].is_rtl() {
            reordered[prev..].reverse();
        }
    }

    (reordered, starts_rtl)
}

/// How much a character should hang into the end margin.
///
/// For more discussion, see:
/// <https://recoveringphysicist.com/21/>
fn overhang(c: char) -> f64 {
    match c {
        // Dashes.
        '–' | '—' => 0.2,
        '-' => 0.55,

        // Punctuation.
        '.' | ',' => 0.8,
        ':' | ';' => 0.3,

        // Arabic
        '\u{60C}' | '\u{6D4}' => 0.4,

        _ => 0.0,
    }
}

/// Whether the hyphen should repeat at the start of the next line.
fn should_repeat_hyphen(pred_line: &Line) -> bool {
    // If the predecessor line does not end with a Dash::HardHyphen, we shall
    // not place a hyphen at the start of the next line.
    if pred_line.dash != Some(Dash::HardHyphen) {
        return false;
    }

    // If there's a trimmed out space, we needn't repeat the hyphen. That's the
    // case of a text like "...kebab é a -melhor- comida que existe", where the
    // hyphens are a kind of emphasis marker.
    if pred_line.trimmed.end != pred_line.end {
        return false;
    }

    // The hyphen should repeat only in the languages that require that feature.
    // For more information see the discussion at https://github.com/typst/typst/issues/3235
    let Some(Item::Text(shape)) = pred_line.last.as_ref() else { return false };

    match shape.lang {
        // - Lower Sorbian: see https://dolnoserbski.de/ortografija/psawidla/K3
        // - Czech: see https://prirucka.ujc.cas.cz/?id=164
        // - Croatian: see http://pravopis.hr/pravilo/spojnica/68/
        // - Polish: see https://www.ortograf.pl/zasady-pisowni/lacznik-zasady-pisowni
        // - Portuguese: see https://www2.senado.leg.br/bdsf/bitstream/handle/id/508145/000997415.pdf (Base XX)
        // - Slovak: see https://www.zones.sk/studentske-prace/gramatika/10620-pravopis-rozdelovanie-slov/
        Lang::LOWER_SORBIAN
        | Lang::CZECH
        | Lang::CROATIAN
        | Lang::POLISH
        | Lang::PORTUGUESE
        | Lang::SLOVAK => true,
        // In Spanish the hyphen is required only if the word next to hyphen is
        // not capitalized. Otherwise, the hyphen must not be repeated.
        //
        // See § 4.1.1.1.2.e on the "Ortografía de la lengua española"
        // https://www.rae.es/ortografía/como-signo-de-división-de-palabras-a-final-de-línea
        Lang::SPANISH => pred_line.bidi.text[pred_line.end..]
            .chars()
            .next()
            .map(|c| !c.is_uppercase())
            .unwrap_or(false),
        _ => false,
    }
}
