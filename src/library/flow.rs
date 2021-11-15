use std::fmt::{self, Debug, Formatter};

use super::prelude::*;
use super::Spacing;

/// `flow`: A vertical flow of paragraphs and other layout nodes.
pub fn flow(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    enum Child {
        Spacing(Spacing),
        Any(Template),
    }

    castable! {
        Child,
        Expected: "linear, fractional or template",
        Value::Length(v) => Self::Spacing(Spacing::Linear(v.into())),
        Value::Relative(v) => Self::Spacing(Spacing::Linear(v.into())),
        Value::Linear(v) => Self::Spacing(Spacing::Linear(v)),
        Value::Fractional(v) => Self::Spacing(Spacing::Fractional(v)),
        Value::Template(v) => Self::Any(v),
    }

    let children: Vec<Child> = args.all().collect();

    Ok(Value::Template(Template::from_block(move |style| {
        let children = children
            .iter()
            .map(|child| match child {
                Child::Spacing(spacing) => FlowChild::Spacing(*spacing),
                Child::Any(child) => {
                    FlowChild::Node(child.to_flow(style).pack(), style.aligns.block)
                }
            })
            .collect();

        FlowNode { children }
    })))
}

/// A vertical flow of content consisting of paragraphs and other layout nodes.
///
/// This node is reponsible for layouting both the top-level content flow and
/// the contents of boxes.
#[derive(Debug, Hash)]
pub struct FlowNode {
    /// The children that compose the flow. There are different kinds of
    /// children for different purposes.
    pub children: Vec<FlowChild>,
}

/// A child of a flow node.
#[derive(Hash)]
pub enum FlowChild {
    /// Vertical spacing between other children.
    Spacing(Spacing),
    /// Any block node and how to align it in the flow.
    Node(BlockNode, Align),
}

impl BlockLevel for FlowNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        FlowLayouter::new(self, regions.clone()).layout(ctx)
    }
}

impl Debug for FlowChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(v) => write!(f, "Spacing({:?})", v),
            Self::Node(node, _) => node.fmt(f),
        }
    }
}

/// Performs flow layout.
struct FlowLayouter<'a> {
    /// The flow node to layout.
    flow: &'a FlowNode,
    /// Whether the flow should expand to fill the region.
    expand: Spec<bool>,
    /// The region to layout into.
    regions: Regions,
    /// The full size of `regions.current` that was available before we started
    /// subtracting.
    full: Size,
    /// The size used by the frames for the current region.
    used: Size,
    /// The sum of fractional ratios in the current region.
    fr: Fractional,
    /// Spacing and layouted nodes.
    items: Vec<FlowItem>,
    /// Finished frames for previous regions.
    finished: Vec<Constrained<Rc<Frame>>>,
}

/// A prepared item in a flow layout.
enum FlowItem {
    /// Absolute spacing between other items.
    Absolute(Length),
    /// Fractional spacing between other items.
    Fractional(Fractional),
    /// A layouted child node.
    Frame(Rc<Frame>, Align),
}

impl<'a> FlowLayouter<'a> {
    /// Create a new flow layouter.
    fn new(flow: &'a FlowNode, mut regions: Regions) -> Self {
        // Disable vertical expansion for children.
        let expand = regions.expand;
        regions.expand.y = false;

        Self {
            flow,
            expand,
            full: regions.current,
            regions,
            used: Size::zero(),
            fr: Fractional::zero(),
            items: vec![],
            finished: vec![],
        }
    }

    /// Layout all children.
    fn layout(mut self, ctx: &mut LayoutContext) -> Vec<Constrained<Rc<Frame>>> {
        for child in &self.flow.children {
            match *child {
                FlowChild::Spacing(Spacing::Linear(v)) => {
                    self.layout_absolute(v);
                }
                FlowChild::Spacing(Spacing::Fractional(v)) => {
                    self.items.push(FlowItem::Fractional(v));
                    self.fr += v;
                }
                FlowChild::Node(ref node, align) => {
                    self.layout_node(ctx, node, align);
                }
            }
        }

        self.finish_region();
        self.finished
    }

    /// Layout absolute spacing.
    fn layout_absolute(&mut self, amount: Linear) {
        // Resolve the linear, limiting it to the remaining available space.
        let remaining = &mut self.regions.current.h;
        let resolved = amount.resolve(self.full.h);
        let limited = resolved.min(*remaining);
        *remaining -= limited;
        self.used.h += limited;
        self.items.push(FlowItem::Absolute(resolved));
    }

    /// Layout a block node.
    fn layout_node(&mut self, ctx: &mut LayoutContext, node: &BlockNode, align: Align) {
        let frames = node.layout(ctx, &self.regions);
        let len = frames.len();
        for (i, frame) in frames.into_iter().enumerate() {
            // Grow our size.
            let size = frame.item.size;
            self.used.h += size.h;
            self.used.w.set_max(size.w);

            // Remember the frame and shrink available space in the region for the
            // following children.
            self.items.push(FlowItem::Frame(frame.item, align));
            self.regions.current.h -= size.h;

            if i + 1 < len {
                self.finish_region();
            }
        }
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self) {
        // Determine the size that remains for fractional spacing.
        let remaining = self.full.h - self.used.h;

        // Determine the size of the flow in this region dependening on whether
        // the region expands.
        let mut size = Size::new(
            if self.expand.x { self.full.w } else { self.used.w },
            if self.expand.y { self.full.h } else { self.used.h },
        );

        // Expand fully if there are fr spacings.
        if !self.fr.is_zero() && self.full.h.is_finite() {
            size.h = self.full.h;
        }

        let mut output = Frame::new(size, size.h);
        let mut before = Length::zero();
        let mut ruler = Align::Start;
        let mut first = true;

        // Place all frames.
        for item in self.items.drain(..) {
            match item {
                FlowItem::Absolute(v) => before += v,
                FlowItem::Fractional(v) => {
                    let ratio = v / self.fr;
                    if remaining.is_finite() && ratio.is_finite() {
                        before += ratio * remaining;
                    }
                }
                FlowItem::Frame(frame, align) => {
                    ruler = ruler.max(align);

                    // Align vertically.
                    let y =
                        ruler.resolve(Dir::TTB, before .. before + size.h - self.used.h);

                    let pos = Point::new(Length::zero(), y);
                    if first {
                        // The baseline of the flow is that of the first frame.
                        output.baseline = pos.y + frame.baseline;
                        first = false;
                    }

                    before += frame.size.h;
                    output.push_frame(pos, frame);
                }
            }
        }

        // Generate tight constraints for now.
        let mut cts = Constraints::new(self.expand);
        cts.exact = self.full.to_spec().map(Some);
        cts.base = self.regions.base.to_spec().map(Some);

        self.regions.next();
        self.full = self.regions.current;
        self.used = Size::zero();
        self.fr = Fractional::zero();
        self.finished.push(output.constrain(cts));
    }
}
