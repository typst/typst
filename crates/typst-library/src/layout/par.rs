use std::iter::Peekable;

use icu_properties::{maps::CodePointMapData, LineBreak};
use icu_provider::AsDeserializingBufferProvider;
use icu_provider_adapters::fork::ForkByKeyProvider;
use icu_provider_blob::BlobDataProvider;
use icu_segmenter::{LineBreakIteratorUtf8, LineSegmenter};
use once_cell::sync::Lazy;
use typst::eval::Tracer;
use typst::model::DelayedErrors;
use unicode_bidi::{BidiInfo, Level as BidiLevel};
use unicode_script::{Script, UnicodeScript};

use super::{BoxElem, HElem, Sizing, Spacing};
use crate::layout::AlignElem;
use crate::math::EquationElem;
use crate::prelude::*;
use crate::text::{
    is_gb_style, shape, LinebreakElem, Quoter, Quotes, ShapedText, SmartquoteElem,
    SpaceElem, TextElem,
};

/// Arranges text, spacing and inline-level elements into a paragraph.
///
/// Although this function is primarily used in set rules to affect paragraph
/// properties, it can also be used to explicitly render its argument onto a
/// paragraph of its own.
///
/// # Example
/// ```example
/// #show par: set block(spacing: 0.65em)
/// #set par(
///   first-line-indent: 1em,
///   justify: true,
/// )
///
/// We proceed by contradiction.
/// Suppose that there exists a set
/// of positive integers $a$, $b$, and
/// $c$ that satisfies the equation
/// $a^n + b^n = c^n$ for some
/// integer value of $n > 2$.
///
/// Without loss of generality,
/// let $a$ be the smallest of the
/// three integers. Then, we ...
/// ```
#[elem(title = "Paragraph", Construct)]
pub struct ParElem {
    /// The spacing between lines.
    #[resolve]
    #[default(Em::new(0.65).into())]
    pub leading: Length,

    /// Whether to justify text in its line.
    ///
    /// Hyphenation will be enabled for justified paragraphs if the
    /// [text function's `hyphenate` property]($text.hyphenate) is set to
    /// `{auto}` and the current language is known.
    ///
    /// Note that the current [alignment]($align) still has an effect on the
    /// placement of the last line except if it ends with a
    /// [justified line break]($linebreak.justify).
    #[default(false)]
    pub justify: bool,

    /// How to determine line breaks.
    ///
    /// When this property is set to `{auto}`, its default value, optimized line
    /// breaks will be used for justified paragraphs. Enabling optimized line
    /// breaks for ragged paragraphs may also be worthwhile to improve the
    /// appearance of the text.
    ///
    /// ```example
    /// #set page(width: 207pt)
    /// #set par(linebreaks: "simple")
    /// Some texts feature many longer
    /// words. Those are often exceedingly
    /// challenging to break in a visually
    /// pleasing way.
    ///
    /// #set par(linebreaks: "optimized")
    /// Some texts feature many longer
    /// words. Those are often exceedingly
    /// challenging to break in a visually
    /// pleasing way.
    /// ```
    pub linebreaks: Smart<Linebreaks>,

    /// The indent the first line of a paragraph should have.
    ///
    /// Only the first line of a consecutive paragraph will be indented (not
    /// the first one in a block or on the page).
    ///
    /// By typographic convention, paragraph breaks are indicated either by some
    /// space between paragraphs or by indented first lines. Consider reducing
    /// the [paragraph spacing]($block.spacing) to the [`leading`] when
    /// using this property (e.g. using
    /// `[#show par: set block(spacing: 0.65em)]`).
    pub first_line_indent: Length,

    /// The indent all but the first line of a paragraph should have.
    #[resolve]
    pub hanging_indent: Length,

    /// The contents of the paragraph.
    #[external]
    #[required]
    pub body: Content,

    /// The paragraph's children.
    #[internal]
    #[variadic]
    pub children: Vec<Content>,
}

impl Construct for ParElem {
    fn construct(vm: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        // The paragraph constructor is special: It doesn't create a paragraph
        // element. Instead, it just ensures that the passed content lives in a
        // separate paragraph and styles it.
        let styles = Self::set(vm, args)?;
        let body = args.expect::<Content>("body")?;
        Ok(Content::sequence([
            ParbreakElem::new().pack(),
            body.styled_with_map(styles),
            ParbreakElem::new().pack(),
        ]))
    }
}

impl ParElem {
    /// Layout the paragraph into a collection of lines.
    #[tracing::instrument(name = "ParElement::layout", skip_all)]
    pub fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        consecutive: bool,
        region: Size,
        expand: bool,
    ) -> SourceResult<Fragment> {
        #[comemo::memoize]
        #[allow(clippy::too_many_arguments)]
        fn cached(
            par: &ParElem,
            world: Tracked<dyn World + '_>,
            introspector: Tracked<Introspector>,
            locator: Tracked<Locator>,
            delayed: TrackedMut<DelayedErrors>,
            tracer: TrackedMut<Tracer>,
            styles: StyleChain,
            consecutive: bool,
            region: Size,
            expand: bool,
        ) -> SourceResult<Fragment> {
            let mut locator = Locator::chained(locator);
            let mut vt = Vt {
                world,
                introspector,
                locator: &mut locator,
                delayed,
                tracer,
            };
            let children = par.children();

            // Collect all text into one string for BiDi analysis.
            let (text, segments, spans) = collect(&children, &styles, consecutive)?;

            // Perform BiDi analysis and then prepare paragraph layout by building a
            // representation on which we can do line breaking without layouting
            // each and every line from scratch.
            let p = prepare(&mut vt, &children, &text, segments, spans, styles, region)?;

            // Break the paragraph into lines.
            let lines = linebreak(&vt, &p, region.x - p.hang);

            // Stack the lines into one frame per region.
            finalize(&mut vt, &p, &lines, region, expand)
        }

        let fragment = cached(
            self,
            vt.world,
            vt.introspector,
            vt.locator.track(),
            TrackedMut::reborrow_mut(&mut vt.delayed),
            TrackedMut::reborrow_mut(&mut vt.tracer),
            styles,
            consecutive,
            region,
            expand,
        )?;

        vt.locator.visit_frames(&fragment);
        Ok(fragment)
    }
}

/// How to determine line breaks in a paragraph.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Linebreaks {
    /// Determine the line breaks in a simple first-fit style.
    Simple,
    /// Optimize the line breaks for the whole paragraph.
    ///
    /// Typst will try to produce more evenly filled lines of text by
    /// considering the whole paragraph when calculating line breaks.
    Optimized,
}

/// A paragraph break.
///
/// This starts a new paragraph. Especially useful when used within code like
/// [for loops]($scripting/#loops). Multiple consecutive
/// paragraph breaks collapse into a single one.
///
/// # Example
/// ```example
/// #for i in range(3) {
///   [Blind text #i: ]
///   lorem(5)
///   parbreak()
/// }
/// ```
///
/// # Syntax
/// Instead of calling this function, you can insert a blank line into your
/// markup to create a paragraph break.
#[elem(title = "Paragraph Break", Unlabellable)]
pub struct ParbreakElem {}

impl Unlabellable for ParbreakElem {}

/// Range of a substring of text.
type Range = std::ops::Range<usize>;

// The characters by which spacing, inline content and pins are replaced in the
// paragraph's full text.
const SPACING_REPLACE: char = ' '; // Space
const OBJ_REPLACE: char = '\u{FFFC}'; // Object Replacement Character

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
    /// The styles shared by all children.
    styles: StyleChain<'a>,
    /// Whether to hyphenate if it's the same for all children.
    hyphenate: Option<bool>,
    /// The text language if it's the same for all children.
    lang: Option<Lang>,
    /// The paragraph's resolved horizontal alignment.
    align: FixedAlign,
    /// Whether to justify the paragraph.
    justify: bool,
    /// The paragraph's hanging indent.
    hang: Abs,
}

impl<'a> Preparation<'a> {
    /// Find the item that contains the given `text_offset`.
    fn find(&self, text_offset: usize) -> Option<&Item<'a>> {
        let mut cursor = 0;
        for item in &self.items {
            let end = cursor + item.len();
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

            let len = item.len();
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

/// A segment of one or multiple collapsed children.
#[derive(Debug, Copy, Clone)]
enum Segment<'a> {
    /// One or multiple collapsed text or text-equivalent children. Stores how
    /// long the segment is (in bytes of the full text string).
    Text(usize),
    /// Horizontal spacing between other segments.
    Spacing(Spacing),
    /// A mathematical equation.
    Equation(&'a EquationElem),
    /// A box with arbitrary content.
    Box(&'a BoxElem, bool),
    /// Metadata.
    Meta,
}

impl Segment<'_> {
    /// The text length of the item.
    fn len(&self) -> usize {
        match *self {
            Self::Text(len) => len,
            Self::Spacing(_) => SPACING_REPLACE.len_utf8(),
            Self::Box(_, true) => SPACING_REPLACE.len_utf8(),
            Self::Equation(_) | Self::Box(_, _) => OBJ_REPLACE.len_utf8(),
            Self::Meta => 0,
        }
    }
}

/// A prepared item in a paragraph layout.
#[derive(Debug)]
enum Item<'a> {
    /// A shaped text run with consistent style and direction.
    Text(ShapedText<'a>),
    /// Absolute spacing between other items.
    Absolute(Abs),
    /// Fractional spacing between other items.
    Fractional(Fr, Option<(&'a BoxElem, StyleChain<'a>)>),
    /// Layouted inline-level content.
    Frame(Frame),
    /// Metadata.
    Meta(Frame),
}

impl<'a> Item<'a> {
    /// If this a text item, return it.
    fn text(&self) -> Option<&ShapedText<'a>> {
        match self {
            Self::Text(shaped) => Some(shaped),
            _ => None,
        }
    }

    fn text_mut(&mut self) -> Option<&mut ShapedText<'a>> {
        match self {
            Self::Text(shaped) => Some(shaped),
            _ => None,
        }
    }

    /// The text length of the item.
    fn len(&self) -> usize {
        match self {
            Self::Text(shaped) => shaped.text.len(),
            Self::Absolute(_) | Self::Fractional(_, _) => SPACING_REPLACE.len_utf8(),
            Self::Frame(_) => OBJ_REPLACE.len_utf8(),
            Self::Meta(_) => 0,
        }
    }

    /// The natural layouted width of the item.
    fn width(&self) -> Abs {
        match self {
            Self::Text(shaped) => shaped.width,
            Self::Absolute(v) => *v,
            Self::Frame(frame) => frame.width(),
            Self::Fractional(_, _) | Self::Meta(_) => Abs::zero(),
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
            if (cursor..=cursor + len).contains(&offset) {
                return (span, u16::try_from(offset - cursor).unwrap_or(0));
            }
            cursor += len;
        }
        (Span::detached(), 0)
    }
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
    dash: bool,
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

            let len = item.len();
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

/// Collect all text of the paragraph into one string. This also performs
/// string-level preprocessing like case transformations.
#[allow(clippy::type_complexity)]
fn collect<'a>(
    children: &'a [Content],
    styles: &'a StyleChain<'a>,
    consecutive: bool,
) -> SourceResult<(String, Vec<(Segment<'a>, StyleChain<'a>)>, SpanMapper)> {
    let mut full = String::new();
    let mut quoter = Quoter::new();
    let mut segments = vec![];
    let mut spans = SpanMapper::new();
    let mut iter = children.iter().peekable();

    let first_line_indent = ParElem::first_line_indent_in(*styles);
    if !first_line_indent.is_zero()
        && consecutive
        && AlignElem::alignment_in(*styles).resolve(*styles).x
            == TextElem::dir_in(*styles).start().into()
    {
        full.push(SPACING_REPLACE);
        segments.push((Segment::Spacing(first_line_indent.into()), *styles));
    }

    let hang = ParElem::hanging_indent_in(*styles);
    if !hang.is_zero() {
        full.push(SPACING_REPLACE);
        segments.push((Segment::Spacing((-hang).into()), *styles));
    }

    while let Some(mut child) = iter.next() {
        let outer = styles;
        let mut styles = *styles;
        if let Some((elem, local)) = child.to_styled() {
            child = elem;
            styles = outer.chain(local);
        }

        let segment = if child.is::<SpaceElem>() {
            full.push(' ');
            Segment::Text(1)
        } else if let Some(elem) = child.to::<TextElem>() {
            let prev = full.len();
            if let Some(case) = TextElem::case_in(styles) {
                full.push_str(&case.apply(&elem.text()));
            } else {
                full.push_str(&elem.text());
            }
            Segment::Text(full.len() - prev)
        } else if let Some(elem) = child.to::<HElem>() {
            if elem.amount().is_zero() {
                continue;
            }

            full.push(SPACING_REPLACE);
            Segment::Spacing(elem.amount())
        } else if let Some(elem) = child.to::<LinebreakElem>() {
            let c = if elem.justify(styles) { '\u{2028}' } else { '\n' };
            full.push(c);
            Segment::Text(c.len_utf8())
        } else if let Some(elem) = child.to::<SmartquoteElem>() {
            let prev = full.len();
            if SmartquoteElem::enabled_in(styles) {
                let quotes = SmartquoteElem::quotes_in(styles);
                let lang = TextElem::lang_in(styles);
                let region = TextElem::region_in(styles);
                let quotes = Quotes::new(
                    &quotes,
                    lang,
                    region,
                    SmartquoteElem::alternative_in(styles),
                );
                let peeked = iter.peek().and_then(|child| {
                    let child = if let Some((child, _)) = child.to_styled() {
                        child
                    } else {
                        child
                    };
                    if let Some(elem) = child.to::<TextElem>() {
                        elem.text().chars().next()
                    } else if child.is::<SmartquoteElem>() {
                        Some('"')
                    } else if child.is::<SpaceElem>()
                        || child.is::<HElem>()
                        || child.is::<LinebreakElem>()
                    {
                        Some(SPACING_REPLACE)
                    } else {
                        Some(OBJ_REPLACE)
                    }
                });

                full.push_str(quoter.quote(&quotes, elem.double(styles), peeked));
            } else {
                full.push(if elem.double(styles) { '"' } else { '\'' });
            }
            Segment::Text(full.len() - prev)
        } else if let Some(elem) = child.to::<EquationElem>() {
            full.push(OBJ_REPLACE);
            Segment::Equation(elem)
        } else if let Some(elem) = child.to::<BoxElem>() {
            let frac = elem.width(styles).is_fractional();
            full.push(if frac { SPACING_REPLACE } else { OBJ_REPLACE });
            Segment::Box(elem, frac)
        } else if child.is::<MetaElem>() {
            Segment::Meta
        } else {
            bail!(child.span(), "unexpected paragraph child");
        };

        if let Some(last) = full.chars().last() {
            quoter.last(last, child.is::<SmartquoteElem>());
        }

        spans.push(segment.len(), child.span());

        if let (Some((Segment::Text(last_len), last_styles)), Segment::Text(len)) =
            (segments.last_mut(), segment)
        {
            if *last_styles == styles {
                *last_len += len;
                continue;
            }
        }

        segments.push((segment, styles));
    }

    Ok((full, segments, spans))
}

/// Prepare paragraph layout by shaping the whole paragraph and layouting all
/// contained inline-level content.
fn prepare<'a>(
    vt: &mut Vt,
    children: &'a [Content],
    text: &'a str,
    segments: Vec<(Segment<'a>, StyleChain<'a>)>,
    spans: SpanMapper,
    styles: StyleChain<'a>,
    region: Size,
) -> SourceResult<Preparation<'a>> {
    let dir = TextElem::dir_in(styles);
    let bidi = BidiInfo::new(
        text,
        match dir {
            Dir::LTR => Some(BidiLevel::ltr()),
            Dir::RTL => Some(BidiLevel::rtl()),
            _ => None,
        },
    );

    let mut cursor = 0;
    let mut items = vec![];

    // Shape / layout the children and collect them into items.
    for (segment, styles) in segments {
        let end = cursor + segment.len();
        match segment {
            Segment::Text(_) => {
                shape_range(&mut items, vt, &bidi, cursor..end, &spans, styles);
            }
            Segment::Spacing(spacing) => match spacing {
                Spacing::Rel(v) => {
                    let resolved = v.resolve(styles).relative_to(region.x);
                    items.push(Item::Absolute(resolved));
                }
                Spacing::Fr(v) => {
                    items.push(Item::Fractional(v, None));
                }
            },
            Segment::Equation(equation) => {
                let pod = Regions::one(region, Axes::splat(false));
                let mut frame = equation.layout(vt, styles, pod)?.into_frame();
                frame.translate(Point::with_y(TextElem::baseline_in(styles)));
                items.push(Item::Frame(frame));
            }
            Segment::Box(elem, _) => {
                if let Sizing::Fr(v) = elem.width(styles) {
                    items.push(Item::Fractional(v, Some((elem, styles))));
                } else {
                    let pod = Regions::one(region, Axes::splat(false));
                    let mut frame = elem.layout(vt, styles, pod)?.into_frame();
                    frame.translate(Point::with_y(TextElem::baseline_in(styles)));
                    items.push(Item::Frame(frame));
                }
            }
            Segment::Meta => {
                let mut frame = Frame::soft(Size::zero());
                frame.meta(styles, true);
                items.push(Item::Meta(frame));
            }
        }

        cursor = end;
    }

    Ok(Preparation {
        bidi,
        items,
        spans,
        styles,
        hyphenate: shared_get(styles, children, TextElem::hyphenate_in),
        lang: shared_get(styles, children, TextElem::lang_in),
        align: AlignElem::alignment_in(styles).resolve(styles).x,
        justify: ParElem::justify_in(styles),
        hang: ParElem::hanging_indent_in(styles),
    })
}

/// Group a range of text by BiDi level and script, shape the runs and generate
/// items for them.
fn shape_range<'a>(
    items: &mut Vec<Item<'a>>,
    vt: &Vt,
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
        let shaped =
            shape(vt, range.start, &bidi.text[range], spans, styles, dir, lang, region);
        items.push(Item::Text(shaped));
    };

    let mut prev_level = BidiLevel::ltr();
    let mut prev_script = Script::Unknown;
    let mut cursor = range.start;

    // Group by embedding level and script.  If the text's script is explicitly
    // set (rather than inferred from the glpyhs), we keep the script at an
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

/// Get a style property, but only if it is the same for all children of the
/// paragraph.
fn shared_get<T: PartialEq>(
    styles: StyleChain<'_>,
    children: &[Content],
    getter: fn(StyleChain) -> T,
) -> Option<T> {
    let value = getter(styles);
    children
        .iter()
        .filter_map(|child| child.to_styled())
        .all(|(_, local)| getter(styles.chain(local)) == value)
        .then_some(value)
}

/// Find suitable linebreaks.
fn linebreak<'a>(vt: &Vt, p: &'a Preparation<'a>, width: Abs) -> Vec<Line<'a>> {
    let linebreaks = ParElem::linebreaks_in(p.styles).unwrap_or_else(|| {
        if ParElem::justify_in(p.styles) {
            Linebreaks::Optimized
        } else {
            Linebreaks::Simple
        }
    });

    match linebreaks {
        Linebreaks::Simple => linebreak_simple(vt, p, width),
        Linebreaks::Optimized => linebreak_optimized(vt, p, width),
    }
}

/// Perform line breaking in simple first-fit style. This means that we build
/// lines greedily, always taking the longest possible line. This may lead to
/// very unbalanced line, but is fast and simple.
fn linebreak_simple<'a>(vt: &Vt, p: &'a Preparation<'a>, width: Abs) -> Vec<Line<'a>> {
    let mut lines = vec![];
    let mut start = 0;
    let mut last = None;

    for (end, mandatory, hyphen) in breakpoints(p) {
        // Compute the line and its size.
        let mut attempt = line(vt, p, start..end, mandatory, hyphen);

        // If the line doesn't fit anymore, we push the last fitting attempt
        // into the stack and rebuild the line from the attempt's end. The
        // resulting line cannot be broken up further.
        if !width.fits(attempt.width) {
            if let Some((last_attempt, last_end)) = last.take() {
                lines.push(last_attempt);
                start = last_end;
                attempt = line(vt, p, start..end, mandatory, hyphen);
            }
        }

        // Finish the current line if there is a mandatory line break (i.e.
        // due to "\n") or if the line doesn't fit horizontally already
        // since then no shorter line will be possible.
        if mandatory || !width.fits(attempt.width) {
            lines.push(attempt);
            start = end;
            last = None;
        } else {
            last = Some((attempt, end));
        }
    }

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
fn linebreak_optimized<'a>(vt: &Vt, p: &'a Preparation<'a>, width: Abs) -> Vec<Line<'a>> {
    /// The cost of a line or paragraph layout.
    type Cost = f64;

    /// An entry in the dynamic programming table.
    struct Entry<'a> {
        pred: usize,
        total: Cost,
        line: Line<'a>,
    }

    // Cost parameters.
    const HYPH_COST: Cost = 0.5;
    const RUNT_COST: Cost = 0.5;
    const CONSECUTIVE_DASH_COST: Cost = 0.3;
    const MAX_COST: Cost = 1_000_000.0;
    const MIN_RATIO: f64 = -1.0;

    // Dynamic programming table.
    let mut active = 0;
    let mut table = vec![Entry {
        pred: 0,
        total: 0.0,
        line: line(vt, p, 0..0, false, false),
    }];

    let em = TextElem::size_in(p.styles);

    for (end, mandatory, hyphen) in breakpoints(p) {
        let k = table.len();
        let eof = end == p.bidi.text.len();
        let mut best: Option<Entry> = None;

        // Find the optimal predecessor.
        for (i, pred) in table.iter().enumerate().skip(active) {
            // Layout the line.
            let start = pred.line.end;

            let attempt = line(vt, p, start..end, mandatory, hyphen);

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
                // We should stretch the line above its stretchability. Now calculate the extra amount.
                let extra_stretch = (delta - adjust) / attempt.justifiables() as f64;
                // Normalize the amount by half Em size.
                ratio = 1.0 + extra_stretch / (em / 2.0);
            }

            // Determine the cost of the line.
            let min_ratio = if p.justify { MIN_RATIO } else { 0.0 };
            let mut cost = if ratio < min_ratio {
                // The line is overfull. This is the case if
                // - justification is on, but we'd need to shrink too much
                // - justification is off and the line just doesn't fit
                // Since any longer line will also be overfull, we can deactivate
                // this breakpoint.
                active = i + 1;
                MAX_COST
            } else if mandatory || eof {
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
            if k == i + 1 && eof {
                cost += RUNT_COST;
            }

            // Penalize hyphens.
            if hyphen {
                cost += HYPH_COST;
            }

            // In Knuth paper, cost = (1 + 100|r|^3 + p)^2 + a,
            // where r is the ratio, p=50 is the penalty, and a=3000 is consecutive the penalty.
            // We divide the whole formula by 10, resulting (0.01 + |r|^3 + p)^2 + a,
            // where p=0.5 and a=0.3
            cost = (0.01 + cost).powi(2);

            // Penalize two consecutive dashes (not necessarily hyphens) extra.
            if attempt.dash && pred.line.dash {
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
    }

    // Retrace the best path.
    let mut lines = vec![];
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

/// Generated by the following command:
///
/// ```sh
/// icu4x-datagen  --locales full --keys-for-bin target/debug/typst \
///                --format blob --out library/assets/icudata.postcard --overwrite
/// ```
///
/// Install icu4x-datagen with `cargo install icu4x-datagen`.
static ICU_DATA: &[u8] = include_bytes!("../../assets/icudata.postcard");

/// Generated by the following command:
///
/// ```sh
/// icu4x-datagen --locales zh ja --keys segmenter/line@1 --format blob \
///               --out library/assets/cj_linebreak_data.postcard --overwrite
/// ```
///
/// The used icu4x-datagen should be patched by
/// https://github.com/peng1999/icu4x/commit/b9beb6cbf633d61fc3d7983e5baf7f4449fbfae5
static CJ_LINEBREAK_DATA: &[u8] =
    include_bytes!("../../assets/cj_linebreak_data.postcard");

/// The general line break segmenter.
static SEGMENTER: Lazy<LineSegmenter> = Lazy::new(|| {
    let provider = BlobDataProvider::try_new_from_static_blob(ICU_DATA).unwrap();
    LineSegmenter::try_new_lstm_with_buffer_provider(&provider).unwrap()
});

/// The Unicode line break properties for each code point.
static CJ_SEGMENTER: Lazy<LineSegmenter> = Lazy::new(|| {
    let provider = BlobDataProvider::try_new_from_static_blob(ICU_DATA).unwrap();
    let cj_blob = BlobDataProvider::try_new_from_static_blob(CJ_LINEBREAK_DATA).unwrap();
    let cj_provider = ForkByKeyProvider::new(cj_blob, provider);
    LineSegmenter::try_new_lstm_with_buffer_provider(&cj_provider).unwrap()
});

/// The line break segmenter for Chinese/Jpanese text.
static LINEBREAK_DATA: Lazy<CodePointMapData<LineBreak>> = Lazy::new(|| {
    let provider = BlobDataProvider::try_new_from_static_blob(ICU_DATA).unwrap();
    let deser_provider = provider.as_deserializing();
    icu_properties::maps::load_line_break(&deser_provider).unwrap()
});

/// Determine all possible points in the text where lines can broken.
///
/// Returns for each breakpoint the text index, whether the break is mandatory
/// (after `\n`) and whether a hyphen is required (when breaking inside of a
/// word).
fn breakpoints<'a>(p: &'a Preparation<'a>) -> Breakpoints<'a> {
    let mut linebreaks = if matches!(p.lang, Some(Lang::CHINESE | Lang::JAPANESE)) {
        CJ_SEGMENTER.segment_str(p.bidi.text)
    } else {
        SEGMENTER.segment_str(p.bidi.text)
    };
    // The iterator always yields a breakpoint at index 0, we want to ignore it
    linebreaks.next();
    Breakpoints {
        p,
        linebreaks: linebreaks.peekable(),
        syllables: None,
        offset: 0,
        suffix: 0,
        end: 0,
        mandatory: false,
    }
}

/// An iterator over the line break opportunities in a text.
struct Breakpoints<'a> {
    /// The paragraph's items.
    p: &'a Preparation<'a>,
    /// The inner iterator over the unicode line break opportunities.
    linebreaks: Peekable<LineBreakIteratorUtf8<'a, 'a>>,
    /// Iterator over syllables of the current word.
    syllables: Option<hypher::Syllables<'a>>,
    /// The current text offset.
    offset: usize,
    /// The trimmed end of the current word.
    suffix: usize,
    /// The untrimmed end of the current word.
    end: usize,
    /// Whether the break after the current word is mandatory.
    mandatory: bool,
}

impl Iterator for Breakpoints<'_> {
    type Item = (usize, bool, bool);

    fn next(&mut self) -> Option<Self::Item> {
        // If we're currently in a hyphenated "word", process the next syllable.
        if let Some(syllable) = self.syllables.as_mut().and_then(Iterator::next) {
            self.offset += syllable.len();
            if self.offset == self.suffix {
                self.offset = self.end;
            }

            // Filter out hyphenation opportunities where hyphenation was
            // actually disabled.
            let hyphen = self.offset < self.end;
            if hyphen && !self.hyphenate(self.offset) {
                return self.next();
            }

            return Some((self.offset, self.mandatory && !hyphen, hyphen));
        }

        let lb = LINEBREAK_DATA.as_borrowed();

        // Get the next "word".
        self.end = self.linebreaks.next()?;
        self.mandatory =
            self.p.bidi.text[..self.end].chars().next_back().map_or(false, |c| {
                matches!(
                    lb.get(c),
                    LineBreak::MandatoryBreak
                        | LineBreak::CarriageReturn
                        | LineBreak::LineFeed
                        | LineBreak::NextLine
                ) || self.end == self.p.bidi.text.len()
            });

        // Hyphenate the next word.
        if self.p.hyphenate != Some(false) {
            if let Some(lang) = self.lang(self.offset) {
                let word = &self.p.bidi.text[self.offset..self.end];
                let trimmed = word.trim_end_matches(|c: char| !c.is_alphabetic());
                if !trimmed.is_empty() {
                    self.suffix = self.offset + trimmed.len();
                    self.syllables = Some(hypher::hyphenate(trimmed, lang));
                    return self.next();
                }
            }
        }

        // Fix for https://github.com/unicode-org/icu4x/issues/3811
        if !self.mandatory {
            while let Some(&next) = self.linebreaks.peek() {
                if !self.p.bidi.text[self.end..next]
                    .contains(|c: char| !c.is_whitespace())
                {
                    self.end = next;
                    self.linebreaks.next();
                } else {
                    break;
                }
            }
        }

        self.offset = self.end;
        Some((self.end, self.mandatory, false))
    }
}

impl Breakpoints<'_> {
    /// Whether hyphenation is enabled at the given offset.
    fn hyphenate(&self, offset: usize) -> bool {
        self.p
            .hyphenate
            .or_else(|| {
                let shaped = self.p.find(offset)?.text()?;
                Some(TextElem::hyphenate_in(shaped.styles))
            })
            .unwrap_or(false)
    }

    /// The text language at the given offset.
    fn lang(&self, offset: usize) -> Option<hypher::Lang> {
        let lang = self.p.lang.or_else(|| {
            let shaped = self.p.find(offset)?.text()?;
            Some(TextElem::lang_in(shaped.styles))
        })?;

        let bytes = lang.as_str().as_bytes().try_into().ok()?;
        hypher::Lang::from_iso(bytes)
    }
}

/// Create a line which spans the given range.
fn line<'a>(
    vt: &Vt,
    p: &'a Preparation,
    mut range: Range,
    mandatory: bool,
    hyphen: bool,
) -> Line<'a> {
    let end = range.end;
    let mut justify = p.justify && end < p.bidi.text.len() && !mandatory;

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
            dash: false,
        };
    }

    // Slice out the relevant items.
    let (expanded, mut inner) = p.slice(range.clone());
    let mut width = Abs::zero();

    // Reshape the last item if it's split in half or hyphenated.
    let mut last = None;
    let mut dash = false;
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
        dash = hyphen || shy || trimmed.ends_with(['-', '–', '—']);
        justify |= text.ends_with('\u{2028}');

        // Deal with CJK punctuation at line ends.
        let gb_style = is_gb_style(shaped.lang, shaped.region);
        let end_cjk_punct = trimmed
            .ends_with(['”', '’', '，', '。', '、', '：', '；', '》', '）', '』', '」']);

        // Usually, we don't want to shape an empty string because:
        // - We don't want the height of trimmed whitespace in a different
        //   font to be considered for the line height.
        // - Even if it's in the same font, its unnecessary.
        //
        // There is one exception though. When the whole line is empty, we
        // need the shaped empty string to make the line the appropriate
        // height. That is the case exactly if the string is empty and there
        // are no other items in the line.
        if hyphen || start + shaped.text.len() > range.end || end_cjk_punct {
            if hyphen || start < range.end || before.is_empty() {
                let mut reshaped = shaped.reshape(vt, &p.spans, start..range.end);
                if hyphen || shy {
                    reshaped.push_hyphen(vt, TextElem::fallback_in(p.styles));
                }
                let punct = reshaped.glyphs.last();
                if let Some(punct) = punct {
                    if punct.is_cjk_left_aligned_punctuation(gb_style) {
                        let shrink_amount = punct.shrinkability().1;
                        let punct = reshaped.glyphs.to_mut().last_mut().unwrap();
                        punct.shrink_right(shrink_amount);
                        reshaped.width -= shrink_amount.at(reshaped.size);
                    }
                }
                width += reshaped.width;
                last = Some(Item::Text(reshaped));
            }

            inner = before;
        }
    }

    // Deal with CJK punctuation at line starts.
    let text = &p.bidi.text[range.start..end];
    let start_cjk_punct = text.starts_with(['“', '‘', '《', '（', '『', '「']);

    // Reshape the start item if it's split in half.
    let mut first = None;
    if let Some((Item::Text(shaped), after)) = inner.split_first() {
        // Compute the range we want to shape.
        let base = expanded.start;
        let end = range.end.min(base + shaped.text.len());

        // Reshape if necessary.
        if range.start + shaped.text.len() > end || start_cjk_punct {
            if range.start < end || start_cjk_punct {
                let reshaped = shaped.reshape(vt, &p.spans, range.start..end);
                width += reshaped.width;
                first = Some(Item::Text(reshaped));
            }

            inner = after;
        }
    }

    if start_cjk_punct {
        let reshaped = first.as_mut().or(last.as_mut()).and_then(Item::text_mut);
        if let Some(reshaped) = reshaped {
            if let Some(punct) = reshaped.glyphs.first() {
                if punct.is_cjk_right_aligned_punctuation() {
                    let shrink_amount = punct.shrinkability().0;
                    let punct = reshaped.glyphs.to_mut().first_mut().unwrap();
                    punct.shrink_left(shrink_amount);
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
    vt: &mut Vt,
    p: &Preparation,
    lines: &[Line],
    region: Size,
    expand: bool,
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
        .map(|line| commit(vt, p, line, width, region.y))
        .collect::<SourceResult<_>>()?;

    // Prevent orphans.
    let leading = ParElem::leading_in(p.styles);
    if frames.len() >= 2 && !frames[1].is_empty() {
        let second = frames.remove(1);
        let first = &mut frames[0];
        merge(first, second, leading);
    }

    // Prevent widows.
    let len = frames.len();
    if len >= 2 && !frames[len - 2].is_empty() {
        let second = frames.pop().unwrap();
        let first = frames.last_mut().unwrap();
        merge(first, second, leading);
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
    vt: &mut Vt,
    p: &Preparation,
    line: &Line,
    width: Abs,
    full: Abs,
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
    // The justicication_ratio is for the first step justification,
    // extra_justification is for the last step.
    // For more info on multi-step justification, see Procedures for Inter-
    // Character Space Expansion in W3C document Chinese Layout Requirements.
    let fr = line.fr();
    let mut justification_ratio = 0.0;
    let mut extra_justification = Abs::zero();

    let shrink = line.shrinkability();
    let stretch = line.stretchability();
    if remaining < Abs::zero() && shrink > Abs::zero() {
        // Attempt to reduce the length of the line, using shrinkability.
        justification_ratio = (remaining / shrink).max(-1.0);
        remaining = (remaining + shrink).min(Abs::zero());
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
            Item::Absolute(v) => {
                offset += *v;
            }
            Item::Fractional(v, elem) => {
                let amount = v.share(fr, remaining);
                if let Some((elem, styles)) = elem {
                    let region = Size::new(amount, full);
                    let pod = Regions::one(region, Axes::new(true, false));
                    let mut frame = elem.layout(vt, *styles, pod)?.into_frame();
                    frame.translate(Point::with_y(TextElem::baseline_in(*styles)));
                    push(&mut offset, frame);
                } else {
                    offset += amount;
                }
            }
            Item::Text(shaped) => {
                let frame = shaped.build(vt, justification_ratio, extra_justification);
                push(&mut offset, frame);
            }
            Item::Frame(frame) | Item::Meta(frame) => {
                push(&mut offset, frame.clone());
            }
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
    let starts_rtl = levels.first().map_or(false, |level| level.is_rtl());

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
/// https://recoveringphysicist.com/21/
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
