use std::sync::Arc;

use either::Either;
use unicode_bidi::{BidiInfo, Level};
use xi_unicode::LineBreakIterator;

use super::{shape, ShapedText, TextNode};
use crate::font::FontStore;
use crate::library::layout::Spacing;
use crate::library::prelude::*;
use crate::util::{ArcExt, EcoString, RangeExt, SliceExt};

/// Arrange text, spacing and inline-level nodes into a paragraph.
#[derive(Hash)]
pub struct ParNode(pub StyleVec<ParChild>);

/// A uniformly styled atomic piece of a paragraph.
#[derive(Hash)]
pub enum ParChild {
    /// A chunk of text.
    Text(EcoString),
    /// Horizontal spacing between other children.
    Spacing(Spacing),
    /// An arbitrary inline-level node.
    Node(LayoutNode),
}

#[node]
impl ParNode {
    /// An ISO 639-1 language code.
    #[property(referenced)]
    pub const LANG: Option<EcoString> = None;
    /// The direction for text and inline objects.
    pub const DIR: Dir = Dir::LTR;
    /// How to align text and inline objects in their line.
    pub const ALIGN: Align = Align::Left;
    /// Whether to justify text in its line.
    pub const JUSTIFY: bool = false;
    /// How to determine line breaks.
    pub const LINEBREAKS: Smart<Linebreaks> = Smart::Auto;
    /// Whether to hyphenate text to improve line breaking. When `auto`, words
    /// will will be hyphenated if and only if justification is enabled.
    pub const HYPHENATE: Smart<bool> = Smart::Auto;
    /// The spacing between lines (dependent on scaled font size).
    pub const LEADING: Linear = Relative::new(0.65).into();
    /// The extra spacing between paragraphs (dependent on scaled font size).
    pub const SPACING: Linear = Relative::new(0.55).into();
    /// The indent the first line of a consecutive paragraph should have.
    pub const INDENT: Linear = Linear::zero();

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        // The paragraph constructor is special: It doesn't create a paragraph
        // since that happens automatically through markup. Instead, it just
        // lifts the passed body to the block level so that it won't merge with
        // adjacent stuff and it styles the contained paragraphs.
        Ok(Content::Block(args.expect("body")?))
    }

    fn set(args: &mut Args) -> TypResult<StyleMap> {
        let mut styles = StyleMap::new();

        let lang = args.named::<Option<EcoString>>("lang")?;
        let mut dir =
            lang.clone().flatten().map(|iso| match iso.to_lowercase().as_str() {
                "ar" | "dv" | "fa" | "he" | "ks" | "pa" | "ps" | "sd" | "ug" | "ur"
                | "yi" => Dir::RTL,
                _ => Dir::LTR,
            });

        if let Some(Spanned { v, span }) = args.named::<Spanned<Dir>>("dir")? {
            if v.axis() != SpecAxis::Horizontal {
                bail!(span, "must be horizontal");
            }
            dir = Some(v);
        }

        let align =
            if let Some(Spanned { v, span }) = args.named::<Spanned<Align>>("align")? {
                if v.axis() != SpecAxis::Horizontal {
                    bail!(span, "must be horizontal");
                }
                Some(v)
            } else {
                dir.map(|dir| dir.start().into())
            };

        styles.set_opt(Self::LANG, lang);
        styles.set_opt(Self::DIR, dir);
        styles.set_opt(Self::ALIGN, align);
        styles.set_opt(Self::JUSTIFY, args.named("justify")?);
        styles.set_opt(Self::LINEBREAKS, args.named("linebreaks")?);
        styles.set_opt(Self::HYPHENATE, args.named("hyphenate")?);
        styles.set_opt(Self::LEADING, args.named("leading")?);
        styles.set_opt(Self::SPACING, args.named("spacing")?);
        styles.set_opt(Self::INDENT, args.named("indent")?);

        Ok(styles)
    }
}

impl ParNode {
    /// Concatenate all text in the paragraph into one string, replacing spacing
    /// with a space character and other non-text nodes with the object
    /// replacement character.
    fn collect_text(&self) -> String {
        let mut text = String::new();
        for string in self.strings() {
            text.push_str(string);
        }
        text
    }

    /// The range of each item in the collected text.
    fn ranges(&self) -> impl Iterator<Item = Range> + '_ {
        let mut cursor = 0;
        self.strings().map(move |string| {
            let start = cursor;
            cursor += string.len();
            start .. cursor
        })
    }

    /// The string representation of each child.
    fn strings(&self) -> impl Iterator<Item = &str> {
        self.0.items().map(|child| match child {
            ParChild::Text(text) => text,
            ParChild::Spacing(_) => " ",
            ParChild::Node(_) => "\u{FFFC}",
        })
    }
}

impl Layout for ParNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        // Collect all text into one string and perform BiDi analysis.
        let text = self.collect_text();

        // Prepare paragraph layout by building a representation on which we can
        // do line breaking without layouting each and every line from scratch.
        let p = prepare(ctx, self, &text, regions, &styles)?;

        // Break the paragraph into lines.
        let lines = linebreak(&p, &mut ctx.fonts, regions.first.x, styles);

        // Stack the lines into one frame per region.
        Ok(stack(&lines, &ctx.fonts, regions, styles))
    }
}

impl Debug for ParNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Par ")?;
        self.0.fmt(f)
    }
}

impl Debug for ParChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Text(text) => write!(f, "Text({:?})", text),
            Self::Spacing(kind) => write!(f, "{:?}", kind),
            Self::Node(node) => node.fmt(f),
        }
    }
}

impl Merge for ParChild {
    fn merge(&mut self, next: &Self) -> bool {
        if let (Self::Text(left), Self::Text(right)) = (self, next) {
            left.push_str(right);
            true
        } else {
            false
        }
    }
}

/// How to determine line breaks in a paragraph.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Linebreaks {
    /// Determine the linebreaks in a simple first-fit style.
    Simple,
    /// Optimize the linebreaks for the whole paragraph.
    Optimized,
}

castable! {
    Linebreaks,
    Expected: "string",
    Value::Str(string) => match string.as_str() {
        "simple" => Self::Simple,
        "optimized" => Self::Optimized,
        _ => Err(r#"expected "simple" or "optimized""#)?,
    },
}

/// A paragraph break.
pub struct ParbreakNode;

#[node]
impl ParbreakNode {
    fn construct(_: &mut Context, _: &mut Args) -> TypResult<Content> {
        Ok(Content::Parbreak)
    }
}

/// A line break.
pub struct LinebreakNode;

#[node]
impl LinebreakNode {
    fn construct(_: &mut Context, _: &mut Args) -> TypResult<Content> {
        Ok(Content::Linebreak)
    }
}

/// Range of a substring of text.
type Range = std::ops::Range<usize>;

/// A paragraph representation in which children are already layouted and text
/// is already preshaped.
///
/// In many cases, we can directly reuse these results when constructing a line.
/// Only when a line break falls onto a text index that is not safe-to-break per
/// rustybuzz, we have to reshape that portion.
struct Preparation<'a> {
    /// Bidirectional text embedding levels for the paragraph.
    bidi: BidiInfo<'a>,
    /// Spacing, separated text runs and layouted nodes.
    items: Vec<ParItem<'a>>,
    /// The ranges of the items in `bidi.text`.
    ranges: Vec<Range>,
}

impl Preparation<'_> {
    /// Find the index of the item whose range contains the `text_offset`.
    fn find(&self, text_offset: usize) -> Option<usize> {
        self.ranges.binary_search_by(|r| r.locate(text_offset)).ok()
    }
}

/// A prepared item in a paragraph layout.
enum ParItem<'a> {
    /// Absolute spacing between other items.
    Absolute(Length),
    /// Fractional spacing between other items.
    Fractional(Fractional),
    /// A shaped text run with consistent direction.
    Text(ShapedText<'a>),
    /// A layouted child node.
    Frame(Frame),
}

/// A layouted line, consisting of a sequence of layouted paragraph items that
/// are mostly borrowed from the preparation phase. This type enables you to
/// measure the size of a line in a range before comitting to building the
/// line's frame.
///
/// At most two paragraph items must be created individually for this line: The
/// first and last one since they may be broken apart by the start or end of the
/// line, respectively. But even those can partially reuse previous results when
/// the break index is safe-to-break per rustybuzz.
struct Line<'a> {
    /// Bidi information about the paragraph.
    bidi: &'a BidiInfo<'a>,
    /// The range the line spans in the paragraph.
    range: Range,
    /// A reshaped text item if the line sliced up a text item at the start.
    first: Option<ParItem<'a>>,
    /// Middle items which don't need to be reprocessed.
    items: &'a [ParItem<'a>],
    /// A reshaped text item if the line sliced up a text item at the end. If
    /// there is only one text item, this takes precedence over `first`.
    last: Option<ParItem<'a>>,
    /// The ranges, indexed as `[first, ..items, last]`. The ranges for `first`
    /// and `last` aren't trimmed to the line, but it doesn't matter because
    /// we're just checking which range an index falls into.
    ranges: &'a [Range],
    /// The size of the line.
    size: Size,
    /// The baseline of the line.
    baseline: Length,
    /// The sum of fractional ratios in the line.
    fr: Fractional,
    /// Whether the line ends at a mandatory break.
    mandatory: bool,
    /// Whether the line ends with a hyphen or dash, either naturally or through
    /// hyphenation.
    dash: bool,
}

impl<'a> Line<'a> {
    /// Iterate over the line's items.
    fn items(&self) -> impl Iterator<Item = &ParItem<'a>> {
        self.first.iter().chain(self.items).chain(&self.last)
    }

    /// Find the index of the item whose range contains the `text_offset`.
    fn find(&self, text_offset: usize) -> Option<usize> {
        self.ranges.binary_search_by(|r| r.locate(text_offset)).ok()
    }

    /// Get the item at the index.
    fn get(&self, index: usize) -> Option<&ParItem<'a>> {
        self.items().nth(index)
    }

    // How many justifiable glyphs the line contains.
    fn justifiables(&self) -> usize {
        let mut count = 0;
        for item in self.items() {
            if let ParItem::Text(shaped) = item {
                count += shaped.justifiables();
            }
        }
        count
    }

    /// How much of the line is stretchable spaces.
    fn stretch(&self) -> Length {
        let mut stretch = Length::zero();
        for item in self.items() {
            if let ParItem::Text(shaped) = item {
                stretch += shaped.stretch();
            }
        }
        stretch
    }
}

/// Prepare paragraph layout by shaping the whole paragraph and layouting all
/// contained inline-level nodes.
fn prepare<'a>(
    ctx: &mut Context,
    par: &'a ParNode,
    text: &'a str,
    regions: &Regions,
    styles: &'a StyleChain,
) -> TypResult<Preparation<'a>> {
    let bidi = BidiInfo::new(&text, match styles.get(ParNode::DIR) {
        Dir::LTR => Some(Level::ltr()),
        Dir::RTL => Some(Level::rtl()),
        _ => None,
    });

    let mut items = vec![];
    let mut ranges = vec![];

    // Layout the children and collect them into items.
    for (range, (child, map)) in par.ranges().zip(par.0.iter()) {
        let styles = map.chain(styles);
        match child {
            ParChild::Text(_) => {
                // TODO: Also split by language and script.
                let mut cursor = range.start;
                for (level, count) in bidi.levels[range].group() {
                    let start = cursor;
                    cursor += count;
                    let subrange = start .. cursor;
                    let text = &bidi.text[subrange.clone()];
                    let dir = if level.is_ltr() { Dir::LTR } else { Dir::RTL };
                    let shaped = shape(&mut ctx.fonts, text, styles, dir);
                    items.push(ParItem::Text(shaped));
                    ranges.push(subrange);
                }
            }
            ParChild::Spacing(spacing) => match *spacing {
                Spacing::Linear(v) => {
                    let resolved = v.resolve(regions.base.x);
                    items.push(ParItem::Absolute(resolved));
                    ranges.push(range);
                }
                Spacing::Fractional(v) => {
                    items.push(ParItem::Fractional(v));
                    ranges.push(range);
                }
            },
            ParChild::Node(node) => {
                // Prevent margin overhang in the inline node except if there's
                // just this one.
                let local;
                let styles = if par.0.len() != 1 {
                    local = StyleMap::with(TextNode::OVERHANG, false);
                    local.chain(&styles)
                } else {
                    styles
                };

                let size = Size::new(regions.first.x, regions.base.y);
                let pod = Regions::one(size, regions.base, Spec::splat(false));
                let frame = node.layout(ctx, &pod, styles)?.remove(0);
                items.push(ParItem::Frame(Arc::take(frame)));
                ranges.push(range);
            }
        }
    }

    Ok(Preparation { bidi, items, ranges })
}

/// Find suitable linebreaks.
fn linebreak<'a>(
    p: &'a Preparation<'a>,
    fonts: &mut FontStore,
    width: Length,
    styles: StyleChain,
) -> Vec<Line<'a>> {
    let breaks = styles.get(ParNode::LINEBREAKS).unwrap_or_else(|| {
        if styles.get(ParNode::JUSTIFY) {
            Linebreaks::Optimized
        } else {
            Linebreaks::Simple
        }
    });

    let breaker = match breaks {
        Linebreaks::Simple => linebreak_simple,
        Linebreaks::Optimized => linebreak_optimized,
    };

    breaker(p, fonts, width, styles)
}

/// Perform line breaking in simple first-fit style. This means that we build
/// lines a greedily, always taking the longest possible line. This may lead to
/// very unbalanced line, but is fast and simple.
fn linebreak_simple<'a>(
    p: &'a Preparation<'a>,
    fonts: &mut FontStore,
    width: Length,
    styles: StyleChain,
) -> Vec<Line<'a>> {
    let mut lines = vec![];
    let mut start = 0;
    let mut last = None;

    for (end, mandatory, hyphen) in breakpoints(&p.bidi.text, styles) {
        // Compute the line and its size.
        let mut attempt = line(p, fonts, start .. end, mandatory, hyphen);

        // If the line doesn't fit anymore, we push the last fitting attempt
        // into the stack and rebuild the line from its end. The resulting
        // line cannot be broken up further.
        if !width.fits(attempt.size.x) {
            if let Some((last_attempt, last_end)) = last.take() {
                lines.push(last_attempt);
                start = last_end;
                attempt = line(p, fonts, start .. end, mandatory, hyphen);
            }
        }

        // Finish the current line if there is a mandatory line break (i.e.
        // due to "\n") or if the line doesn't fit horizontally already
        // since then no shorter line will be possible.
        if mandatory || !width.fits(attempt.size.x) {
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
fn linebreak_optimized<'a>(
    p: &'a Preparation<'a>,
    fonts: &mut FontStore,
    width: Length,
    styles: StyleChain,
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
    const HYPH_COST: Cost = 0.5;
    const CONSECUTIVE_DASH_COST: Cost = 30.0;
    const MAX_COST: Cost = 1_000_000.0;
    const MIN_COST: Cost = -MAX_COST;
    const MIN_RATIO: f64 = -0.15;

    let em = styles.get(TextNode::SIZE);
    let justify = styles.get(ParNode::JUSTIFY);

    // Dynamic programming table.
    let mut active = 0;
    let mut table = vec![Entry {
        pred: 0,
        total: 0.0,
        line: line(p, fonts, 0 .. 0, false, false),
    }];

    for (end, mandatory, hyphen) in breakpoints(&p.bidi.text, styles) {
        let k = table.len();
        let eof = end == p.bidi.text.len();
        let mut best: Option<Entry> = None;

        // Find the optimal predecessor.
        for (i, pred) in table.iter_mut().enumerate().skip(active) {
            // Layout the line.
            let start = pred.line.range.end;
            let attempt = line(p, fonts, start .. end, mandatory, hyphen);

            // Determine how much the line's spaces would need to be stretched
            // to make it the desired width.
            let delta = width - attempt.size.x;
            let mut ratio = delta / attempt.stretch();
            if ratio.is_infinite() {
                ratio = delta / (em / 2.0);
            }

            // At some point, it doesn't matter any more.
            ratio = ratio.min(10.0);

            // Determine the cost of the line.
            let mut cost = if ratio < if justify { MIN_RATIO } else { 0.0 } {
                // The line is overfull. This is the case if
                // - justification is on, but we'd need to shrink to much
                // - justification is off and the line just doesn't fit
                // Since any longer line will also be overfull, we can deactive
                // this breakpoint.
                active = i + 1;
                MAX_COST
            } else if eof {
                // This is the final line and its not overfull since then
                // we would have taken the above branch.
                0.0
            } else if mandatory {
                // This is a mandatory break and the line is not overfull, so it
                // has minimum cost. All breakpoints before this one become
                // inactive since no line can span above the mandatory break.
                active = k;
                MIN_COST
            } else {
                // Normal line with cost of |ratio^3|.
                ratio.powi(3).abs()
            };

            // Penalize hyphens and especially two consecutive hyphens.
            if hyphen {
                cost += HYPH_COST;
            }
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

/// Determine all possible points in the text where lines can broken.
///
/// Returns for each breakpoint the text index, whether the break is mandatory
/// (after `\n`) and whether a hyphen is required (when breaking inside of a
/// word).
fn breakpoints<'a>(
    text: &'a str,
    styles: StyleChain,
) -> impl Iterator<Item = (usize, bool, bool)> + 'a {
    let mut lang = None;
    if styles.get(ParNode::HYPHENATE).unwrap_or(styles.get(ParNode::JUSTIFY)) {
        lang = styles
            .get(ParNode::LANG)
            .as_ref()
            .and_then(|iso| iso.as_bytes().try_into().ok())
            .and_then(hypher::Lang::from_iso);
    }

    let breaks = LineBreakIterator::new(text);
    let mut last = 0;

    if let Some(lang) = lang {
        Either::Left(breaks.flat_map(move |(end, mandatory)| {
            // We don't want to confuse the hyphenator with trailing
            // punctuation, so we trim it. And if that makes the word empty, we
            // need to return the single breakpoint manually because hypher
            // would eat it.
            let word = &text[last .. end];
            let trimmed = word.trim_end_matches(|c: char| !c.is_alphabetic());
            let suffix = last + trimmed.len();
            let mut start = std::mem::replace(&mut last, end);
            if trimmed.is_empty() {
                Either::Left([(end, mandatory, false)].into_iter())
            } else {
                Either::Right(hypher::hyphenate(trimmed, lang).map(move |syllable| {
                    start += syllable.len();
                    if start == suffix {
                        start = end;
                    }
                    let hyphen = start < end;
                    (start, mandatory && !hyphen, hyphen)
                }))
            }
        }))
    } else {
        Either::Right(breaks.map(|(e, m)| (e, m, false)))
    }
}

/// Create a line which spans the given range.
fn line<'a>(
    p: &'a Preparation,
    fonts: &mut FontStore,
    range: Range,
    mandatory: bool,
    hyphen: bool,
) -> Line<'a> {
    // Find the items which bound the text range.
    let last_idx = p.find(range.end.saturating_sub(1)).unwrap();
    let first_idx = if range.is_empty() {
        last_idx
    } else {
        p.find(range.start).unwrap()
    };

    // Slice out the relevant items.
    let mut items = &p.items[first_idx ..= last_idx];

    // Reshape the last item if it's split in half.
    let mut last = None;
    let mut dash = false;
    if let Some((ParItem::Text(shaped), before)) = items.split_last() {
        // Compute the range we want to shape, trimming whitespace at the
        // end of the line.
        let base = p.ranges[last_idx].start;
        let start = range.start.max(base);
        let trimmed = p.bidi.text[start .. range.end].trim_end();
        let shy = trimmed.ends_with('\u{ad}');
        dash = hyphen || shy || trimmed.ends_with(['-', '–', '—']);

        // Usually, we don't want to shape an empty string because:
        // - We don't want the height of trimmed whitespace in a different
        //   font to be considered for the line height.
        // - Even if it's in the same font, its unnecessary.
        //
        // There is one exception though. When the whole line is empty, we
        // need the shaped empty string to make the line the appropriate
        // height. That is the case exactly if the string is empty and there
        // are no other items in the line.
        if hyphen || trimmed.len() < shaped.text.len() {
            if hyphen || !trimmed.is_empty() || before.is_empty() {
                let end = start + trimmed.len();
                let shifted = start - base .. end - base;
                let mut reshaped = shaped.reshape(fonts, shifted);
                if hyphen || shy {
                    reshaped.push_hyphen(fonts);
                }
                last = Some(ParItem::Text(reshaped));
            }

            items = before;
        }
    }

    // Reshape the start item if it's split in half.
    let mut first = None;
    if let Some((ParItem::Text(shaped), after)) = items.split_first() {
        // Compute the range we want to shape.
        let Range { start: base, end: first_end } = p.ranges[first_idx];
        let start = range.start;
        let end = range.end.min(first_end);

        // Reshape if necessary.
        if end - start < shaped.text.len() {
            if start < end {
                let shifted = start - base .. end - base;
                let reshaped = shaped.reshape(fonts, shifted);
                first = Some(ParItem::Text(reshaped));
            }

            items = after;
        }
    }

    let mut width = Length::zero();
    let mut top = Length::zero();
    let mut bottom = Length::zero();
    let mut fr = Fractional::zero();

    // Measure the size of the line.
    for item in first.iter().chain(items).chain(&last) {
        match item {
            ParItem::Absolute(v) => width += *v,
            ParItem::Fractional(v) => fr += *v,
            ParItem::Text(shaped) => {
                width += shaped.size.x;
                top.set_max(shaped.baseline);
                bottom.set_max(shaped.size.y - shaped.baseline);
            }
            ParItem::Frame(frame) => {
                width += frame.size.x;
                top.set_max(frame.baseline());
                bottom.set_max(frame.size.y - frame.baseline());
            }
        }
    }

    Line {
        bidi: &p.bidi,
        range,
        first,
        items,
        last,
        ranges: &p.ranges[first_idx ..= last_idx],
        size: Size::new(width, top + bottom),
        baseline: top,
        fr,
        mandatory,
        dash,
    }
}

/// Combine layouted lines into one frame per region.
fn stack(
    lines: &[Line],
    fonts: &FontStore,
    regions: &Regions,
    styles: StyleChain,
) -> Vec<Arc<Frame>> {
    let em = styles.get(TextNode::SIZE);
    let leading = styles.get(ParNode::LEADING).resolve(em);
    let align = styles.get(ParNode::ALIGN);
    let justify = styles.get(ParNode::JUSTIFY);

    // Determine the paragraph's width: Full width of the region if we
    // should expand or there's fractional spacing, fit-to-width otherwise.
    let mut width = regions.first.x;
    if !regions.expand.x && lines.iter().all(|line| line.fr.is_zero()) {
        width = lines.iter().map(|line| line.size.x).max().unwrap_or_default();
    }

    // State for final frame building.
    let mut regions = regions.clone();
    let mut finished = vec![];
    let mut first = true;
    let mut output = Frame::new(Size::with_x(width));

    // Stack the lines into one frame per region.
    for line in lines {
        while !regions.first.y.fits(line.size.y) && !regions.in_last() {
            finished.push(Arc::new(output));
            output = Frame::new(Size::with_x(width));
            regions.next();
            first = true;
        }

        if !first {
            output.size.y += leading;
        }

        let frame = commit(line, fonts, width, align, justify);
        let pos = Point::with_y(output.size.y);
        output.size.y += frame.size.y;
        output.merge_frame(pos, frame);

        regions.first.y -= line.size.y + leading;
        first = false;
    }

    finished.push(Arc::new(output));
    finished
}

/// Commit to a line and build its frame.
fn commit(
    line: &Line,
    fonts: &FontStore,
    width: Length,
    align: Align,
    justify: bool,
) -> Frame {
    let size = Size::new(width, line.size.y);
    let mut remaining = width - line.size.x;
    let mut offset = Length::zero();

    // Reorder the line from logical to visual order.
    let reordered = reorder(line);

    // Handle hanging punctuation to the left.
    if let Some(ParItem::Text(text)) = reordered.first() {
        if let Some(glyph) = text.glyphs.first() {
            if text.styles.get(TextNode::OVERHANG) {
                let start = text.dir.is_positive();
                let em = text.styles.get(TextNode::SIZE);
                let amount = overhang(glyph.c, start) * glyph.x_advance.resolve(em);
                offset -= amount;
                remaining += amount;
            }
        }
    }

    // Handle hanging punctuation to the right.
    if let Some(ParItem::Text(text)) = reordered.last() {
        if let Some(glyph) = text.glyphs.last() {
            if text.styles.get(TextNode::OVERHANG)
                && (reordered.len() > 1 || text.glyphs.len() > 1)
            {
                let start = !text.dir.is_positive();
                let em = text.styles.get(TextNode::SIZE);
                let amount = overhang(glyph.c, start) * glyph.x_advance.resolve(em);
                remaining += amount;
            }
        }
    }

    // Determine how much to justify each space.
    let mut justification = Length::zero();
    if remaining < Length::zero()
        || (justify
            && !line.mandatory
            && line.range.end < line.bidi.text.len()
            && line.fr.is_zero())
    {
        let justifiables = line.justifiables();
        if justifiables > 0 {
            justification = remaining / justifiables as f64;
            remaining = Length::zero();
        }
    }

    let mut output = Frame::new(size);
    output.baseline = Some(line.baseline);

    // Construct the line's frame from left to right.
    for item in reordered {
        let mut position = |frame: Frame| {
            let x = offset + align.resolve(remaining);
            let y = line.baseline - frame.baseline();
            offset += frame.size.x;
            output.merge_frame(Point::new(x, y), frame);
        };

        match item {
            ParItem::Absolute(v) => offset += *v,
            ParItem::Fractional(v) => offset += v.resolve(line.fr, remaining),
            ParItem::Text(shaped) => position(shaped.build(fonts, justification)),
            ParItem::Frame(frame) => position(frame.clone()),
        }
    }

    output
}

/// Return a line's items in visual order.
fn reorder<'a>(line: &'a Line<'a>) -> Vec<&'a ParItem<'a>> {
    let mut reordered = vec![];

    // The bidi crate doesn't like empty lines.
    if line.range.is_empty() {
        return reordered;
    }

    // Find the paragraph that contains the line.
    let para = line
        .bidi
        .paragraphs
        .iter()
        .find(|para| para.range.contains(&line.range.start))
        .unwrap();

    // Compute the reordered ranges in visual order (left to right).
    let (levels, runs) = line.bidi.visual_runs(para, line.range.clone());

    // Collect the reordered items.
    for run in runs {
        let first_idx = line.find(run.start).unwrap();
        let last_idx = line.find(run.end - 1).unwrap();
        let range = first_idx ..= last_idx;

        // Provide the items forwards or backwards depending on the run's
        // direction.
        if levels[run.start].is_ltr() {
            reordered.extend(range.filter_map(|i| line.get(i)));
        } else {
            reordered.extend(range.rev().filter_map(|i| line.get(i)));
        }
    }

    reordered
}

/// How much a character should hang into the margin.
///
/// For selection of overhang characters, see also:
/// https://recoveringphysicist.com/21/
fn overhang(c: char, start: bool) -> f64 {
    match c {
        '“' | '”' | '„' | '‟' | '"' if start => 1.0,
        '‘' | '’' | '‚' | '‛' | '\'' if start => 1.0,

        '“' | '”' | '„' | '‟' | '"' if !start => 0.6,
        '‘' | '’' | '‚' | '‛' | '\'' if !start => 0.6,
        '–' | '—' if !start => 0.2,
        '-' if !start => 0.55,

        '.' | ',' => 0.8,
        ':' | ';' => 0.3,
        '«' | '»' => 0.2,
        '‹' | '›' => 0.4,

        // Arabic and Ideographic
        '\u{60C}' | '\u{6D4}' => 0.4,
        '\u{3001}' | '\u{3002}' => 1.0,

        _ => 0.0,
    }
}
