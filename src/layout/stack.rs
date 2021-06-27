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
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        StackLayouter::new(self, regions.clone()).layout(ctx)
    }
}

impl From<StackNode> for AnyNode {
    fn from(stack: StackNode) -> Self {
        Self::new(stack)
    }
}

/// Performs stack layout.
struct StackLayouter<'a> {
    /// The stack node to layout.
    stack: &'a StackNode,
    /// The axis of the main direction.
    main: SpecAxis,
    /// Whether the stack should expand to fill the region.
    expand: Spec<bool>,
    /// The region to layout into.
    regions: Regions,
    /// The full size of `regions.current` that was available before we started
    /// subtracting.
    full: Size,
    /// The generic size used by the frames for the current region.
    used: Gen<Length>,
    /// The alignment ruler for the current region.
    ruler: Align,
    /// The constraints for the current region.
    constraints: Constraints,
    /// Offset, alignment and frame for all children that fit into the current
    /// region. The exact positions are not known yet.
    frames: Vec<(Length, Gen<Align>, Rc<Frame>)>,
    /// Finished frames for previous regions.
    finished: Vec<Constrained<Rc<Frame>>>,
}

impl<'a> StackLayouter<'a> {
    /// Create a new stack layouter.
    fn new(stack: &'a StackNode, mut regions: Regions) -> Self {
        let main = stack.dirs.main.axis();
        let full = regions.current;
        let expand = regions.expand;

        // Disable expansion on the main axis for children.
        regions.expand.set(main, false);

        if let Some(aspect) = stack.aspect {
            regions.current = regions.current.with_aspect(aspect.into_inner());
        }

        Self {
            stack,
            main,
            constraints: Constraints::new(expand),
            expand,
            regions,
            full,
            used: Gen::zero(),
            ruler: Align::Start,
            frames: vec![],
            finished: vec![],
        }
    }

    /// Layout all children.
    fn layout(mut self, ctx: &mut LayoutContext) -> Vec<Constrained<Rc<Frame>>> {
        for child in &self.stack.children {
            match *child {
                StackChild::Spacing(amount) => self.space(amount),
                StackChild::Any(ref node, aligns) => {
                    let nodes = node.layout(ctx, &self.regions);
                    let len = nodes.len();
                    for (i, frame) in nodes.into_iter().enumerate() {
                        if i + 1 != len {
                            self.constraints.exact = self.full.to_spec().map(Some);
                        }
                        self.push_frame(frame.item, aligns);
                    }
                }
            }
        }

        self.finish_region();
        self.finished
    }

    /// Add main-axis spacing into the current region.
    fn space(&mut self, amount: Length) {
        // Cap the spacing to the remaining available space. This action does
        // not directly affect the constraints because of the cap.
        let remaining = self.regions.current.get_mut(self.main);
        let capped = amount.min(*remaining);

        // Grow our size and shrink the available space in the region.
        self.used.main += capped;
        *remaining -= capped;
    }

    /// Push a frame into the current or next fitting region, finishing regions
    /// if necessary.
    fn push_frame(&mut self, frame: Rc<Frame>, aligns: Gen<Align>) {
        let size = frame.size.to_gen(self.main);

        // Don't allow `Start` after `End` in the same region.
        if aligns.main < self.ruler {
            self.finish_region();
        }

        // Find a fitting region.
        while !self.regions.current.get(self.main).fits(size.main)
            && !self.regions.in_full_last()
        {
            self.constraints
                .max
                .get_mut(self.main)
                .set_min(size.main + self.used.main);
            self.finish_region();
        }

        // Shrink available space in the region.
        *self.regions.current.get_mut(self.main) -= size.main;

        // Grow our size.
        let offset = self.used.main;
        self.used.main += size.main;
        self.used.cross.set_max(size.cross);
        self.ruler = aligns.main;

        // Remember the frame with offset and alignment.
        self.frames.push((offset, aligns, frame));
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self) {
        let expand = self.expand;
        let used = self.used.to_size(self.main);

        // Determine the stack's size dependening on whether the region is
        // fixed.
        let mut size = Size::new(
            if expand.horizontal {
                self.constraints.exact.horizontal = Some(self.full.width);
                self.full.width
            } else {
                self.constraints.min.horizontal = Some(used.width);
                used.width
            },
            if expand.vertical {
                self.constraints.exact.vertical = Some(self.full.height);
                self.full.height
            } else {
                self.constraints.min.vertical = Some(used.height);
                used.height
            },
        );

        // Make sure the stack's size satisfies the aspect ratio.
        if let Some(aspect) = self.stack.aspect {
            self.constraints.exact = self.full.to_spec().map(Some);
            self.constraints.min = Spec::splat(None);
            self.constraints.max = Spec::splat(None);
            let width = size
                .width
                .max(aspect.into_inner() * size.height)
                .min(self.full.width)
                .min(aspect.into_inner() * self.full.height);

            size = Size::new(width, width / aspect.into_inner());
        }

        let mut output = Frame::new(size, size.height);
        let mut first = true;

        // Place all frames.
        for (offset, aligns, frame) in self.frames.drain(..) {
            let stack_size = size.to_gen(self.main);
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

        self.regions.next();
        if let Some(aspect) = self.stack.aspect {
            self.regions.current = self.regions.current.with_aspect(aspect.into_inner());
        }

        self.full = self.regions.current;
        self.used = Gen::zero();
        self.ruler = Align::Start;
        self.finished.push(output.constrain(self.constraints));
        self.constraints = Constraints::new(expand);
    }
}
