use decorum::N64;

use super::*;

/// A node that stacks its children.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct StackNode {
    /// The `main` and `cross` directions of this stack.
    ///
    /// The children are stacked along the `main` direction. The `cross`
    /// direction is required for aligning the children.
    pub dirs: Gen<Dir>,
    /// The fixed aspect ratio between width and height, if any.
    ///
    /// The resulting frames will satisfy `width = aspect * height`.
    pub aspect: Option<N64>,
    /// The nodes to be stacked.
    pub children: Vec<StackChild>,
}

/// A child of a stack node.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum StackChild {
    /// Spacing between other nodes.
    Spacing(Length),
    /// Any child node and how to align it in the stack.
    Any(AnyNode, Gen<Align>),
}

impl Layout for StackNode {
    fn layout(&self, ctx: &mut LayoutContext, regions: &Regions) -> Vec<Frame> {
        StackLayouter::new(self, regions.clone()).layout(ctx)
    }
}

impl From<StackNode> for AnyNode {
    fn from(stack: StackNode) -> Self {
        Self::new(stack)
    }
}

struct StackLayouter<'a> {
    /// The directions of the stack.
    stack: &'a StackNode,
    /// The axis of the main direction.
    main: SpecAxis,
    /// Whether the stack should expand to fill the region.
    expand: Spec<bool>,
    /// The region to layout into.
    regions: Regions,
    /// Offset, alignment and frame for all children that fit into the current
    /// region. The exact positions are not known yet.
    frames: Vec<(Length, Gen<Align>, Frame)>,
    /// The full size of `regions.current` that was available before we started
    /// subtracting.
    full: Size,
    /// The generic size used by the frames for the current region.
    used: Gen<Length>,
    /// The alignment ruler for the current region.
    ruler: Align,
    /// Finished frames for previous regions.
    finished: Vec<Frame>,
}

impl<'a> StackLayouter<'a> {
    fn new(stack: &'a StackNode, mut regions: Regions) -> Self {
        let main = stack.dirs.main.axis();
        let full = regions.current;
        let expand = regions.expand;

        // Disable expansion on the main axis for children.
        *regions.expand.get_mut(main) = false;

        if let Some(aspect) = stack.aspect {
            regions.apply_aspect_ratio(aspect);
        }

        Self {
            stack,
            main,
            expand,
            regions,
            finished: vec![],
            frames: vec![],
            full,
            used: Gen::zero(),
            ruler: Align::Start,
        }
    }

    fn layout(mut self, ctx: &mut LayoutContext) -> Vec<Frame> {
        for child in &self.stack.children {
            match *child {
                StackChild::Spacing(amount) => self.push_spacing(amount),
                StackChild::Any(ref node, aligns) => {
                    let mut frames = node.layout(ctx, &self.regions).into_iter();
                    if let Some(frame) = frames.next() {
                        self.push_frame(frame, aligns);
                    }

                    for frame in frames {
                        self.finish_region();
                        self.push_frame(frame, aligns);
                    }
                }
            }
        }

        self.finish_region();
        self.finished
    }

    fn push_spacing(&mut self, amount: Length) {
        // Cap the spacing to the remaining available space.
        let remaining = self.regions.current.get_mut(self.main);
        let capped = amount.min(*remaining);

        // Grow our size and shrink the available space in the region.
        self.used.main += capped;
        *remaining -= capped;
    }

    fn push_frame(&mut self, frame: Frame, aligns: Gen<Align>) {
        let size = frame.size;

        // Don't allow `Start` after `End` in the same region.
        if self.ruler > aligns.main {
            self.finish_region();
        }

        // Adjust the ruler.
        self.ruler = aligns.main;

        // Find a fitting region.
        while !self.regions.current.fits(size) && !self.regions.in_full_last() {
            self.finish_region();
        }

        // Remember the frame with offset and alignment.
        self.frames.push((self.used.main, aligns, frame));

        // Grow our size and shrink available space in the region.
        let gen = size.to_gen(self.main);
        self.used.main += gen.main;
        self.used.cross.set_max(gen.cross);
        *self.regions.current.get_mut(self.main) -= gen.main;
    }

    fn finish_region(&mut self) {
        let used = self.used.to_size(self.main);
        let expand = self.expand;

        // Determine the stack's size dependening on whether the region is
        // fixed.
        let mut stack_size = Size::new(
            if expand.horizontal { self.full.width } else { used.width },
            if expand.vertical { self.full.height } else { used.height },
        );

        // Make sure the stack's size satisfies the aspect ratio.
        if let Some(aspect) = self.stack.aspect {
            let width = stack_size
                .width
                .max(aspect.into_inner() * stack_size.height)
                .min(self.full.width)
                .min(aspect.into_inner() * self.full.height);

            stack_size = Size::new(width, width / aspect.into_inner());
        }

        let mut output = Frame::new(stack_size, stack_size.height);
        let mut first = true;

        // Place all frames.
        for (offset, aligns, frame) in std::mem::take(&mut self.frames) {
            let stack_size = stack_size.to_gen(self.main);
            let child_size = frame.size.to_gen(self.main);

            // Align along the cross axis.
            let cross = aligns.cross.resolve(
                self.stack.dirs.cross,
                Length::zero() .. stack_size.cross - child_size.cross,
            );

            // Align along the main axis.
            let main = aligns.main.resolve(
                self.stack.dirs.main,
                if self.stack.dirs.main.is_positive() {
                    offset .. stack_size.main - self.used.main + offset
                } else {
                    let offset_with_self = offset + child_size.main;
                    self.used.main - offset_with_self
                        .. stack_size.main - offset_with_self
                },
            );

            let pos = Gen::new(cross, main).to_point(self.main);

            // The baseline of the stack is that of the first frame.
            if first {
                output.baseline = pos.y + frame.baseline;
                first = false;
            }

            output.push_frame(pos, frame);
        }

        // Move on to the next region.
        self.regions.next();
        if let Some(aspect) = self.stack.aspect {
            self.regions.apply_aspect_ratio(aspect);
        }

        self.full = self.regions.current;
        self.used = Gen::zero();
        self.ruler = Align::Start;
        self.finished.push(output);
    }
}
