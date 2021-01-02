use super::*;

/// A node that stacks its children.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeStack {
    /// The `main` and `cross` directions of this stack.
    ///
    /// The children are stacked along the `main` direction. The `cross`
    /// direction is required for aligning the children.
    pub dirs: LayoutDirs,
    /// How to align this stack in _its_ parent.
    pub align: ChildAlign,
    /// Whether to expand the axes to fill the area or to fit the content.
    pub expansion: Gen<Expansion>,
    /// The nodes to be stacked.
    pub children: Vec<Node>,
}

impl Layout for NodeStack {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
        let mut layouter = StackLayouter::new(self, areas.clone());
        for child in &self.children {
            match child.layout(ctx, &layouter.areas) {
                Layouted::Spacing(spacing) => layouter.push_spacing(spacing),
                Layouted::Frame(frame, align) => layouter.push_frame(frame, align),
                Layouted::Frames(frames, align) => {
                    for frame in frames {
                        layouter.push_frame(frame, align);
                    }
                }
            }
        }
        Layouted::Frames(layouter.finish(), self.align)
    }
}

impl From<NodeStack> for Node {
    fn from(stack: NodeStack) -> Self {
        Self::any(stack)
    }
}

struct StackLayouter<'a> {
    stack: &'a NodeStack,
    main: SpecAxis,
    dirs: LayoutDirs,
    areas: Areas,
    finished: Vec<Frame>,
    frames: Vec<(Length, Frame, ChildAlign)>,
    used: Gen<Length>,
    ruler: Align,
}

impl<'a> StackLayouter<'a> {
    fn new(stack: &'a NodeStack, areas: Areas) -> Self {
        Self {
            stack,
            main: stack.dirs.main.axis(),
            dirs: stack.dirs,
            areas,
            finished: vec![],
            frames: vec![],
            used: Gen::ZERO,
            ruler: Align::Start,
        }
    }

    fn push_spacing(&mut self, amount: Length) {
        let main_rest = self.areas.current.rem.get_mut(self.main);
        let capped = amount.min(*main_rest);
        *main_rest -= capped;
        self.used.main += capped;
    }

    fn push_frame(&mut self, frame: Frame, align: ChildAlign) {
        if self.ruler > align.main {
            self.finish_area();
        }

        while !self.areas.current.rem.fits(frame.size) {
            if self.areas.in_full_last() {
                // TODO: Diagnose once the necessary spans exist.
                let _ = warning!("cannot fit frame into any area");
                break;
            } else {
                self.finish_area();
            }
        }

        let size = frame.size.switch(self.dirs);
        self.frames.push((self.used.main, frame, align));

        *self.areas.current.rem.get_mut(self.main) -= size.main;
        self.used.main += size.main;
        self.used.cross = self.used.cross.max(size.cross);
        self.ruler = align.main;
    }

    fn finish_area(&mut self) {
        let full_size = {
            let full = self.areas.current.full.switch(self.dirs);
            Gen::new(
                match self.stack.expansion.main {
                    Expansion::Fill => full.main,
                    Expansion::Fit => self.used.main.min(full.main),
                },
                match self.stack.expansion.cross {
                    Expansion::Fill => full.cross,
                    Expansion::Fit => self.used.cross.min(full.cross),
                },
            )
        };

        let mut output = Frame::new(full_size.switch(self.dirs).to_size());

        for (before, frame, align) in std::mem::take(&mut self.frames) {
            let child_size = frame.size.switch(self.dirs);

            // Align along the main axis.
            let main = align.main.resolve(if self.dirs.main.is_positive() {
                let after_with_self = self.used.main - before;
                before .. full_size.main - after_with_self
            } else {
                let before_with_self = before + child_size.main;
                let after = self.used.main - (before + child_size.main);
                full_size.main - before_with_self .. after
            });

            // Align along the cross axis.
            let cross = align.cross.resolve(if self.dirs.cross.is_positive() {
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
