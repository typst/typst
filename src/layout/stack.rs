use std::fmt::{self, Debug, Formatter};

use super::*;

/// A node that stacks its children.
#[derive(Debug)]
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub struct StackNode {
    /// The stacking direction.
    pub dir: Dir,
    /// The nodes to be stacked.
    pub children: Vec<StackChild>,
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

/// A child of a stack node.
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub struct StackChild {
    /// The node itself.
    pub node: LayoutNode,
    /// How to align the node along the block axis.
    pub align: Align,
}

impl StackChild {
    /// Create a new stack child.
    pub fn new(node: impl Into<LayoutNode>, align: Align) -> Self {
        Self { node: node.into(), align }
    }

    /// Create a spacing stack child.
    pub fn spacing(amount: impl Into<Linear>, axis: SpecAxis) -> Self {
        Self::new(SpacingNode { amount: amount.into(), axis }, Align::Start)
    }
}

impl Debug for StackChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}: ", self.align)?;
        self.node.fmt(f)
    }
}

/// Performs stack layout.
struct StackLayouter<'a> {
    /// The stack node to layout.
    stack: &'a StackNode,
    /// The axis of the block direction.
    axis: SpecAxis,
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
    /// Offset, alignment and frame for all children that fit into the current
    /// region. The exact positions are not known yet.
    frames: Vec<(Length, Align, Rc<Frame>)>,
    /// Finished frames for previous regions.
    finished: Vec<Constrained<Rc<Frame>>>,
}

impl<'a> StackLayouter<'a> {
    /// Create a new stack layouter.
    fn new(stack: &'a StackNode, mut regions: Regions) -> Self {
        let axis = stack.dir.axis();
        let full = regions.current;
        let expand = regions.expand;

        // Disable expansion along the block axis for children.
        regions.expand.set(axis, false);

        Self {
            stack,
            axis,
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
            let frames = child.node.layout(ctx, &self.regions);
            let len = frames.len();
            for (i, frame) in frames.into_iter().enumerate() {
                self.push_frame(frame.item, child.align);
                if i + 1 < len {
                    self.finish_region();
                }
            }
        }

        self.finish_region();
        self.finished
    }

    /// Push a frame into the current region.
    fn push_frame(&mut self, frame: Rc<Frame>, align: Align) {
        // Grow our size.
        let offset = self.used.block;
        let size = frame.size.to_gen(self.axis);
        self.used.block += size.block;
        self.used.inline.set_max(size.inline);
        self.ruler = self.ruler.max(align);

        // Remember the frame and shrink available space in the region for the
        // following children.
        self.frames.push((offset, self.ruler, frame));
        *self.regions.current.get_mut(self.axis) -= size.block;
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self) {
        // Determine the stack's size dependening on whether the region expands.
        let used = self.used.to_size(self.axis);
        let size = Size::new(
            if self.expand.x { self.full.w } else { used.w },
            if self.expand.y { self.full.h } else { used.h },
        );

        let mut output = Frame::new(size, size.h);
        let mut first = true;

        // Place all frames.
        for (offset, align, frame) in self.frames.drain(..) {
            let stack_size = size.to_gen(self.axis);
            let child_size = frame.size.to_gen(self.axis);

            // Align along the block axis.
            let block = align.resolve(
                self.stack.dir,
                if self.stack.dir.is_positive() {
                    offset .. stack_size.block - self.used.block + offset
                } else {
                    let offset_with_self = offset + child_size.block;
                    self.used.block - offset_with_self
                        .. stack_size.block - offset_with_self
                },
            );

            let pos = Gen::new(Length::zero(), block).to_point(self.axis);

            // The baseline of the stack is that of the first frame.
            if first {
                output.baseline = pos.y + frame.baseline;
                first = false;
            }

            output.push_frame(pos, frame);
        }

        // Generate tight constraints for now.
        let mut cts = Constraints::new(self.expand);
        cts.exact = self.full.to_spec().map(Some);
        cts.base = self.regions.base.to_spec().map(Some);

        self.regions.next();
        self.full = self.regions.current;
        self.used = Gen::zero();
        self.ruler = Align::Start;
        self.finished.push(output.constrain(cts));
    }
}
