use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use itertools::Either;
use unicode_bidi::{BidiInfo, Level};
use xi_unicode::LineBreakIterator;

use super::prelude::*;
use super::{shape, ShapedText, SpacingKind, SpacingNode, TextNode};
use crate::util::{EcoString, RangeExt, RcExt, SliceExt};

/// `parbreak`: Start a new paragraph.
pub fn parbreak(_: &mut EvalContext, _: &mut Args) -> TypResult<Value> {
    Ok(Value::Node(Node::Parbreak))
}

/// `linebreak`: Start a new line.
pub fn linebreak(_: &mut EvalContext, _: &mut Args) -> TypResult<Value> {
    Ok(Value::Node(Node::Linebreak))
}

/// A node that arranges its children into a paragraph.
#[derive(Hash)]
pub struct ParNode(pub Vec<ParChild>);

#[properties]
impl ParNode {
    /// The direction for text and inline objects.
    pub const DIR: Dir = Dir::LTR;
    /// How to align text and inline objects in their line.
    pub const ALIGN: Align = Align::Left;
    /// The spacing between lines (dependent on scaled font size).
    pub const LEADING: Linear = Relative::new(0.65).into();
    /// The spacing between paragraphs (dependent on scaled font size).
    pub const SPACING: Linear = Relative::new(1.2).into();
}

impl Construct for ParNode {
    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        // Lift to a block so that it doesn't merge with adjacent stuff.
        Ok(Node::Block(args.expect::<Node>("body")?.into_block()))
    }
}

impl Set for ParNode {
    fn set(styles: &mut Styles, args: &mut Args) -> TypResult<()> {
        let spacing = args.named("spacing")?;
        let leading = args.named("leading")?;

        let mut dir =
            args.named("lang")?
                .map(|iso: EcoString| match iso.to_lowercase().as_str() {
                    "ar" | "he" | "fa" | "ur" | "ps" | "yi" => Dir::RTL,
                    "en" | "fr" | "de" => Dir::LTR,
                    _ => Dir::LTR,
                });

        if let Some(Spanned { v, span }) = args.named::<Spanned<Dir>>("dir")? {
            if v.axis() != SpecAxis::Horizontal {
                bail!(span, "must be horizontal");
            }
            dir = Some(v);
        }

        let mut align = None;
        if let Some(Spanned { v, span }) = args.named::<Spanned<Align>>("align")? {
            if v.axis() != SpecAxis::Horizontal {
                bail!(span, "must be horizontal");
            }
            align = Some(v);
        }

        if let (Some(dir), None) = (dir, align) {
            align = Some(if dir == Dir::LTR { Align::Left } else { Align::Right });
        }

        styles.set_opt(Self::DIR, dir);
        styles.set_opt(Self::ALIGN, align);
        styles.set_opt(Self::LEADING, leading);
        styles.set_opt(Self::SPACING, spacing);

        Ok(())
    }
}

impl Layout for ParNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        // Collect all text into one string used for BiDi analysis.
        let text = self.collect_text();

        // Find out the BiDi embedding levels.
        let bidi = BidiInfo::new(&text, Level::from_dir(ctx.styles.get(Self::DIR)));

        // Prepare paragraph layout by building a representation on which we can
        // do line breaking without layouting each and every line from scratch.
        let layouter = ParLayouter::new(self, ctx, regions, bidi);

        // Find suitable linebreaks.
        layouter.layout(ctx, regions.clone())
    }
}

impl ParNode {
    /// Concatenate all text in the paragraph into one string, replacing spacing
    /// with a space character and other non-text nodes with the object
    /// replacement character. Returns the full text alongside the range each
    /// child spans in the text.
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
        self.0.iter().map(|child| match child {
            ParChild::Spacing(_) => " ",
            ParChild::Text(ref node) => &node.text,
            ParChild::Node(_) => "\u{FFFC}",
        })
    }
}

impl Debug for ParNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Par ")?;
        f.debug_list().entries(&self.0).finish()
    }
}

/// A child of a paragraph node.
#[derive(Hash)]
pub enum ParChild {
    /// Spacing between other nodes.
    Spacing(SpacingNode),
    /// A run of text and how to align it in its line.
    Text(TextNode),
    /// Any child node and how to align it in its line.
    Node(PackedNode),
}

impl ParChild {
    /// Create a text child.
    pub fn text(text: impl Into<EcoString>, styles: Styles) -> Self {
        Self::Text(TextNode { text: text.into(), styles })
    }

    /// A reference to the child's styles.
    pub fn styles(&self) -> &Styles {
        match self {
            Self::Spacing(node) => &node.styles,
            Self::Text(node) => &node.styles,
            Self::Node(node) => &node.styles,
        }
    }

    /// A mutable reference to the child's styles.
    pub fn styles_mut(&mut self) -> &mut Styles {
        match self {
            Self::Spacing(node) => &mut node.styles,
            Self::Text(node) => &mut node.styles,
            Self::Node(node) => &mut node.styles,
        }
    }
}

impl Debug for ParChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(node) => node.fmt(f),
            Self::Text(node) => node.fmt(f),
            Self::Node(node) => node.fmt(f),
        }
    }
}

/// A paragraph representation in which children are already layouted and text
/// is separated into shapable runs.
struct ParLayouter<'a> {
    /// How to align text in its line.
    align: Align,
    /// The spacing to insert between each line.
    leading: Length,
    /// Bidirectional text embedding levels for the paragraph.
    bidi: BidiInfo<'a>,
    /// Spacing, separated text runs and layouted nodes.
    items: Vec<ParItem<'a>>,
    /// The ranges of the items in `bidi.text`.
    ranges: Vec<Range>,
}

/// Range of a substring of text.
type Range = std::ops::Range<usize>;

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

impl<'a> ParLayouter<'a> {
    /// Prepare initial shaped text and layouted children.
    fn new(
        par: &'a ParNode,
        ctx: &mut LayoutContext,
        regions: &Regions,
        bidi: BidiInfo<'a>,
    ) -> Self {
        let mut items = vec![];
        let mut ranges = vec![];

        // Layout the children and collect them into items.
        for (range, child) in par.ranges().zip(&par.0) {
            match child {
                ParChild::Spacing(node) => match node.kind {
                    SpacingKind::Linear(v) => {
                        let resolved = v.resolve(regions.current.x);
                        items.push(ParItem::Absolute(resolved));
                        ranges.push(range);
                    }
                    SpacingKind::Fractional(v) => {
                        items.push(ParItem::Fractional(v));
                        ranges.push(range);
                    }
                },
                ParChild::Text(node) => {
                    // TODO: Also split by language and script.
                    let mut cursor = range.start;
                    for (level, group) in bidi.levels[range].group_by_key(|&lvl| lvl) {
                        let start = cursor;
                        cursor += group.len();
                        let subrange = start .. cursor;
                        let text = &bidi.text[subrange.clone()];
                        let styles = node.styles.chain(&ctx.styles);
                        let shaped = shape(&mut ctx.fonts, text, styles, level.dir());
                        items.push(ParItem::Text(shaped));
                        ranges.push(subrange);
                    }
                }
                ParChild::Node(node) => {
                    let size = Size::new(regions.current.x, regions.base.y);
                    let pod = Regions::one(size, regions.base, Spec::splat(false));
                    let frame = node.layout(ctx, &pod).remove(0);
                    items.push(ParItem::Frame(Rc::take(frame.item)));
                    ranges.push(range);
                }
            }
        }

        let em = ctx.styles.get(TextNode::SIZE).abs;
        let align = ctx.styles.get(ParNode::ALIGN);
        let leading = ctx.styles.get(ParNode::LEADING).resolve(em);

        Self { align, leading, bidi, items, ranges }
    }

    /// Find first-fit line breaks and build the paragraph.
    fn layout(
        self,
        ctx: &mut LayoutContext,
        regions: Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut stack = LineStack::new(self.leading, regions);

        // The current line attempt.
        // Invariant: Always fits into `stack.regions.current`.
        let mut last = None;

        // The start of the line in `last`.
        let mut start = 0;

        // Find suitable line breaks.
        // TODO: Provide line break opportunities on alignment changes.
        for (end, mandatory) in LineBreakIterator::new(self.bidi.text) {
            // Compute the line and its size.
            let mut line = LineLayout::new(ctx, &self, start .. end);

            // If the line doesn't fit anymore, we push the last fitting attempt
            // into the stack and rebuild the line from its end. The resulting
            // line cannot be broken up further.
            if !stack.regions.current.fits(line.size) {
                if let Some((last_line, last_end)) = last.take() {
                    let fits =
                        stack.regions.current.zip(line.size).map(|(c, s)| c.fits(s));

                    // Since the new line try did not fit, no region that would
                    // fit the line will yield the same line break. Therefore,
                    // the width of the region must not fit the width of the
                    // tried line.
                    if !fits.x {
                        stack.cts.max.x.set_min(line.size.x);
                    }

                    // Same as above, but for height.
                    if !fits.y {
                        let too_large = stack.size.y + self.leading + line.size.y;
                        stack.cts.max.y.set_min(too_large);
                    }

                    // Don't start new lines at every opportunity when we are
                    // overflowing.
                    if !stack.overflowing || !fits.x {
                        stack.push(last_line);
                        stack.cts.min.y = Some(stack.size.y);
                        start = last_end;
                        line = LineLayout::new(ctx, &self, start .. end);
                    }
                }
            }

            // If the line does not fit vertically, we start a new region.
            while !stack.regions.current.y.fits(line.size.y) {
                if stack.regions.in_last() {
                    stack.overflowing = true;
                    break;
                }

                // Again, the line must not fit. It would if the space taken up
                // plus the line height would fit, therefore the constraint
                // below.
                let too_large = stack.size.y + self.leading + line.size.y;
                stack.cts.max.y.set_min(too_large);
                stack.finish_region(ctx);
            }

            // If the line does not fit horizontally or we have a mandatory
            // line break (i.e. due to "\n"), we push the line into the
            // stack.
            if mandatory || !stack.regions.current.x.fits(line.size.x) {
                start = end;
                last = None;

                stack.push(line);

                // If there is a trailing line break at the end of the
                // paragraph, we want to force an empty line.
                if mandatory && end == self.bidi.text.len() {
                    let line = LineLayout::new(ctx, &self, end .. end);
                    if stack.regions.current.y.fits(line.size.y) {
                        stack.push(line);
                    }
                }

                stack.cts.min.y = Some(stack.size.y);
            } else {
                // Otherwise, the line fits both horizontally and vertically
                // and we remember it.
                stack.cts.min.x.set_max(line.size.x);
                last = Some((line, end));
            }
        }

        if let Some((line, _)) = last {
            stack.push(line);
            stack.cts.min.y = Some(stack.size.y);
        }

        stack.finish(ctx)
    }

    /// Find the index of the item whose range contains the `text_offset`.
    fn find(&self, text_offset: usize) -> Option<usize> {
        self.ranges.binary_search_by(|r| r.locate(text_offset)).ok()
    }
}

/// A lightweight representation of a line that spans a specific range in a
/// paragraph's text. This type enables you to cheaply measure the size of a
/// line in a range before comitting to building the line's frame.
struct LineLayout<'a> {
    /// Bidi information about the paragraph.
    par: &'a ParLayouter<'a>,
    /// The range the line spans in the paragraph.
    line: Range,
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
}

impl<'a> LineLayout<'a> {
    /// Create a line which spans the given range.
    fn new(ctx: &mut LayoutContext, par: &'a ParLayouter<'a>, mut line: Range) -> Self {
        // Find the items which bound the text range.
        let last_idx = par.find(line.end.saturating_sub(1)).unwrap();
        let first_idx = if line.is_empty() {
            last_idx
        } else {
            par.find(line.start).unwrap()
        };

        // Slice out the relevant items and ranges.
        let mut items = &par.items[first_idx ..= last_idx];
        let ranges = &par.ranges[first_idx ..= last_idx];

        // Reshape the last item if it's split in half.
        let mut last = None;
        if let Some((ParItem::Text(shaped), rest)) = items.split_last() {
            // Compute the range we want to shape, trimming whitespace at the
            // end of the line.
            let base = par.ranges[last_idx].start;
            let start = line.start.max(base);
            let end = start + par.bidi.text[start .. line.end].trim_end().len();
            let range = start - base .. end - base;

            // Reshape if necessary.
            if range.len() < shaped.text.len() {
                // If start == end and the rest is empty, then we have an empty
                // line. To make that line have the appropriate height, we shape the
                // empty string.
                if !range.is_empty() || rest.is_empty() {
                    // Reshape that part.
                    let reshaped = shaped.reshape(&mut ctx.fonts, range);
                    last = Some(ParItem::Text(reshaped));
                }

                items = rest;
                line.end = end;
            }
        }

        // Reshape the start item if it's split in half.
        let mut first = None;
        if let Some((ParItem::Text(shaped), rest)) = items.split_first() {
            // Compute the range we want to shape.
            let Range { start: base, end: first_end } = par.ranges[first_idx];
            let start = line.start;
            let end = line.end.min(first_end);
            let range = start - base .. end - base;

            // Reshape if necessary.
            if range.len() < shaped.text.len() {
                if !range.is_empty() {
                    let reshaped = shaped.reshape(&mut ctx.fonts, range);
                    first = Some(ParItem::Text(reshaped));
                }

                items = rest;
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

        Self {
            par,
            line,
            first,
            items,
            last,
            ranges,
            size: Size::new(width, top + bottom),
            baseline: top,
            fr,
        }
    }

    /// Build the line's frame.
    fn build(&self, ctx: &LayoutContext, width: Length) -> Frame {
        let size = Size::new(self.size.x.max(width), self.size.y);
        let remaining = size.x - self.size.x;

        let mut offset = Length::zero();
        let mut output = Frame::new(size);
        output.baseline = Some(self.baseline);

        for item in self.reordered() {
            let mut position = |frame: Frame| {
                let x = offset + self.par.align.resolve(remaining);
                let y = self.baseline - frame.baseline();
                offset += frame.size.x;
                output.merge_frame(Point::new(x, y), frame);
            };

            match item {
                ParItem::Absolute(v) => offset += *v,
                ParItem::Fractional(v) => offset += v.resolve(self.fr, remaining),
                ParItem::Text(shaped) => position(shaped.build(&ctx.fonts)),
                ParItem::Frame(frame) => position(frame.clone()),
            }
        }

        output
    }

    /// Iterate through the line's items in visual order.
    fn reordered(&self) -> impl Iterator<Item = &ParItem<'a>> {
        // The bidi crate doesn't like empty lines.
        let (levels, runs) = if !self.line.is_empty() {
            // Find the paragraph that contains the line.
            let para = self
                .par
                .bidi
                .paragraphs
                .iter()
                .find(|para| para.range.contains(&self.line.start))
                .unwrap();

            // Compute the reordered ranges in visual order (left to right).
            self.par.bidi.visual_runs(para, self.line.clone())
        } else {
            (vec![], vec![])
        };

        runs.into_iter()
            .flat_map(move |run| {
                let first_idx = self.find(run.start).unwrap();
                let last_idx = self.find(run.end - 1).unwrap();
                let range = first_idx ..= last_idx;

                // Provide the items forwards or backwards depending on the run's
                // direction.
                if levels[run.start].is_ltr() {
                    Either::Left(range)
                } else {
                    Either::Right(range.rev())
                }
            })
            .map(move |idx| self.get(idx).unwrap())
    }

    /// Find the index of the item whose range contains the `text_offset`.
    fn find(&self, text_offset: usize) -> Option<usize> {
        self.ranges.binary_search_by(|r| r.locate(text_offset)).ok()
    }

    /// Get the item at the index.
    fn get(&self, index: usize) -> Option<&ParItem<'a>> {
        self.first.iter().chain(self.items).chain(&self.last).nth(index)
    }
}

/// Stacks lines on top of each other.
struct LineStack<'a> {
    leading: Length,
    full: Size,
    regions: Regions,
    size: Size,
    lines: Vec<LineLayout<'a>>,
    finished: Vec<Constrained<Rc<Frame>>>,
    cts: Constraints,
    overflowing: bool,
    fractional: bool,
}

impl<'a> LineStack<'a> {
    /// Create an empty line stack.
    fn new(leading: Length, regions: Regions) -> Self {
        Self {
            leading,
            full: regions.current,
            cts: Constraints::new(regions.expand),
            regions,
            size: Size::zero(),
            lines: vec![],
            finished: vec![],
            overflowing: false,
            fractional: false,
        }
    }

    /// Push a new line into the stack.
    fn push(&mut self, line: LineLayout<'a>) {
        self.regions.current.y -= line.size.y + self.leading;

        self.size.x.set_max(line.size.x);
        self.size.y += line.size.y;
        if !self.lines.is_empty() {
            self.size.y += self.leading;
        }

        self.fractional |= !line.fr.is_zero();
        self.lines.push(line);
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self, ctx: &LayoutContext) {
        if self.regions.expand.x || self.fractional {
            self.size.x = self.regions.current.x;
            self.cts.exact.x = Some(self.regions.current.x);
        }

        if self.overflowing {
            self.cts.min.y = None;
            self.cts.max.y = None;
            self.cts.exact = self.full.map(Some);
        }

        let mut output = Frame::new(self.size);
        let mut offset = Length::zero();

        for line in self.lines.drain(..) {
            let frame = line.build(ctx, self.size.x);
            let pos = Point::with_y(offset);
            offset += frame.size.y + self.leading;
            output.merge_frame(pos, frame);
        }

        self.cts.base = self.regions.base.map(Some);
        self.finished.push(output.constrain(self.cts));
        self.regions.next();
        self.full = self.regions.current;
        self.size = Size::zero();
        self.cts = Constraints::new(self.regions.expand);
    }

    /// Finish the last region and return the built frames.
    fn finish(mut self, ctx: &LayoutContext) -> Vec<Constrained<Rc<Frame>>> {
        self.finish_region(ctx);
        self.finished
    }
}

/// Additional methods for BiDi levels.
trait LevelExt: Sized {
    fn from_dir(dir: Dir) -> Option<Self>;
    fn dir(self) -> Dir;
}

impl LevelExt for Level {
    fn from_dir(dir: Dir) -> Option<Self> {
        match dir {
            Dir::LTR => Some(Level::ltr()),
            Dir::RTL => Some(Level::rtl()),
            _ => None,
        }
    }

    fn dir(self) -> Dir {
        if self.is_ltr() { Dir::LTR } else { Dir::RTL }
    }
}
