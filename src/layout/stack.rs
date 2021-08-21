use super::*;

/// A node that stacks its children.
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub struct StackNode {
    /// The inline and block directions of this stack.
    ///
    /// The children are stacked along the block direction. The inline direction
    /// is required for aligning the children.
    pub dirs: Gen<Dir>,
    /// The nodes to be stacked.
    pub children: Vec<StackChild>,
}

/// A child of a stack node.
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub enum StackChild {
    /// Spacing between other nodes.
    Spacing(Linear),
    /// Any child node and how to align it in the stack.
    Any(LayoutNode, Gen<Align>),
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

impl From<StackNode> for LayoutNode {
    fn from(stack: StackNode) -> Self {
        Self::new(stack)
    }
}

/// Performs stack layout.
struct StackLayouter<'a> {
    /// The stack node to layout.
    stack: &'a StackNode,
    /// The axis of the block direction.
    block: SpecAxis,
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
    /// Whether the last region can fit all the remaining content.
    overflowing: bool,
    /// Offset, alignment and frame for all children that fit into the current
    /// region. The exact positions are not known yet.
    frames: Vec<(Length, Gen<Align>, Rc<Frame>)>,
    /// Finished frames for previous regions.
    finished: Vec<Constrained<Rc<Frame>>>,
}

impl<'a> StackLayouter<'a> {
    /// Create a new stack layouter.
    fn new(stack: &'a StackNode, mut regions: Regions) -> Self {
        let block = stack.dirs.block.axis();
        let full = regions.current;
        let expand = regions.expand;

        // Disable expansion along the block axis for children.
        regions.expand.set(block, false);

        Self {
            stack,
            block,
            expand,
            regions,
            full,
            used: Gen::zero(),
            ruler: Align::Start,
            constraints: Constraints::new(expand),
            overflowing: false,
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
                        if i + 1 < len {
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

    /// Add block-axis spacing into the current region.
    fn space(&mut self, amount: Linear) {
        // Resolve the linear.
        let full = self.full.get(self.block);
        let resolved = amount.resolve(full);

        // Cap the spacing to the remaining available space. This action does
        // not directly affect the constraints because of the cap.
        let remaining = self.regions.current.get_mut(self.block);
        let capped = resolved.min(*remaining);

        // Grow our size and shrink the available space in the region.
        self.used.block += capped;
        *remaining -= capped;
    }

    /// Push a frame into the current or next fitting region, finishing regions
    /// if necessary.
    fn push_frame(&mut self, frame: Rc<Frame>, aligns: Gen<Align>) {
        let size = frame.size.to_gen(self.block);

        // Don't allow `Start` after `End` in the same region.
        if aligns.block < self.ruler {
            self.finish_region();
        }

        // Find a fitting region.
        while !self.regions.current.get(self.block).fits(size.block) {
            if self.regions.in_full_last() {
                self.overflowing = true;
                break;
            }

            self.constraints
                .max
                .get_mut(self.block)
                .set_min(self.used.block + size.block);

            self.finish_region();
        }

        // Shrink available space in the region.
        *self.regions.current.get_mut(self.block) -= size.block;

        // Grow our size.
        let offset = self.used.block;
        self.used.block += size.block;
        self.used.inline.set_max(size.inline);
        self.ruler = aligns.block;

        // Remember the frame with offset and alignment.
        self.frames.push((offset, aligns, frame));
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self) {
        let expand = self.expand;
        let used = self.used.to_size(self.block);

        // Determine the stack's size dependening on whether the region is
        // fixed.
        let size = Size::new(
            if expand.x {
                self.constraints.exact.x = Some(self.full.w);
                self.full.w
            } else {
                self.constraints.min.x = Some(used.w);
                used.w
            },
            if expand.y {
                self.constraints.exact.y = Some(self.full.h);
                self.full.h
            } else {
                self.constraints.min.y = Some(used.h);
                used.h
            },
        );

        if self.overflowing {
            self.constraints.min.y = None;
            self.constraints.max.y = None;
            self.constraints.exact = self.full.to_spec().map(Some);
        }

        let mut output = Frame::new(size, size.h);
        let mut first = true;

        // Place all frames.
        for (offset, aligns, frame) in self.frames.drain(..) {
            let stack_size = size.to_gen(self.block);
            let child_size = frame.size.to_gen(self.block);

            // Align along the inline axis.
            let inline = aligns.inline.resolve(
                self.stack.dirs.inline,
                Length::zero() .. stack_size.inline - child_size.inline,
            );

            // Align along the block axis.
            let block = aligns.block.resolve(
                self.stack.dirs.block,
                if self.stack.dirs.block.is_positive() {
                    offset .. stack_size.block - self.used.block + offset
                } else {
                    let offset_with_self = offset + child_size.block;
                    self.used.block - offset_with_self
                        .. stack_size.block - offset_with_self
                },
            );

            let pos = Gen::new(inline, block).to_point(self.block);

            // The baseline of the stack is that of the first frame.
            if first {
                output.baseline = pos.y + frame.baseline;
                first = false;
            }

            output.push_frame(pos, frame);
        }

        self.regions.next();
        self.full = self.regions.current;
        self.used = Gen::zero();
        self.ruler = Align::Start;
        self.finished.push(output.constrain(self.constraints));
        self.constraints = Constraints::new(expand);
    }
}
