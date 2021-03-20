use super::*;

/// A node that stacks its children.
#[derive(Debug, Clone, PartialEq)]
pub struct StackNode {
    /// The `main` and `cross` directions of this stack.
    ///
    /// The children are stacked along the `main` direction. The `cross`
    /// direction is required for aligning the children.
    pub dirs: LayoutDirs,
    /// How to align this stack in its parent.
    pub aligns: LayoutAligns,
    /// The nodes to be stacked.
    pub children: Vec<Node>,
}

impl Layout for StackNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Fragment {
        let mut layouter = StackLayouter::new(self.dirs, areas.clone());
        for child in &self.children {
            match child.layout(ctx, &layouter.areas) {
                Fragment::Spacing(spacing) => layouter.push_spacing(spacing),
                Fragment::Frame(frame, aligns) => layouter.push_frame(frame, aligns),
                Fragment::Frames(frames, aligns) => {
                    for frame in frames {
                        layouter.push_frame(frame, aligns);
                    }
                }
            }
        }
        Fragment::Frames(layouter.finish(), self.aligns)
    }
}

impl From<StackNode> for AnyNode {
    fn from(stack: StackNode) -> Self {
        Self::new(stack)
    }
}

struct StackLayouter {
    main: SpecAxis,
    dirs: LayoutDirs,
    areas: Areas,
    finished: Vec<Frame>,
    frames: Vec<(Length, Frame, LayoutAligns)>,
    used: Gen<Length>,
    ruler: Align,
}

impl StackLayouter {
    fn new(dirs: LayoutDirs, areas: Areas) -> Self {
        Self {
            main: dirs.main.axis(),
            dirs,
            areas,
            finished: vec![],
            frames: vec![],
            used: Gen::ZERO,
            ruler: Align::Start,
        }
    }

    fn push_spacing(&mut self, amount: Length) {
        let main_rest = self.areas.current.get_mut(self.main);
        let capped = amount.min(*main_rest);
        *main_rest -= capped;
        self.used.main += capped;
    }

    fn push_frame(&mut self, frame: Frame, aligns: LayoutAligns) {
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

        let size = frame.size.switch(self.dirs);
        self.frames.push((self.used.main, frame, aligns));

        *self.areas.current.get_mut(self.main) -= size.main;
        self.used.main += size.main;
        self.used.cross = self.used.cross.max(size.cross);
        self.ruler = aligns.main;
    }

    fn finish_area(&mut self) {
        let full_size = {
            let expand = self.areas.expand;
            let full = self.areas.full;
            let current = self.areas.current;
            let used = self.used.switch(self.dirs).to_size();

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

            size.switch(self.dirs)
        };

        let mut output = Frame::new(full_size.switch(self.dirs).to_size());

        for (before, frame, aligns) in std::mem::take(&mut self.frames) {
            let child_size = frame.size.switch(self.dirs);

            // Align along the main axis.
            let main = aligns.main.resolve(if self.dirs.main.is_positive() {
                let after_with_self = self.used.main - before;
                before .. full_size.main - after_with_self
            } else {
                let before_with_self = before + child_size.main;
                let after = self.used.main - (before + child_size.main);
                full_size.main - before_with_self .. after
            });

            // Align along the cross axis.
            let cross = aligns.cross.resolve(if self.dirs.cross.is_positive() {
                Length::ZERO .. full_size.cross - child_size.cross
            } else {
                full_size.cross - child_size.cross .. Length::ZERO
            });

            let pos = Gen::new(main, cross).switch(self.dirs).to_point();
            output.push_frame(pos, frame);
        }

        self.finished.push(output);

        self.areas.next();
        self.used = Gen::ZERO;
        self.ruler = Align::Start;
    }

    fn finish(mut self) -> Vec<Frame> {
        self.finish_area();
        self.finished
    }
}
