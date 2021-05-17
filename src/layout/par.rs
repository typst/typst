use std::fmt::{self, Debug, Formatter};
use std::mem;

use unicode_bidi::{BidiInfo, Level};
use xi_unicode::LineBreakIterator;

use super::*;
use crate::exec::FontProps;
use crate::util::{RangeExt, SliceExt};

type Range = std::ops::Range<usize>;

/// A node that arranges its children into a paragraph.
#[derive(Debug, Clone, PartialEq)]
pub struct ParNode {
    /// The inline direction of this paragraph.
    pub dir: Dir,
    /// The spacing to insert between each line.
    pub line_spacing: Length,
    /// The nodes to be arranged in a paragraph.
    pub children: Vec<ParChild>,
}

/// A child of a paragraph node.
#[derive(Clone, PartialEq)]
pub enum ParChild {
    /// Spacing between other nodes.
    Spacing(Length),
    /// A run of text and how to align it in its line.
    Text(String, FontProps, Align),
    /// Any child node and how to align it in its line.
    Any(AnyNode, Align),
}

impl Layout for ParNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Frame> {
        // Collect all text into one string used for BiDi analysis.
        let text = self.collect_text();

        // Find out the BiDi embedding levels.
        let bidi = BidiInfo::new(&text, Level::from_dir(self.dir));

        // Build a representation of the paragraph on which we can do
        // linebreaking without layouting each and every line from scratch.
        let layout = ParLayout::new(ctx, areas, self, bidi);

        // Find suitable linebreaks.
        layout.build(ctx, areas.clone(), self)
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

impl From<ParNode> for AnyNode {
    fn from(par: ParNode) -> Self {
        Self::new(par)
    }
}

impl Debug for ParChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(amount) => write!(f, "Spacing({:?})", amount),
            Self::Text(text, _, align) => write!(f, "Text({:?}, {:?})", text, align),
            Self::Any(any, align) => {
                f.debug_tuple("Any").field(any).field(align).finish()
            }
        }
    }
}

/// A paragraph representation in which children are already layouted and text
/// is separated into shapable runs.
struct ParLayout<'a> {
    /// The top-level direction.
    dir: Dir,
    /// Bidirectional text embedding levels for the paragraph.
    bidi: BidiInfo<'a>,
    /// Layouted children and separated text runs.
    items: Vec<ParItem<'a>>,
    /// The ranges of the items in `bidi.text`.
    ranges: Vec<Range>,
}

impl<'a> ParLayout<'a> {
    /// Build a paragraph layout for the given node.
    fn new(
        ctx: &mut LayoutContext,
        areas: &Areas,
        par: &'a ParNode,
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
                ParChild::Text(_, ref props, align) => {
                    // TODO: Also split by language and script.
                    for (subrange, dir) in split_runs(&bidi, range) {
                        let text = &bidi.text[subrange.clone()];
                        let shaped = shape(ctx, text, dir, props);
                        items.push(ParItem::Text(shaped, align));
                        ranges.push(subrange);
                    }
                }
                ParChild::Any(ref node, align) => {
                    let frames = node.layout(ctx, areas);
                    assert_eq!(frames.len(), 1);

                    let frame = frames.into_iter().next().unwrap();
                    items.push(ParItem::Frame(frame, align));
                    ranges.push(range);
                }
            }
        }

        Self { dir: par.dir, bidi, items, ranges }
    }

    /// Find first-fit line breaks and build the paragraph.
    fn build(self, ctx: &mut LayoutContext, areas: Areas, par: &ParNode) -> Vec<Frame> {
        let mut stack = LineStack::new(par.line_spacing, areas);

        // The current line attempt.
        // Invariant: Always fits into `stack.areas.current`.
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
            if !stack.areas.current.fits(line.size) {
                if let Some((last_line, last_end)) = last.take() {
                    stack.push(last_line);
                    start = last_end;
                    line = LineLayout::new(ctx, &self, start .. end);
                }
            }

            // If the line does not fit vertically, we start a new area.
            if !stack.areas.current.height.fits(line.size.height)
                && !stack.areas.in_full_last()
            {
                stack.finish_area(ctx);
            }

            if mandatory || !stack.areas.current.width.fits(line.size.width) {
                // If the line does not fit horizontally or we have a mandatory
                // line break (i.e. due to "\n"), we push the line into the
                // stack.
                stack.push(line);
                start = end;
                last = None;

                // If there is a trailing line break at the end of the
                // paragraph, we want to force an empty line.
                if mandatory && end == self.bidi.text.len() {
                    stack.push(LineLayout::new(ctx, &self, end .. end));
                }
            } else {
                // Otherwise, the line fits both horizontally and vertically
                // and we remember it.
                last = Some((line, end));
            }
        }

        if let Some((line, _)) = last {
            stack.push(line);
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
    bidi.levels[range.clone()]
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
    Frame(Frame, Align),
}

impl ParItem<'_> {
    /// The size of the item.
    pub fn size(&self) -> Size {
        match self {
            Self::Spacing(amount) => Size::new(*amount, Length::ZERO),
            Self::Text(shaped, _) => shaped.size,
            Self::Frame(frame, _) => frame.size,
        }
    }

    /// The baseline of the item.
    pub fn baseline(&self) -> Length {
        match self {
            Self::Spacing(_) => Length::ZERO,
            Self::Text(shaped, _) => shaped.baseline,
            Self::Frame(frame, _) => frame.baseline,
        }
    }
}

/// A simple layouter that stacks lines into areas.
struct LineStack<'a> {
    line_spacing: Length,
    areas: Areas,
    finished: Vec<Frame>,
    lines: Vec<LineLayout<'a>>,
    size: Size,
}

impl<'a> LineStack<'a> {
    fn new(line_spacing: Length, areas: Areas) -> Self {
        Self {
            line_spacing,
            areas,
            finished: vec![],
            lines: vec![],
            size: Size::ZERO,
        }
    }

    fn push(&mut self, line: LineLayout<'a>) {
        self.areas.current.height -= line.size.height + self.line_spacing;

        self.size.width = self.size.width.max(line.size.width);
        self.size.height += line.size.height;
        if !self.lines.is_empty() {
            self.size.height += self.line_spacing;
        }

        self.lines.push(line);
    }

    fn finish_area(&mut self, ctx: &mut LayoutContext) {
        if self.areas.fixed.horizontal {
            self.size.width = self.areas.current.width;
        }

        let mut output = Frame::new(self.size, self.size.height);
        let mut first = true;
        let mut offset = Length::ZERO;

        for line in mem::take(&mut self.lines) {
            let frame = line.build(ctx, self.size.width);
            let Frame { size, baseline, .. } = frame;

            let pos = Point::new(Length::ZERO, offset);
            output.push_frame(pos, frame);

            if first {
                output.baseline = offset + baseline;
                first = false;
            }

            offset += size.height + self.line_spacing;
        }

        self.finished.push(output);
        self.areas.next();
        self.size = Size::ZERO;
    }

    fn finish(mut self, ctx: &mut LayoutContext) -> Vec<Frame> {
        self.finish_area(ctx);
        self.finished
    }
}

/// A lightweight representation of a line that spans a specific range in a
/// paragraph's text. This type enables you to cheaply measure the size of a
/// line in a range before comitting to building the line's frame.
struct LineLayout<'a> {
    /// The paragraph the line was created in.
    par: &'a ParLayout<'a>,
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
    fn new(ctx: &mut LayoutContext, par: &'a ParLayout<'a>, mut line: Range) -> Self {
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

        let mut width = Length::ZERO;
        let mut top = Length::ZERO;
        let mut bottom = Length::ZERO;

        // Measure the size of the line.
        for item in first.iter().chain(items).chain(&last) {
            let size = item.size();
            let baseline = item.baseline();
            width += size.width;
            top = top.max(baseline);
            bottom = bottom.max(size.height - baseline);
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
        }
    }

    /// Build the line's frame.
    fn build(&self, ctx: &mut LayoutContext, width: Length) -> Frame {
        let full_width = self.size.width.max(width);
        let full_size = Size::new(full_width, self.size.height);
        let free_width = full_width - self.size.width;

        let mut output = Frame::new(full_size, self.baseline);
        let mut ruler = Align::Start;
        let mut offset = Length::ZERO;

        self.reordered(|item| {
            let frame = match *item {
                ParItem::Spacing(amount) => {
                    offset += amount;
                    return;
                }
                ParItem::Text(ref shaped, align) => {
                    ruler = ruler.max(align);
                    shaped.build(ctx)
                }
                ParItem::Frame(ref frame, align) => {
                    ruler = ruler.max(align);
                    frame.clone()
                }
            };

            let Frame { size, baseline, .. } = frame;
            let pos = Point::new(
                ruler.resolve(self.par.dir, offset .. free_width + offset),
                self.baseline - baseline,
            );

            output.push_frame(pos, frame);
            offset += size.width;
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
            .par
            .bidi
            .paragraphs
            .iter()
            .find(|para| para.range.contains(&self.line.start))
            .unwrap();

        // Compute the reordered ranges in visual order (left to right).
        let (levels, runs) = self.par.bidi.visual_runs(para, self.line.clone());

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

/// Helper methods for BiDi levels.
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
