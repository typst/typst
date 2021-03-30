use std::fmt::{self, Debug, Formatter};
use std::mem;
use std::ops::Range;

use unicode_bidi::{BidiInfo, Level};
use xi_unicode::LineBreakIterator;

use super::*;
use crate::exec::FontProps;
use crate::parse::is_newline;

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
    Text(TextNode, Align),
    /// Any child node and how to align it in its line.
    Any(AnyNode, Align),
}

/// A consecutive, styled run of text.
#[derive(Clone, PartialEq)]
pub struct TextNode {
    /// The text.
    pub text: String,
    /// Properties used for font selection and layout.
    pub props: FontProps,
}

impl Layout for ParNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Frame> {
        let mut text = String::new();
        let mut ranges = vec![];

        // Collect all text into one string used for BiDi analysis.
        for child in &self.children {
            let start = text.len();
            match child {
                ParChild::Spacing(_) => text.push(' '),
                ParChild::Text(node, _) => text.push_str(&node.text),
                ParChild::Any(_, _) => text.push('\u{FFFC}'),
            }
            ranges.push(start .. text.len());
        }

        // Find out the BiDi embedding levels.
        let bidi = BidiInfo::new(&text, Level::from_dir(self.dir));

        let mut layouter =
            ParLayouter::new(self.dir, self.line_spacing, &bidi, areas.clone());

        // Layout the children.
        for (range, child) in ranges.into_iter().zip(&self.children) {
            match *child {
                ParChild::Spacing(amount) => {
                    layouter.push_spacing(range, amount);
                }
                ParChild::Text(ref node, align) => {
                    layouter.push_text(ctx, range, &node.props, align);
                }
                ParChild::Any(ref node, align) => {
                    for frame in node.layout(ctx, &layouter.areas) {
                        layouter.push_frame(range.clone(), frame, align);
                    }
                }
            }
        }

        layouter.finish()
    }
}

impl From<ParNode> for AnyNode {
    fn from(par: ParNode) -> Self {
        Self::new(par)
    }
}

struct ParLayouter<'a> {
    dir: Dir,
    line_spacing: Length,
    bidi: &'a BidiInfo<'a>,
    areas: Areas,
    finished: Vec<Frame>,
    stack: Vec<(Length, Frame, Align)>,
    stack_size: Size,
    line: Line,
}

struct Line {
    items: Vec<LineItem>,
    width: Length,
    top: Length,
    bottom: Length,
    ruler: Align,
    hard: bool,
}

struct LineItem {
    range: Range<usize>,
    frame: Frame,
    align: Align,
}

impl<'a> ParLayouter<'a> {
    fn new(dir: Dir, line_spacing: Length, bidi: &'a BidiInfo<'a>, areas: Areas) -> Self {
        Self {
            dir,
            line_spacing,
            bidi,
            areas,
            finished: vec![],
            stack: vec![],
            stack_size: Size::ZERO,
            line: Line::new(true),
        }
    }

    /// Push horizontal spacing.
    fn push_spacing(&mut self, range: Range<usize>, amount: Length) {
        let amount = amount.min(self.areas.current.width - self.line.width);
        self.line.width += amount;
        self.line.items.push(LineItem {
            range,
            frame: Frame::new(Size::new(amount, Length::ZERO), Length::ZERO),
            align: Align::default(),
        })
    }

    /// Push text with equal font properties, but possibly containing runs of
    /// different directions.
    fn push_text(
        &mut self,
        ctx: &mut LayoutContext,
        range: Range<usize>,
        props: &FontProps,
        align: Align,
    ) {
        let levels = &self.bidi.levels[range.clone()];

        let mut start = range.start;
        let mut last = match levels.first() {
            Some(&level) => level,
            None => return,
        };

        // Split into runs with the same embedding level.
        for (idx, &level) in levels.iter().enumerate() {
            let end = range.start + idx;
            if last != level {
                self.push_run(ctx, start .. end, last.dir(), props, align);
                start = end;
            }
            last = level;
        }

        self.push_run(ctx, start .. range.end, last.dir(), props, align);
    }

    /// Push a text run with fixed direction.
    fn push_run(
        &mut self,
        ctx: &mut LayoutContext,
        range: Range<usize>,
        dir: Dir,
        props: &FontProps,
        align: Align,
    ) {
        // Position in the text at which the current line starts.
        let mut start = range.start;

        // The current line attempt: Text shaped up to the previous line break
        // opportunity.
        let mut last = None;

        // Create an iterator over the line break opportunities.
        let text = &self.bidi.text[range.clone()];
        let mut iter = LineBreakIterator::new(text).peekable();

        while let Some(&(end, mandatory)) = iter.peek() {
            // Slice the line of text.
            let end = range.start + end;
            let line = &self.bidi.text[start .. end];

            // Remove trailing newline and spacing at the end of lines.
            let mut line = line.trim_end_matches(is_newline);
            if end != range.end {
                line = line.trim_end();
            }

            // Shape the line.
            let frame = shape(line, dir, &mut ctx.env.fonts, props);

            // Find out whether the runs still fits into the line.
            if self.usable().fits(frame.size) {
                if mandatory {
                    // We have to break here because the text contained a hard
                    // line break like "\n".
                    self.push_frame(start .. end, frame, align);
                    self.finish_line(true);
                    start = end;
                    last = None;
                } else {
                    // Still fits, so we remember it and try making the line
                    // even longer.
                    last = Some((frame, end));
                }
            } else if let Some((frame, pos)) = last.take() {
                // The line we just tried doesn't fit. So we write the line up
                // to the last position.
                self.push_frame(start .. pos, frame, align);
                self.finish_line(false);
                start = pos;

                // Retry writing just the single piece.
                continue;
            } else {
                // Since `last` is `None`, we are at the first piece behind a
                // line break and it still doesn't fit. Since we can't break it
                // up further, we just have to push it.
                self.push_frame(start .. end, frame, align);
                self.finish_line(false);
                start = end;
            }

            iter.next();
        }

        // Leftovers.
        if let Some((frame, pos)) = last {
            self.push_frame(start .. pos, frame, align);
        }
    }

    fn push_frame(&mut self, range: Range<usize>, frame: Frame, align: Align) {
        // When the alignment of the last pushed frame (stored in the "ruler")
        // is further to the end than the new `frame`, we need a line break.
        //
        // For example
        // ```
        // #align(right)[First] #align(center)[Second]
        // ```
        // would be laid out as:
        // +----------------------------+
        // |                      First |
        // |           Second           |
        // +----------------------------+
        if self.line.ruler > align {
            self.finish_line(false);
        }

        // Find out whether the area still has enough space for this frame.
        if !self.usable().fits(frame.size) && self.line.width > Length::ZERO {
            self.finish_line(false);

            // Here, we can directly check whether the frame fits into
            // `areas.current` since we just called `finish_line`.
            while !self.areas.current.fits(frame.size) {
                if self.areas.in_full_last() {
                    // The frame fits nowhere.
                    // TODO: Should this be placed into the first area or the last?
                    // TODO: Produce diagnostic once the necessary spans exist.
                    break;
                } else {
                    self.finish_area();
                }
            }
        }

        // A line can contain frames with different alignments. Their exact
        // positions are calculated later depending on the alignments.
        let Frame { size, baseline, .. } = frame;
        self.line.items.push(LineItem { range, frame, align });
        self.line.width += size.width;
        self.line.top = self.line.top.max(baseline);
        self.line.bottom = self.line.bottom.max(size.height - baseline);
        self.line.ruler = align;
    }

    fn usable(&self) -> Size {
        // Space occupied by previous lines is already removed from
        // `areas.current`, but the width of the current line needs to be
        // subtracted to make sure the frame fits.
        let mut usable = self.areas.current;
        usable.width -= self.line.width;
        usable
    }

    fn finish_line(&mut self, hard: bool) {
        let mut line = mem::replace(&mut self.line, Line::new(hard));
        if !line.hard && line.items.is_empty() {
            return;
        }

        // BiDi reordering.
        line.reorder(&self.bidi);

        let full_size = {
            let expand = self.areas.expand.horizontal;
            let full = self.areas.full.width;
            Size::new(expand.resolve(line.width, full), line.top + line.bottom)
        };

        let mut output = Frame::new(full_size, line.top + line.bottom);
        let mut offset = Length::ZERO;

        for item in line.items {
            // Align along the x axis.
            let x = item.align.resolve(if self.dir.is_positive() {
                offset .. full_size.width - line.width + offset
            } else {
                full_size.width - line.width + offset .. offset
            });

            offset += item.frame.size.width;

            let pos = Point::new(x, line.top - item.frame.baseline);
            output.push_frame(pos, item.frame);
        }

        // Add line spacing, but only between lines, not after the last line.
        if !self.stack.is_empty() {
            self.stack_size.height += self.line_spacing;
            self.areas.current.height -= self.line_spacing;
        }

        self.stack.push((self.stack_size.height, output, line.ruler));
        self.stack_size.height += full_size.height;
        self.stack_size.width = self.stack_size.width.max(full_size.width);
        self.areas.current.height -= full_size.height;
    }

    fn finish_area(&mut self) {
        let mut output = Frame::new(self.stack_size, Length::ZERO);
        let mut baseline = None;

        for (before, line, align) in mem::take(&mut self.stack) {
            // Align along the x axis.
            let x = align.resolve(if self.dir.is_positive() {
                Length::ZERO .. self.stack_size.width - line.size.width
            } else {
                self.stack_size.width - line.size.width .. Length::ZERO
            });

            let pos = Point::new(x, before);
            baseline.get_or_insert(pos.y + line.baseline);
            output.push_frame(pos, line);
        }

        if let Some(baseline) = baseline {
            output.baseline = baseline;
        }

        self.finished.push(output);
        self.areas.next();
        self.stack_size = Size::ZERO;
    }

    fn finish(mut self) -> Vec<Frame> {
        self.finish_line(false);
        self.finish_area();
        self.finished
    }
}

impl Line {
    fn new(hard: bool) -> Self {
        Self {
            items: vec![],
            width: Length::ZERO,
            top: Length::ZERO,
            bottom: Length::ZERO,
            ruler: Align::Start,
            hard,
        }
    }

    fn reorder(&mut self, bidi: &BidiInfo) {
        let items = &mut self.items;
        let line_range = match (items.first(), items.last()) {
            (Some(first), Some(last)) => first.range.start .. last.range.end,
            _ => return,
        };

        // Find the paragraph that contains the frame.
        let para = bidi
            .paragraphs
            .iter()
            .find(|para| para.range.contains(&line_range.start))
            .unwrap();

        // Compute the reordered ranges in visual order (left to right).
        let (levels, ranges) = bidi.visual_runs(para, line_range);

        // Reorder the items.
        items.sort_by_key(|item| {
            let Range { start, end } = item.range;

            // Determine the index in visual order.
            let idx = ranges.iter().position(|r| r.contains(&start)).unwrap();

            // A run might span more than one frame. To sort frames inside a run
            // based on the run's direction, we compute the distance from
            // the "start" of the run.
            let run = &ranges[idx];
            let dist = if levels[start].is_ltr() {
                start - run.start
            } else {
                run.end - end
            };

            (idx, dist)
        });
    }
}

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

impl Debug for ParChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(amount) => write!(f, "Spacing({:?})", amount),
            Self::Text(node, align) => write!(f, "Text({:?}, {:?})", node.text, align),
            Self::Any(any, align) => {
                f.debug_tuple("Any").field(any).field(align).finish()
            }
        }
    }
}

impl Debug for TextNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Text({:?})", self.text)
    }
}
