use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use itertools::Either;
use unicode_bidi::{BidiInfo, Level};
use xi_unicode::LineBreakIterator;

use super::prelude::*;
use super::{shape, Decoration, ShapedText, Spacing};
use crate::style::TextStyle;
use crate::util::{EcoString, RangeExt, RcExt, SliceExt};

/// `par`: Configure paragraphs.
pub fn par(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let spacing = args.named("spacing")?;
    let leading = args.named("leading")?;

    let mut dir = args.named::<EcoString>("lang")?.map(|iso| {
        match iso.to_ascii_lowercase().as_str() {
            "ar" | "he" | "fa" | "ur" | "ps" | "yi" => Dir::RTL,
            "en" | "fr" | "de" => Dir::LTR,
            _ => Dir::LTR,
        }
    });

    if let Some(Spanned { v, span }) = args.named::<Spanned<Dir>>("dir")? {
        if v.axis() == SpecAxis::Horizontal {
            dir = Some(v)
        } else {
            bail!(span, "must be horizontal");
        }
    }

    ctx.template.modify(move |style| {
        let par = style.par_mut();

        if let Some(dir) = dir {
            par.dir = dir;
        }

        if let Some(leading) = leading {
            par.leading = leading;
        }

        if let Some(spacing) = spacing {
            par.spacing = spacing;
        }
    });

    ctx.template.parbreak();

    Ok(Value::None)
}

/// A node that arranges its children into a paragraph.
#[derive(Debug, Hash)]
pub struct ParNode {
    /// The inline direction of this paragraph.
    pub dir: Dir,
    /// The spacing to insert between each line.
    pub leading: Length,
    /// The children to be arranged in a paragraph.
    pub children: Vec<ParChild>,
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
        let bidi = BidiInfo::new(&text, Level::from_dir(self.dir));

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
        self.children.iter().map(|child| match child {
            ParChild::Spacing(_) => " ",
            ParChild::Text(ref piece, ..) => piece,
            ParChild::Node(..) => "\u{FFFC}",
            ParChild::Decorate(_) | ParChild::Undecorate => "",
        })
    }
}

/// A child of a paragraph node.
#[derive(Hash)]
pub enum ParChild {
    /// Spacing between other nodes.
    Spacing(Spacing),
    /// A run of text and how to align it in its line.
    Text(EcoString, Align, Rc<TextStyle>),
    /// Any child node and how to align it in its line.
    Node(PackedNode, Align),
    /// A decoration that applies until a matching `Undecorate`.
    Decorate(Decoration),
    /// The end of a decoration.
    Undecorate,
}

impl Debug for ParChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(v) => write!(f, "Spacing({:?})", v),
            Self::Text(text, ..) => write!(f, "Text({:?})", text),
            Self::Node(node, ..) => node.fmt(f),
            Self::Decorate(deco) => write!(f, "Decorate({:?})", deco),
            Self::Undecorate => write!(f, "Undecorate"),
        }
    }
}

/// A paragraph representation in which children are already layouted and text
/// is separated into shapable runs.
struct ParLayouter<'a> {
    /// The top-level direction.
    dir: Dir,
    /// The line spacing.
    leading: Length,
    /// Bidirectional text embedding levels for the paragraph.
    bidi: BidiInfo<'a>,
    /// Spacing, separated text runs and layouted nodes.
    items: Vec<ParItem<'a>>,
    /// The ranges of the items in `bidi.text`.
    ranges: Vec<Range>,
    /// The decorations and the ranges they span.
    decos: Vec<(Range, &'a Decoration)>,
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
    Text(ShapedText<'a>, Align),
    /// A layouted child node.
    Frame(Frame, Align),
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
        let mut starts = vec![];
        let mut decos = vec![];

        // Layout the children and collect them into items.
        for (range, child) in par.ranges().zip(&par.children) {
            match *child {
                ParChild::Spacing(Spacing::Linear(v)) => {
                    let resolved = v.resolve(regions.current.w);
                    items.push(ParItem::Absolute(resolved));
                    ranges.push(range);
                }
                ParChild::Spacing(Spacing::Fractional(v)) => {
                    items.push(ParItem::Fractional(v));
                    ranges.push(range);
                }
                ParChild::Text(_, align, ref style) => {
                    // TODO: Also split by language and script.
                    let mut cursor = range.start;
                    for (level, group) in bidi.levels[range].group_by_key(|&lvl| lvl) {
                        let start = cursor;
                        cursor += group.len();
                        let subrange = start .. cursor;
                        let text = &bidi.text[subrange.clone()];
                        let shaped = shape(ctx, text, style, level.dir());
                        items.push(ParItem::Text(shaped, align));
                        ranges.push(subrange);
                    }
                }
                ParChild::Node(ref node, align) => {
                    let size = Size::new(regions.current.w, regions.base.h);
                    let expand = Spec::splat(false);
                    let pod = Regions::one(size, regions.base, expand);
                    let frame = node.layout(ctx, &pod).remove(0);
                    items.push(ParItem::Frame(Rc::take(frame.item), align));
                    ranges.push(range);
                }
                ParChild::Decorate(ref deco) => {
                    starts.push((range.start, deco));
                }
                ParChild::Undecorate => {
                    let (start, deco) = starts.pop().unwrap();
                    decos.push((start .. range.end, deco));
                }
            }
        }

        Self {
            dir: par.dir,
            leading: par.leading,
            bidi,
            items,
            ranges,
            decos,
        }
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
                    // Since the new line try did not fit, no region that would
                    // fit the line will yield the same line break. Therefore,
                    // the width of the region must not fit the width of the
                    // tried line.
                    if !stack.regions.current.w.fits(line.size.w) {
                        stack.cts.max.x.set_min(line.size.w);
                    }

                    // Same as above, but for height.
                    if !stack.regions.current.h.fits(line.size.h) {
                        let too_large = stack.size.h + self.leading + line.size.h;
                        stack.cts.max.y.set_min(too_large);
                    }

                    stack.push(last_line);

                    stack.cts.min.y = Some(stack.size.h);
                    start = last_end;
                    line = LineLayout::new(ctx, &self, start .. end);
                }
            }

            // If the line does not fit vertically, we start a new region.
            while !stack.regions.current.h.fits(line.size.h) {
                if stack.regions.in_full_last() {
                    stack.overflowing = true;
                    break;
                }

                // Again, the line must not fit. It would if the space taken up
                // plus the line height would fit, therefore the constraint
                // below.
                let too_large = stack.size.h + self.leading + line.size.h;
                stack.cts.max.y.set_min(too_large);

                stack.finish_region(ctx);
            }

            // If the line does not fit horizontally or we have a mandatory
            // line break (i.e. due to "\n"), we push the line into the
            // stack.
            if mandatory || !stack.regions.current.w.fits(line.size.w) {
                start = end;
                last = None;

                stack.push(line);

                // If there is a trailing line break at the end of the
                // paragraph, we want to force an empty line.
                if mandatory && end == self.bidi.text.len() {
                    let line = LineLayout::new(ctx, &self, end .. end);
                    if stack.regions.current.h.fits(line.size.h) {
                        stack.push(line);
                    }
                }

                stack.cts.min.y = Some(stack.size.h);
            } else {
                // Otherwise, the line fits both horizontally and vertically
                // and we remember it.
                stack.cts.min.x.set_max(line.size.w);
                last = Some((line, end));
            }
        }

        if let Some((line, _)) = last {
            stack.push(line);
            stack.cts.min.y = Some(stack.size.h);
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
        if let Some((ParItem::Text(shaped, align), rest)) = items.split_last() {
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
                    let reshaped = shaped.reshape(ctx, range);
                    last = Some(ParItem::Text(reshaped, *align));
                }

                items = rest;
                line.end = end;
            }
        }

        // Reshape the start item if it's split in half.
        let mut first = None;
        if let Some((ParItem::Text(shaped, align), rest)) = items.split_first() {
            // Compute the range we want to shape.
            let Range { start: base, end: first_end } = par.ranges[first_idx];
            let start = line.start;
            let end = line.end.min(first_end);
            let range = start - base .. end - base;

            // Reshape if necessary.
            if range.len() < shaped.text.len() {
                if !range.is_empty() {
                    let reshaped = shaped.reshape(ctx, range);
                    first = Some(ParItem::Text(reshaped, *align));
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
            match *item {
                ParItem::Absolute(v) => width += v,
                ParItem::Fractional(v) => fr += v,
                ParItem::Text(ShapedText { size, baseline, .. }, _)
                | ParItem::Frame(Frame { size, baseline, .. }, _) => {
                    width += size.w;
                    top.set_max(baseline);
                    bottom.set_max(size.h - baseline);
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
        let size = Size::new(self.size.w.max(width), self.size.h);
        let remaining = size.w - self.size.w;

        let mut output = Frame::new(size, self.baseline);
        let mut offset = Length::zero();
        let mut ruler = Align::Start;

        for (range, item) in self.reordered() {
            let mut position = |mut frame: Frame, align: Align| {
                // Decorate.
                for (deco_range, deco) in &self.par.decos {
                    if deco_range.contains(&range.start) {
                        deco.apply(ctx, &mut frame);
                    }
                }

                // FIXME: Ruler alignment for RTL.
                ruler = ruler.max(align);
                let x = ruler.resolve(self.par.dir, offset .. remaining + offset);
                let y = self.baseline - frame.baseline;
                offset += frame.size.w;

                // Add to the line's frame.
                output.merge_frame(Point::new(x, y), frame);
            };

            match *item {
                ParItem::Absolute(v) => offset += v,
                ParItem::Fractional(v) => {
                    let ratio = v / self.fr;
                    if remaining.is_finite() && ratio.is_finite() {
                        offset += ratio * remaining;
                    }
                }
                ParItem::Text(ref shaped, align) => position(shaped.build(), align),
                ParItem::Frame(ref frame, align) => position(frame.clone(), align),
            }
        }

        output
    }

    /// Iterate through the line's items in visual order.
    fn reordered(&self) -> impl Iterator<Item = (Range, &ParItem<'a>)> {
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
            <_>::default()
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
            .map(move |idx| (self.ranges[idx].clone(), self.get(idx).unwrap()))
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
        self.regions.current.h -= line.size.h + self.leading;

        self.size.w.set_max(line.size.w);
        self.size.h += line.size.h;
        if !self.lines.is_empty() {
            self.size.h += self.leading;
        }

        self.fractional |= !line.fr.is_zero();
        self.lines.push(line);
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self, ctx: &LayoutContext) {
        if self.regions.expand.x || self.fractional {
            self.size.w = self.regions.current.w;
            self.cts.exact.x = Some(self.regions.current.w);
        }

        if self.overflowing {
            self.cts.min.y = None;
            self.cts.max.y = None;
            self.cts.exact = self.full.to_spec().map(Some);
        }

        let mut output = Frame::new(self.size, self.size.h);
        let mut offset = Length::zero();
        let mut first = true;

        for line in self.lines.drain(..) {
            let frame = line.build(ctx, self.size.w);

            let pos = Point::new(Length::zero(), offset);
            if first {
                output.baseline = pos.y + frame.baseline;
                first = false;
            }

            offset += frame.size.h + self.leading;
            output.merge_frame(pos, frame);
        }

        self.finished.push(output.constrain(self.cts));
        self.regions.next();
        self.full = self.regions.current;
        self.cts = Constraints::new(self.regions.expand);
        self.size = Size::zero();
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
