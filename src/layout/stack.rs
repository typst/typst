use super::*;

/// A node that stacks its children.
#[derive(Debug, Clone, PartialEq)]
pub struct StackNode {
    /// The `main` and `cross` directions of this stack.
    ///
    /// The children are stacked along the `main` direction. The `cross`
    /// direction is required for aligning the children.
    pub dirs: Gen<Dir>,
    /// The nodes to be stacked.
    pub children: Vec<StackChild>,
}

/// A child of a stack node.
#[derive(Debug, Clone, PartialEq)]
pub enum StackChild {
    /// Spacing between other nodes.
    Spacing(Length),
    /// Any child node and how to align it in the stack.
    Any(AnyNode, Gen<Align>),
}

impl Layout for StackNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Frame> {
        let mut layouter = StackLayouter::new(self.dirs, areas.clone());
        for child in &self.children {
            match *child {
                StackChild::Spacing(amount) => layouter.push_spacing(amount),
                StackChild::Any(ref node, aligns) => {
                    for frame in node.layout(ctx, &layouter.areas) {
                        layouter.push_frame(frame, aligns);
                    }
                }
            }
        }
        layouter.finish()
    }
}

impl From<StackNode> for AnyNode {
    fn from(stack: StackNode) -> Self {
        Self::new(stack)
    }
}

struct StackLayouter {
    dirs: Gen<Dir>,
    main: SpecAxis,
    areas: Areas,
    finished: Vec<Frame>,
    frames: Vec<(Length, Frame, Gen<Align>)>,
    size: Gen<Length>,
    ruler: Align,
}

impl StackLayouter {
    fn new(dirs: Gen<Dir>, areas: Areas) -> Self {
        Self {
            dirs,
            main: dirs.main.axis(),
            areas,
            finished: vec![],
            frames: vec![],
            size: Gen::ZERO,
            ruler: Align::Start,
        }
    }

    fn push_spacing(&mut self, amount: Length) {
        let main_rest = self.areas.current.get_mut(self.main);
        let capped = amount.min(*main_rest);
        *main_rest -= capped;
        self.size.main += capped;
    }

    fn push_frame(&mut self, frame: Frame, aligns: Gen<Align>) {
        if self.ruler > aligns.main {
            self.finish_area();
        }

        while !self.areas.current.fits(frame.size) {
            if self.areas.in_full_last() {
                // TODO: Diagnose once the necessary spans exist.
                break;
            } else {
                self.finish_area();
            }
        }

        let size = frame.size.switch(self.main);
        self.frames.push((self.size.main, frame, aligns));
        self.ruler = aligns.main;
        self.size.main += size.main;
        self.size.cross = self.size.cross.max(size.cross);
        *self.areas.current.get_mut(self.main) -= size.main;
    }

    fn finish_area(&mut self) {
        let full_size = {
            let Areas { current, full, expand, .. } = self.areas;
            let used = self.size.switch(self.main).to_size();

            let mut size = Size::new(
                expand.horizontal.resolve(used.width, full.width),
                expand.vertical.resolve(used.height, full.height),
            );

            if let Some(aspect) = self.areas.aspect {
                let width = size
                    .width
                    .max(aspect * size.height)
                    .min(current.width)
                    .min((current.height + used.height) / aspect);

                size = Size::new(width, width / aspect);
            }

            size.switch(self.main)
        };

        let mut output = Frame::new(full_size.switch(self.main).to_size());

        for (before, frame, aligns) in std::mem::take(&mut self.frames) {
            let child_size = frame.size.switch(self.main);

            // Align along the main axis.
            let main = aligns.main.resolve(if self.dirs.main.is_positive() {
                let after_with_self = self.size.main - before;
                before .. full_size.main - after_with_self
            } else {
                let before_with_self = before + child_size.main;
                let after = self.size.main - (before + child_size.main);
                full_size.main - before_with_self .. after
            });

            // Align along the cross axis.
            let cross = aligns.cross.resolve(if self.dirs.cross.is_positive() {
                Length::ZERO .. full_size.cross - child_size.cross
            } else {
                full_size.cross - child_size.cross .. Length::ZERO
            });

            let pos = Gen::new(main, cross).switch(self.main).to_point();
            output.push_frame(pos, frame);
        }

        self.finished.push(output);
        self.areas.next();
        self.ruler = Align::Start;
        self.size = Gen::ZERO;
    }

    fn finish(mut self) -> Vec<Frame> {
        self.finish_area();
        self.finished
    }
}
