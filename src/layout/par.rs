use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::mem;

use unicode_bidi::{BidiInfo, Level};
use xi_unicode::LineBreakIterator;

use super::*;
use crate::exec::FontProps;

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
        let (text, ranges) = self.collect_text();

        // Find out the BiDi embedding levels.
        let bidi = BidiInfo::new(&text, Level::from_dir(self.dir));

        // Build a representation of the paragraph on which we can do
        // linebreaking without layouting each and every line from scratch.
        let layout = ParLayout::new(ctx, areas, self, bidi, ranges);

        // Find suitable linebreaks.
        layout.build(ctx, areas.clone(), self)
    }
}

impl ParNode {
    /// Concatenate all text in the paragraph into one string, replacing spacing
    /// with a space character and other non-text nodes with the object
    /// replacement character. Returns the full text alongside the range each
    /// child spans in the text.
    fn collect_text(&self) -> (String, Vec<Range>) {
        let mut text = String::new();
        let mut ranges = vec![];

        for child in &self.children {
            let start = text.len();
            match *child {
                ParChild::Spacing(_) => text.push(' '),
                ParChild::Text(ref piece, _, _) => text.push_str(piece),
                ParChild::Any(_, _) => text.push('\u{FFFC}'),
            }
            ranges.push(start .. text.len());
        }

        (text, ranges)
    }
}

/// A paragraph representation in which children are already layouted and text
/// is separated into shapable runs.
#[derive(Debug)]
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

/// A prepared item in a paragraph layout.
#[derive(Debug)]
enum ParItem<'a> {
    /// Spacing between other items.
    Spacing(Length),
    /// A shaped text run with consistent direction.
    Text(ShapeResult<'a>, Align),
    /// A layouted child node.
    Frame(Frame, Align),
}

impl<'a> ParLayout<'a> {
    /// Build a paragraph layout for the given node.
    fn new(
        ctx: &mut LayoutContext,
        areas: &Areas,
        par: &'a ParNode,
        bidi: BidiInfo<'a>,
        ranges: Vec<Range>,
    ) -> Self {
        // Prepare an iterator over each child an the range it spans.
        let iter = ranges.into_iter().zip(&par.children);

        let mut items = vec![];
        let mut ranges = vec![];

        // Layout the children and collect them into items.
        for (range, child) in iter {
            match *child {
                ParChild::Spacing(amount) => {
                    items.push(ParItem::Spacing(amount));
                    ranges.push(range);
                }
                ParChild::Text(_, ref props, align) => {
                    split_runs(&bidi, range, |sub, dir| {
                        let text = &bidi.text[sub.clone()];
                        let shaped = shape(text, dir, &mut ctx.env.fonts, props);
                        items.push(ParItem::Text(shaped, align));
                        ranges.push(sub);
                    });
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
        let mut start = 0;
        let mut last = None;
        let mut stack = LineStack::new(par.line_spacing, areas);

        // Find suitable line breaks.
        // TODO: Provide line break opportunities on alignment changes.
        for (end, mandatory) in LineBreakIterator::new(self.bidi.text) {
            let mut line = LineLayout::new(&self, start .. end, ctx);
            let mut size = line.measure().0;

            if !stack.areas.current.fits(size) {
                if let Some((last_line, last_end)) = last.take() {
                    stack.push(last_line);
                    start = last_end;
                    line = LineLayout::new(&self, start .. end, ctx);
                    size = line.measure().0;
                }
            }

            if !stack.areas.current.height.fits(size.height)
                && !stack.areas.in_full_last()
            {
                stack.finish_area();
            }

            if mandatory || !stack.areas.current.width.fits(size.width) {
                stack.push(line);
                start = end;
                last = None;

                if mandatory && end == self.bidi.text.len() {
                    stack.push(LineLayout::new(&self, end .. end, ctx));
                }
            } else {
                last = Some((line, end));
            }
        }

        if let Some((line, _)) = last {
            stack.push(line);
        }

        stack.finish()
    }

    /// Find the index of the item whose range contains the `text_offset`.
    #[track_caller]
    fn find(&self, text_offset: usize) -> usize {
        find_range(&self.ranges, text_offset).unwrap()
    }
}

impl ParItem<'_> {
    /// The size and baseline of the item.
    pub fn measure(&self) -> (Size, Length) {
        match self {
            Self::Spacing(amount) => (Size::new(*amount, Length::ZERO), Length::ZERO),
            Self::Text(shaped, _) => shaped.measure(),
            Self::Frame(frame, _) => (frame.size, frame.baseline),
        }
    }
}

/// Split a range of text into runs of consistent direction.
fn split_runs(bidi: &BidiInfo, range: Range, mut f: impl FnMut(Range, Dir)) {
    let levels = &bidi.levels[range.clone()];

    let mut start = range.start;
    let mut last = match levels.first() {
        Some(&level) => level,
        None => return,
    };

    // Split into runs with the same embedding level.
    for (idx, &level) in levels.iter().enumerate() {
        let end = range.start + idx;
        if last != level {
            f(start .. end, last.dir());
            start = end;
        }
        last = level;
    }

    f(start .. range.end, last.dir());
}

/// A lightweight representation of a line that spans a specific range in a
/// paragraph's text. This type enables you to cheaply measure the size of a
/// line in a range before comitting to building the line's frame.
struct LineLayout<'a> {
    par: &'a ParLayout<'a>,
    line: Range,
    first: Option<ParItem<'a>>,
    items: &'a [ParItem<'a>],
    last: Option<ParItem<'a>>,
    ranges: &'a [Range],
}

impl<'a> LineLayout<'a> {
    /// Create a line which spans the given range.
    fn new(par: &'a ParLayout<'a>, mut line: Range, ctx: &mut LayoutContext) -> Self {
        // Find the items which bound the text range.
        let last_idx = par.find(line.end - 1);
        let first_idx = if line.is_empty() {
            last_idx
        } else {
            par.find(line.start)
        };

        // Slice out the relevant items and ranges.
        let mut items = &par.items[first_idx ..= last_idx];
        let ranges = &par.ranges[first_idx ..= last_idx];

        // Reshape the last item if it's split in half.
        let mut last = None;
        if let Some((ParItem::Text(shaped, align), rest)) = items.split_last() {
            // Compute the string slice indices local to the shaped result.
            let range = &par.ranges[last_idx];
            let start = line.start.max(range.start) - range.start;
            let end = line.end - range.start;

            // Trim whitespace at the end of the line.
            let end = start + shaped.text()[start .. end].trim_end().len();
            line.end = range.start + end;

            if start != end || rest.is_empty() {
                // Reshape that part (if the indices span the full range reshaping
                // is fast and does nothing).
                let reshaped = shaped.reshape(start .. end, &mut ctx.env.fonts);
                last = Some(ParItem::Text(reshaped, *align));
            }

            items = rest;
        }

        // Reshape the start item if it's split in half.
        let mut first = None;
        if let Some((ParItem::Text(shaped, align), rest)) = items.split_first() {
            let range = &par.ranges[first_idx];
            let start = line.start - range.start;
            let end = line.end.min(range.end) - range.start;
            if start != end {
                let reshaped = shaped.reshape(start .. end, &mut ctx.env.fonts);
                first = Some(ParItem::Text(reshaped, *align));
            }
            items = rest;
        }

        Self { par, line, first, items, last, ranges }
    }

    /// Measure the size of the line without actually building its frame.
    fn measure(&self) -> (Size, Length) {
        let mut width = Length::ZERO;
        let mut top = Length::ZERO;
        let mut bottom = Length::ZERO;

        for item in self.iter() {
            let (size, baseline) = item.measure();
            width += size.width;
            top = top.max(baseline);
            bottom = bottom.max(size.height - baseline);
        }

        (Size::new(width, top + bottom), top)
    }

    /// Build the line's frame.
    fn build(&self, width: Length) -> Frame {
        let (size, baseline) = self.measure();
        let full_size = Size::new(size.width.max(width), size.height);

        let mut output = Frame::new(full_size, baseline);
        let mut offset = Length::ZERO;

        let mut ruler = Align::Start;
        self.reordered(|item| {
            let (frame, align) = match *item {
                ParItem::Spacing(amount) => {
                    offset += amount;
                    return;
                }
                ParItem::Text(ref shaped, align) => (shaped.build(), align),
                ParItem::Frame(ref frame, align) => (frame.clone(), align),
            };

            ruler = ruler.max(align);

            let range = offset .. full_size.width - size.width + offset;
            let x = ruler.resolve(self.par.dir, range);
            let y = baseline - frame.baseline;

            offset += frame.size.width;
            output.push_frame(Point::new(x, y), frame);
        });

        output
    }

    /// Iterate through the line's items in visual order.
    fn reordered(&self, mut f: impl FnMut(&ParItem<'a>)) {
        if self.line.is_empty() {
            return;
        }

        // Find the paragraph that contains the frame.
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
            let first_idx = self.find(run.start);
            let last_idx = self.find(run.end - 1);
            let range = first_idx ..= last_idx;

            // Provide the items forwards or backwards depending on the run's
            // direction.
            if levels[run.start].is_ltr() {
                for item in range {
                    f(self.get(item));
                }
            } else {
                for item in range.rev() {
                    f(self.get(item));
                }
            }
        }
    }

    /// Find the index of the item whose range contains the `text_offset`.
    #[track_caller]
    fn find(&self, text_offset: usize) -> usize {
        find_range(self.ranges, text_offset).unwrap()
    }

    /// Get the item at the index.
    #[track_caller]
    fn get(&self, index: usize) -> &ParItem<'a> {
        self.iter().nth(index).unwrap()
    }

    /// Iterate over the items of the line.
    fn iter(&self) -> impl Iterator<Item = &ParItem<'a>> {
        self.first.iter().chain(self.items).chain(&self.last)
    }
}

/// Find the range that contains the position.
fn find_range(ranges: &[Range], pos: usize) -> Option<usize> {
    ranges.binary_search_by(|r| cmp(r, pos)).ok()
}

/// Comparison function for a range and a position used in binary search.
fn cmp(range: &Range, pos: usize) -> Ordering {
    if pos < range.start {
        Ordering::Greater
    } else if pos < range.end {
        Ordering::Equal
    } else {
        Ordering::Less
    }
}

/// Stacks lines into paragraph frames.
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
        let size = line.measure().0;

        self.size.width = self.size.width.max(size.width);
        self.size.height += size.height;
        if !self.lines.is_empty() {
            self.size.height += self.line_spacing;
        }

        self.areas.current.height -= size.height + self.line_spacing;
        self.lines.push(line);
    }

    fn finish_area(&mut self) {
        let expand = self.areas.expand.horizontal;
        let full = self.areas.full.width;
        self.size.width = expand.resolve(self.size.width, full);

        let mut output = Frame::new(self.size, self.size.height);
        let mut y = Length::ZERO;
        let mut first = true;

        for line in mem::take(&mut self.lines) {
            let frame = line.build(self.size.width);
            let height = frame.size.height;

            if first {
                output.baseline = y + frame.baseline;
                first = false;
            }

            output.push_frame(Point::new(Length::ZERO, y), frame);
            y += height + self.line_spacing;
        }

        self.finished.push(output);
        self.areas.next();
        self.size = Size::ZERO;
    }

    fn finish(mut self) -> Vec<Frame> {
        self.finish_area();
        self.finished
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
