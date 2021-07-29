use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use unicode_bidi::{BidiInfo, Level};
use xi_unicode::LineBreakIterator;

use super::*;
use crate::exec::FontState;
use crate::util::{EcoString, RangeExt, SliceExt};

type Range = std::ops::Range<usize>;

/// A node that arranges its children into a paragraph.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub struct ParNode {
    /// The inline direction of this paragraph.
    pub dir: Dir,
    /// The spacing to insert between each line.
    pub line_spacing: Length,
    /// The nodes to be arranged in a paragraph.
    pub children: Vec<ParChild>,
}

/// A child of a paragraph node.
#[derive(Clone, Eq, PartialEq)]
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub enum ParChild {
    /// Spacing between other nodes.
    Spacing(Length),
    /// A run of text and how to align it in its line.
    Text(EcoString, Align, Rc<FontState>),
    /// Any child node and how to align it in its line.
    Any(LayoutNode, Align),
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
            ParChild::Text(ref piece, _, _) => piece,
            ParChild::Any(_, _) => "\u{FFFC}",
        })
    }
}

impl From<ParNode> for LayoutNode {
    fn from(par: ParNode) -> Self {
        Self::new(par)
    }
}

impl Debug for ParChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(amount) => write!(f, "Spacing({:?})", amount),
            Self::Text(text, align, _) => write!(f, "Text({:?}, {:?})", text, align),
            Self::Any(any, align) => {
                f.debug_tuple("Any").field(any).field(align).finish()
            }
        }
    }
}

/// A paragraph representation in which children are already layouted and text
/// is separated into shapable runs.
struct ParLayouter<'a> {
    /// The top-level direction.
    dir: Dir,
    /// The line spacing.
    line_spacing: Length,
    /// Bidirectional text embedding levels for the paragraph.
    bidi: BidiInfo<'a>,
    /// Layouted children and separated text runs.
    items: Vec<ParItem<'a>>,
    /// The ranges of the items in `bidi.text`.
    ranges: Vec<Range>,
}

impl<'a> ParLayouter<'a> {
    /// Prepare initial shaped text and layouted children.
    fn new(
        par: &'a ParNode,
        ctx: &mut LayoutContext,
        regions: &Regions,
        bidi: BidiInfo<'a>,
    ) -> Self {
        // Prepare an iterator over each child an the range it spans.
        let mut items = vec![];
        let mut ranges = vec![];

        // Layout the children and collect them into items.
        for (range, child) in par.ranges().zip(&par.children) {
            match *child {
                ParChild::Spacing(amount) => {
                    items.push(ParItem::Spacing(amount));
                    ranges.push(range);
                }
                ParChild::Text(_, align, ref state) => {
                    // TODO: Also split by language and script.
                    for (subrange, dir) in split_runs(&bidi, range) {
                        let text = &bidi.text[subrange.clone()];
                        let shaped = shape(ctx, text, dir, state);
                        items.push(ParItem::Text(shaped, align));
                        ranges.push(subrange);
                    }
                }
                ParChild::Any(ref node, align) => {
                    let frame = node.layout(ctx, regions).remove(0);
                    items.push(ParItem::Frame(frame.item, align));
                    ranges.push(range);
                }
            }
        }

        Self {
            dir: par.dir,
            line_spacing: par.line_spacing,
            bidi,
            items,
            ranges,
        }
    }

    /// Find first-fit line breaks and build the paragraph.
    fn layout(
        self,
        ctx: &mut LayoutContext,
        regions: Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut stack = LineStack::new(self.line_spacing, regions);

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
                    // The region must not fit this line for the result to be valid.
                    if !stack.regions.current.width.fits(line.size.width) {
                        stack.constraints.max.horizontal.set_min(line.size.width);
                    }

                    if !stack.regions.current.height.fits(line.size.height) {
                        stack
                            .constraints
                            .max
                            .vertical
                            .set_min(stack.size.height + line.size.height);
                    }

                    stack.push(last_line);
                    stack.constraints.min.vertical = Some(stack.size.height);
                    start = last_end;
                    line = LineLayout::new(ctx, &self, start .. end);
                }
            }

            // If the line does not fit vertically, we start a new region.
            while !stack.regions.current.height.fits(line.size.height)
                && !stack.regions.in_full_last()
            {
                // Again, the line must not fit. It would if the space taken up
                // plus the line height would fit, therefore the constraint
                // below.
                stack
                    .constraints
                    .max
                    .vertical
                    .set_min(stack.size.height + line.size.height);
                stack.finish_region(ctx);
            }

            // If the line does not fit vertically, we start a new region.
            while !stack.regions.current.height.fits(line.size.height) {
                if stack.regions.in_full_last() {
                    stack.overflowing = true;
                    break;
                }

                stack
                    .constraints
                    .max
                    .vertical
                    .set_min(stack.size.height + line.size.height);
                stack.finish_region(ctx);
            }
            // If the line does not fit horizontally or we have a mandatory
            // line break (i.e. due to "\n"), we push the line into the
            // stack.
            if mandatory || !stack.regions.current.width.fits(line.size.width) {
                stack.push(line);
                start = end;
                last = None;

                stack.constraints.min.vertical = Some(stack.size.height);

                // If there is a trailing line break at the end of the
                // paragraph, we want to force an empty line.
                if mandatory && end == self.bidi.text.len() {
                    stack.push(LineLayout::new(ctx, &self, end .. end));
                    stack.constraints.min.vertical = Some(stack.size.height);
                }
            } else {
                // Otherwise, the line fits both horizontally and vertically
                // and we remember it.
                stack.constraints.min.horizontal.set_max(line.size.width);
                last = Some((line, end));
            }
        }

        if let Some((line, _)) = last {
            stack.push(line);
            stack.constraints.min.vertical = Some(stack.size.height);
        }

        stack.finish(ctx)
    }

    /// Find the index of the item whose range contains the `text_offset`.
    fn find(&self, text_offset: usize) -> Option<usize> {
        self.ranges.binary_search_by(|r| r.locate(text_offset)).ok()
    }
}

/// Split a range of text into runs of consistent direction.
fn split_runs<'a>(
    bidi: &'a BidiInfo,
    range: Range,
) -> impl Iterator<Item = (Range, Dir)> + 'a {
    let mut cursor = range.start;
    bidi.levels[range]
        .group_by_key(|&level| level)
        .map(move |(level, group)| {
            let start = cursor;
            cursor += group.len();
            (start .. cursor, level.dir())
        })
}

/// A prepared item in a paragraph layout.
enum ParItem<'a> {
    /// Spacing between other items.
    Spacing(Length),
    /// A shaped text run with consistent direction.
    Text(ShapedText<'a>, Align),
    /// A layouted child node.
    Frame(Rc<Frame>, Align),
}

impl ParItem<'_> {
    /// The size of the item.
    pub fn size(&self) -> Size {
        match self {
            Self::Spacing(amount) => Size::new(*amount, Length::zero()),
            Self::Text(shaped, _) => shaped.size,
            Self::Frame(frame, _) => frame.size,
        }
    }

    /// The baseline of the item.
    pub fn baseline(&self) -> Length {
        match self {
            Self::Spacing(_) => Length::zero(),
            Self::Text(shaped, _) => shaped.baseline,
            Self::Frame(frame, _) => frame.baseline,
        }
    }
}

/// Stacks lines on top of each other.
struct LineStack<'a> {
    line_spacing: Length,
    full: Size,
    regions: Regions,
    size: Size,
    lines: Vec<LineLayout<'a>>,
    finished: Vec<Constrained<Rc<Frame>>>,
    constraints: Constraints,
    overflowing: bool,
}

impl<'a> LineStack<'a> {
    /// Create an empty line stack.
    fn new(line_spacing: Length, regions: Regions) -> Self {
        Self {
            line_spacing,
            constraints: Constraints::new(regions.expand),
            full: regions.current,
            regions,
            size: Size::zero(),
            lines: vec![],
            finished: vec![],
            overflowing: false,
        }
    }

    /// Push a new line into the stack.
    fn push(&mut self, line: LineLayout<'a>) {
        self.regions.current.height -= line.size.height + self.line_spacing;

        self.size.width.set_max(line.size.width);
        self.size.height += line.size.height;
        if !self.lines.is_empty() {
            self.size.height += self.line_spacing;
        }

        self.lines.push(line);
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self, ctx: &LayoutContext) {
        if self.regions.expand.horizontal {
            self.size.width = self.regions.current.width;
            self.constraints.exact.horizontal = Some(self.regions.current.width);
        }

        if self.overflowing {
            self.constraints.min.vertical = None;
            self.constraints.max.vertical = None;
            self.constraints.exact = self.full.to_spec().map(Some);
        }

        let mut output = Frame::new(self.size, self.size.height);
        let mut offset = Length::zero();
        let mut first = true;

        for line in self.lines.drain(..) {
            let frame = line.build(ctx, self.size.width);

            let pos = Point::new(Length::zero(), offset);
            if first {
                output.baseline = pos.y + frame.baseline;
                first = false;
            }

            offset += frame.size.height + self.line_spacing;
            output.merge_frame(pos, frame);
        }

        self.finished.push(output.constrain(self.constraints));
        self.regions.next();
        self.full = self.regions.current;
        self.constraints = Constraints::new(self.regions.expand);
        self.size = Size::zero();
    }

    /// Finish the last region and return the built frames.
    fn finish(mut self, ctx: &LayoutContext) -> Vec<Constrained<Rc<Frame>>> {
        self.finish_region(ctx);
        self.finished
    }
}

/// A lightweight representation of a line that spans a specific range in a
/// paragraph's text. This type enables you to cheaply measure the size of a
/// line in a range before comitting to building the line's frame.
struct LineLayout<'a> {
    /// The direction of the line.
    dir: Dir,
    /// Bidi information about the paragraph.
    bidi: &'a BidiInfo<'a>,
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

        // Measure the size of the line.
        for item in first.iter().chain(items).chain(&last) {
            let size = item.size();
            let baseline = item.baseline();
            width += size.width;
            top.set_max(baseline);
            bottom.set_max(size.height - baseline);
        }

        Self {
            dir: par.dir,
            bidi: &par.bidi,
            line,
            first,
            items,
            last,
            ranges,
            size: Size::new(width, top + bottom),
            baseline: top,
        }
    }

    /// Build the line's frame.
    fn build(&self, ctx: &LayoutContext, width: Length) -> Frame {
        let size = Size::new(self.size.width.max(width), self.size.height);
        let free = size.width - self.size.width;

        let mut output = Frame::new(size, self.baseline);
        let mut offset = Length::zero();
        let mut ruler = Align::Start;

        self.reordered(|item| {
            let frame = match *item {
                ParItem::Spacing(amount) => {
                    offset += amount;
                    return;
                }
                ParItem::Text(ref shaped, align) => {
                    ruler = ruler.max(align);
                    Rc::new(shaped.build(ctx))
                }
                ParItem::Frame(ref frame, align) => {
                    ruler = ruler.max(align);
                    frame.clone()
                }
            };

            // FIXME: Ruler alignment for RTL.
            let pos = Point::new(
                ruler.resolve(self.dir, offset .. free + offset),
                self.baseline - frame.baseline,
            );

            offset += frame.size.width;
            output.push_frame(pos, frame);
        });

        output
    }

    /// Iterate through the line's items in visual order.
    fn reordered(&self, mut f: impl FnMut(&ParItem<'a>)) {
        // The bidi crate doesn't like empty lines.
        if self.line.is_empty() {
            return;
        }

        // Find the paragraph that contains the line.
        let para = self
            .bidi
            .paragraphs
            .iter()
            .find(|para| para.range.contains(&self.line.start))
            .unwrap();

        // Compute the reordered ranges in visual order (left to right).
        let (levels, runs) = self.bidi.visual_runs(para, self.line.clone());

        // Find the items for each run.
        for run in runs {
            let first_idx = self.find(run.start).unwrap();
            let last_idx = self.find(run.end - 1).unwrap();
            let range = first_idx ..= last_idx;

            // Provide the items forwards or backwards depending on the run's
            // direction.
            if levels[run.start].is_ltr() {
                for item in range {
                    f(self.get(item).unwrap());
                }
            } else {
                for item in range.rev() {
                    f(self.get(item).unwrap());
                }
            }
        }
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
