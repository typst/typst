use super::*;

/// A node that stacks its children.
#[derive(Debug, Clone, PartialEq)]
pub struct StackNode {
    /// The `main` and `cross` directions of this stack.
    ///
    /// The children are stacked along the `main` direction. The `cross`
    /// direction is required for aligning the children.
    pub dirs: Gen<Dir>,
    /// The fixed aspect ratio between width and height, if any.
    ///
    /// The resulting frames will satisfy `width = aspect * height`.
    pub aspect: Option<f64>,
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
        let mut layouter = StackLayouter::new(self.dirs, self.aspect, areas.clone());
        for child in &self.children {
            match *child {
                StackChild::Spacing(amount) => layouter.push_spacing(amount),
                StackChild::Any(ref node, aligns) => {
                    let mut frames = node.layout(ctx, &layouter.areas).into_iter();
                    if let Some(frame) = frames.next() {
                        layouter.push_frame(frame, aligns);
                    }

                    for frame in frames {
                        layouter.finish_area();
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
    aspect: Option<f64>,
    main: SpecAxis,
    areas: Areas,
    finished: Vec<Frame>,
    frames: Vec<(Length, Frame, Gen<Align>)>,
    full: Size,
    size: Gen<Length>,
    ruler: Align,
}

impl StackLayouter {
    fn new(dirs: Gen<Dir>, aspect: Option<f64>, mut areas: Areas) -> Self {
        if let Some(aspect) = aspect {
            areas.apply_aspect_ratio(aspect);
        }

        Self {
            dirs,
            aspect,
            main: dirs.main.axis(),
            finished: vec![],
            frames: vec![],
            full: areas.current,
            size: Gen::ZERO,
            ruler: Align::Start,
            areas,
        }
    }

    fn push_spacing(&mut self, amount: Length) {
        let remaining = self.areas.current.get_mut(self.main);
        let capped = amount.min(*remaining);
        *remaining -= capped;
        self.size.main += capped;
    }

    fn push_frame(&mut self, frame: Frame, aligns: Gen<Align>) {
        if self.ruler > aligns.main {
            self.finish_area();
        }

        while !self.areas.current.fits(frame.size) && !self.areas.in_full_last() {
            self.finish_area();
        }

        let offset = self.size.main;
        let size = frame.size.switch(self.main);
        self.size.main += size.main;
        self.size.cross.set_max(size.cross);
        self.ruler = aligns.main;
        *self.areas.current.get_mut(self.main) -= size.main;
        self.frames.push((offset, frame, aligns));
    }

    fn finish_area(&mut self) {
        let fixed = self.areas.fixed;

        let used = self.size.switch(self.main).to_size();
        let mut size = Size::new(
            if fixed.horizontal { self.full.width } else { used.width },
            if fixed.vertical { self.full.height } else { used.height },
        );

        if let Some(aspect) = self.aspect {
            let width = size
                .width
                .max(aspect * size.height)
                .min(self.full.width)
                .min(aspect * self.full.height);

            size = Size::new(width, width / aspect);
        }

        let mut output = Frame::new(size, size.height);
        let mut first = true;

        let used = self.size;
        let size = size.switch(self.main);

        for (offset, frame, aligns) in std::mem::take(&mut self.frames) {
            let child = frame.size.switch(self.main);

            // Align along the cross axis.
            let cross = aligns
                .cross
                .resolve(self.dirs.cross, Length::ZERO .. size.cross - child.cross);

            // Align along the main axis.
            let main = aligns.main.resolve(
                self.dirs.main,
                if self.dirs.main.is_positive() {
                    offset .. size.main - used.main + offset
                } else {
                    let offset_with_self = offset + child.main;
                    used.main - offset_with_self .. size.main - offset_with_self
                },
            );

            let pos = Gen::new(cross, main).switch(self.main).to_point();
            if first {
                output.baseline = pos.y + frame.baseline;
                first = false;
            }

            output.push_frame(pos, frame);
        }

        self.size = Gen::ZERO;
        self.ruler = Align::Start;
        self.areas.next();
        if let Some(aspect) = self.aspect {
            self.areas.apply_aspect_ratio(aspect);
        }

        self.finished.push(output);
    }

    fn finish(mut self) -> Vec<Frame> {
        self.finish_area();
        self.finished
    }
}
