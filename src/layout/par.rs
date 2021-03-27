use std::fmt::{self, Debug, Formatter};

use super::*;
use crate::exec::FontProps;

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
#[derive(Debug, Clone, PartialEq)]
pub enum ParChild {
    /// Spacing between other nodes.
    Spacing(Length),
    /// A run of text and how to align it in its line.
    Text(TextNode, Align),
    /// Any child node and how to align it in its line.
    Any(AnyNode, Align),
    /// A forced linebreak.
    Linebreak,
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
        write!(f, "Text({})", self.text)
    }
}

impl Layout for ParNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Frame> {
        let mut layouter = ParLayouter::new(self.dir, self.line_spacing, areas.clone());
        for child in &self.children {
            match *child {
                ParChild::Spacing(amount) => layouter.push_spacing(amount),
                ParChild::Text(ref node, align) => {
                    let frame = shape(&node.text, &mut ctx.env.fonts, &node.props);
                    layouter.push_frame(frame, align);
                }
                ParChild::Any(ref node, align) => {
                    for frame in node.layout(ctx, &layouter.areas) {
                        layouter.push_frame(frame, align);
                    }
                }
                ParChild::Linebreak => layouter.finish_line(),
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
    dirs: Gen<Dir>,
    main: SpecAxis,
    cross: SpecAxis,
    line_spacing: Length,
    areas: Areas,
    finished: Vec<Frame>,
    stack: Vec<(Length, Frame, Align)>,
    stack_size: Gen<Length>,
    line: Vec<(Length, Frame, Align)>,
    line_size: Gen<Length>,
    line_ruler: Align,
}

impl ParLayouter {
    fn new(dir: Dir, line_spacing: Length, areas: Areas) -> Self {
        Self {
            dirs: Gen::new(Dir::TTB, dir),
            main: SpecAxis::Vertical,
            cross: SpecAxis::Horizontal,
            line_spacing,
            areas,
            finished: vec![],
            stack: vec![],
            stack_size: Gen::ZERO,
            line: vec![],
            line_size: Gen::ZERO,
            line_ruler: Align::Start,
        }
    }

    fn push_spacing(&mut self, amount: Length) {
        let cross_max = self.areas.current.get(self.cross);
        self.line_size.cross = (self.line_size.cross + amount).min(cross_max);
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
            self.finish_line();
        }

        // Find out whether the area still has enough space for this frame.
        // Space occupied by previous lines is already removed from
        // `areas.current`, but the cross-extent of the current line needs to be
        // subtracted to make sure the frame fits.
        let fits = {
            let mut usable = self.areas.current;
            *usable.get_mut(self.cross) -= self.line_size.cross;
            usable.fits(frame.size)
        };

        if !fits {
            self.finish_line();

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

        // A line can contain frames with different alignments. They exact
        // positions are calculated later depending on the alignments.
        let size = frame.size.switch(self.main);
        self.line.push((self.line_size.cross, frame, align));
        self.line_size.cross += size.cross;
        self.line_size.main = self.line_size.main.max(size.main);
        self.line_ruler = align;
    }

    fn finish_line(&mut self) {
        let full_size = {
            let expand = self.areas.expand.get(self.cross);
            let full = self.areas.full.get(self.cross);
            Gen::new(
                self.line_size.main,
                expand.resolve(self.line_size.cross, full),
            )
        };

        let mut output = Frame::new(full_size.switch(self.main).to_size());

        for (before, frame, align) in std::mem::take(&mut self.line) {
            let child_cross_size = frame.size.get(self.cross);

            // Position along the cross axis.
            let cross = align.resolve(if self.dirs.cross.is_positive() {
                let after_with_self = self.line_size.cross - before;
                before .. full_size.cross - after_with_self
            } else {
                let before_with_self = before + child_cross_size;
                let after = self.line_size.cross - (before + child_cross_size);
                full_size.cross - before_with_self .. after
            });

            let pos = Gen::new(Length::ZERO, cross).switch(self.main).to_point();
            output.push_frame(pos, frame);
        }

        // Add line spacing, but only between lines.
        if !self.stack.is_empty() {
            self.stack_size.main += self.line_spacing;
            *self.areas.current.get_mut(self.main) -= self.line_spacing;
        }

        // Update metrics of paragraph and reset for line.
        self.stack.push((self.stack_size.main, output, self.line_ruler));
        self.stack_size.main += full_size.main;
        self.stack_size.cross = self.stack_size.cross.max(full_size.cross);
        *self.areas.current.get_mut(self.main) -= full_size.main;
        self.line_size = Gen::ZERO;
        self.line_ruler = Align::Start;
    }

    fn finish_area(&mut self) {
        let full_size = self.stack_size;
        let mut output = Frame::new(full_size.switch(self.main).to_size());

        for (before, line, cross_align) in std::mem::take(&mut self.stack) {
            let child_size = line.size.switch(self.main);

            // Position along the main axis.
            let main = if self.dirs.main.is_positive() {
                before
            } else {
                full_size.main - (before + child_size.main)
            };

            // Align along the cross axis.
            let cross = cross_align.resolve(if self.dirs.cross.is_positive() {
                Length::ZERO .. full_size.cross - child_size.cross
            } else {
                full_size.cross - child_size.cross .. Length::ZERO
            });

            let pos = Gen::new(main, cross).switch(self.main).to_point();
            output.push_frame(pos, line);
        }

        self.finished.push(output);
        self.areas.next();

        // Reset metrics for the whole paragraph.
        self.stack_size = Gen::ZERO;
    }

    fn finish(mut self) -> Vec<Frame> {
        self.finish_line();
        self.finish_area();
        self.finished
    }
}
