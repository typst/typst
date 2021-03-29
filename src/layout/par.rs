use std::fmt::{self, Debug, Formatter};
use std::mem;

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

/// A consecutive, styled run of text.
#[derive(Clone, PartialEq)]
pub struct TextNode {
    /// The text.
    pub text: String,
    /// Properties used for font selection and layout.
    pub props: FontProps,
}

impl Debug for TextNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Text({:?})", self.text)
    }
}

impl Layout for ParNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Frame> {
        let mut layouter = ParLayouter::new(self.dir, self.line_spacing, areas.clone());
        for child in &self.children {
            match *child {
                ParChild::Spacing(amount) => layouter.push_spacing(amount),
                ParChild::Text(ref node, align) => layouter.push_text(ctx, node, align),
                ParChild::Any(ref node, align) => {
                    for frame in node.layout(ctx, &layouter.areas) {
                        layouter.push_frame(frame, align);
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

struct ParLayouter {
    dir: Dir,
    line_spacing: Length,
    areas: Areas,
    finished: Vec<Frame>,
    stack: Vec<(Length, Frame, Align)>,
    stack_size: Size,
    line: Vec<(Length, Frame, Align)>,
    line_size: Size,
    line_ruler: Align,
    hard: bool,
}

impl ParLayouter {
    fn new(dir: Dir, line_spacing: Length, areas: Areas) -> Self {
        Self {
            dir,
            line_spacing,
            areas,
            finished: vec![],
            stack: vec![],
            stack_size: Size::ZERO,
            line: vec![],
            line_size: Size::ZERO,
            line_ruler: Align::Start,
            hard: true,
        }
    }

    fn push_spacing(&mut self, amount: Length) {
        let max = self.areas.current.width;
        self.line_size.width = (self.line_size.width + amount).min(max);
    }

    fn push_text(&mut self, ctx: &mut LayoutContext, node: &TextNode, align: Align) {
        // Position in the text at which the current line starts.
        let mut start = 0;

        // The current line attempt: Text shaped up to the previous line break
        // opportunity.
        let mut last = None;

        let mut iter = LineBreakIterator::new(&node.text).peekable();
        while let Some(&(pos, mandatory)) = iter.peek() {
            let line = &node.text[start .. pos];

            // Remove trailing newline and spacing at the end of lines.
            let mut line = line.trim_end_matches(is_newline);
            if pos != node.text.len() {
                line = line.trim_end();
            }

            let frame = shape(line, &mut ctx.env.fonts, &node.props);

            if self.usable().fits(frame.size) {
                // Still fits into the line.
                if mandatory {
                    // We have to break here.
                    self.push_frame(frame, align);
                    self.finish_line(true);
                    start = pos;
                    last = None;
                } else {
                    last = Some((frame, pos));
                }
            } else if let Some((frame, pos)) = last.take() {
                // The line start..pos doesn't fit. So we write the line up to
                // the last position and retry writing just the single piece
                // behind it.
                self.push_frame(frame, align);
                self.finish_line(false);
                start = pos;
                continue;
            } else {
                // Since last is `None`, we are at the first piece behind a line
                // break and it still doesn't fit. Since we can't break it up
                // further, so we just have to push it.
                self.push_frame(frame, align);
                self.finish_line(false);
                start = pos;
            }

            iter.next();
        }

        // Leftovers.
        if let Some((frame, _)) = last {
            self.push_frame(frame, align);
        }
    }

    fn push_frame(&mut self, frame: Frame, align: Align) {
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
        if self.line_ruler > align {
            self.finish_line(false);
        }

        // Find out whether the area still has enough space for this frame.
        if !self.usable().fits(frame.size) && self.line_size.width > Length::ZERO {
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
        let size = frame.size;
        self.line.push((self.line_size.width, frame, align));
        self.line_size.width += size.width;
        self.line_size.height = self.line_size.height.max(size.height);
        self.line_ruler = align;
    }

    fn usable(&self) -> Size {
        // Space occupied by previous lines is already removed from
        // `areas.current`, but the width of the current line needs to be
        // subtracted to make sure the frame fits.
        let mut usable = self.areas.current;
        usable.width -= self.line_size.width;
        usable
    }

    fn finish_line(&mut self, hard: bool) {
        if !mem::replace(&mut self.hard, hard) && self.line.is_empty() {
            return;
        }

        let full_size = {
            let expand = self.areas.expand.horizontal;
            let full = self.areas.full.width;
            Size::new(
                expand.resolve(self.line_size.width, full),
                self.line_size.height,
            )
        };

        let mut output = Frame::new(full_size);
        for (before, frame, align) in mem::take(&mut self.line) {
            // Align along the x axis.
            let x = align.resolve(if self.dir.is_positive() {
                before .. full_size.width - self.line_size.width + before
            } else {
                let before_with_self = before + frame.size.width;
                full_size.width - before_with_self
                    .. self.line_size.width - before_with_self
            });

            let pos = Point::new(x, Length::ZERO);
            output.push_frame(pos, frame);
        }

        // Add line spacing, but only between lines, not after the last line.
        if !self.stack.is_empty() {
            self.stack_size.height += self.line_spacing;
            self.areas.current.height -= self.line_spacing;
        }

        self.stack.push((self.stack_size.height, output, self.line_ruler));
        self.stack_size.height += full_size.height;
        self.stack_size.width = self.stack_size.width.max(full_size.width);
        self.areas.current.height -= full_size.height;
        self.line_size = Size::ZERO;
        self.line_ruler = Align::Start;
    }

    fn finish_area(&mut self) {
        let mut output = Frame::new(self.stack_size);
        for (before, line, align) in mem::take(&mut self.stack) {
            // Align along the x axis.
            let x = align.resolve(if self.dir.is_positive() {
                Length::ZERO .. self.stack_size.width - line.size.width
            } else {
                self.stack_size.width - line.size.width .. Length::ZERO
            });

            let pos = Point::new(x, before);
            output.push_frame(pos, line);
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
