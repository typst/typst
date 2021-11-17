use std::fmt::{self, Debug, Formatter};

use super::prelude::*;
use super::{AlignNode, ParNode, PlacedNode, Spacing};

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
                Child::Any(node) => FlowChild::Node(node.pack(style)),
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

impl Layout for FlowNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        FlowLayouter::new(self, regions.clone()).layout(ctx)
    }
}

/// A child of a flow node.
#[derive(Hash)]
pub enum FlowChild {
    /// Vertical spacing between other children.
    Spacing(Spacing),
    /// A node and how to align it in the flow.
    Node(PackedNode),
}

impl Debug for FlowChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(v) => write!(f, "Spacing({:?})", v),
            Self::Node(node) => node.fmt(f),
        }
    }
}

/// Performs flow layout.
struct FlowLayouter<'a> {
    /// The flow node to layout.
    children: &'a [FlowChild],
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
    /// A frame to be placed directly at the origin.
    Placed(Rc<Frame>),
    /// A frame for a layouted child node and how to align it.
    Frame(Rc<Frame>, Spec<Align>),
}

impl<'a> FlowLayouter<'a> {
    /// Create a new flow layouter.
    fn new(flow: &'a FlowNode, mut regions: Regions) -> Self {
        // Disable vertical expansion for children.
        let expand = regions.expand;
        regions.expand.y = false;

        Self {
            children: &flow.children,
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
        for child in self.children {
            match *child {
                FlowChild::Spacing(Spacing::Linear(v)) => {
                    self.layout_absolute(v);
                }
                FlowChild::Spacing(Spacing::Fractional(v)) => {
                    self.items.push(FlowItem::Fractional(v));
                    self.fr += v;
                }
                FlowChild::Node(ref node) => {
                    self.layout_node(ctx, node);
                }
            }
        }

        self.finish_region();
        self.finished
    }

    /// Layout absolute spacing.
    fn layout_absolute(&mut self, amount: Linear) {
        // Resolve the linear, limiting it to the remaining available space.
        let resolved = amount.resolve(self.full.h);
        let limited = resolved.min(self.regions.current.h);
        self.regions.current.h -= limited;
        self.used.h += limited;
        self.items.push(FlowItem::Absolute(resolved));
    }

    /// Layout a node.
    fn layout_node(&mut self, ctx: &mut LayoutContext, node: &PackedNode) {
        // Placed nodes with vertical alignment are handled separately
        // because their position shouldn't depend on other flow elements.
        if let Some(placed) = node.downcast::<PlacedNode>() {
            if let Some(aligned) = placed.child.downcast::<AlignNode>() {
                if aligned.aligns.y.is_some() {
                    let base = self.regions.base;
                    let pod = Regions::one(base, base, Spec::splat(true));
                    let frame = placed.layout(ctx, &pod).remove(0);
                    self.items.push(FlowItem::Placed(frame.item));
                    return;
                }
            }
        }

        let aligns = Spec::new(
            // For non-expanding paragraphs it is crucial that we align the
            // whole paragraph according to its internal alignment.
            node.downcast::<ParNode>().map_or(Align::Left, |par| par.align),
            // Vertical align node alignment is respected by the flow node.
            node.downcast::<AlignNode>()
                .and_then(|aligned| aligned.aligns.y)
                .unwrap_or(Align::Top),
        );

        let frames = node.layout(ctx, &self.regions);
        let len = frames.len();
        for (i, frame) in frames.into_iter().enumerate() {
            // Grow our size, shrink the region and save the frame for later.
            let size = frame.item.size;
            self.used.h += size.h;
            self.used.w.set_max(size.w);
            self.regions.current.h -= size.h;
            self.items.push(FlowItem::Frame(frame.item, aligns));

            if i + 1 < len {
                self.finish_region();
            }
        }
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self) {
        // Determine the size of the flow in this region dependening on whether
        // the region expands.
        let mut size = Size::new(
            if self.expand.x { self.full.w } else { self.used.w },
            if self.expand.y { self.full.h } else { self.used.h },
        );

        // Account for fractional spacing in the size calculation.
        let remaining = self.full.h - self.used.h;
        if self.fr.get() > 0.0 && self.full.h.is_finite() {
            self.used.h = self.full.h;
            size.h = self.full.h;
        }

        let mut output = Frame::new(size, size.h);
        let mut before = Length::zero();
        let mut ruler = Align::Top;
        let mut first = true;

        // Place all frames.
        for item in self.items.drain(..) {
            match item {
                FlowItem::Absolute(v) => {
                    before += v;
                }
                FlowItem::Fractional(v) => {
                    before += v.resolve(self.fr, remaining);
                }
                FlowItem::Placed(frame) => {
                    output.push_frame(Point::zero(), frame);
                }
                FlowItem::Frame(frame, aligns) => {
                    ruler = ruler.max(aligns.y);

                    // Align horizontally and vertically.
                    let x = aligns.x.resolve(Length::zero() .. size.w - frame.size.w);
                    let y = ruler.resolve(before .. before + size.h - self.used.h);
                    let pos = Point::new(x, y);
                    before += frame.size.h;

                    // The baseline of the flow is that of the first frame.
                    if first {
                        output.baseline = pos.y + frame.baseline;
                        first = false;
                    }

                    output.push_frame(pos, frame);
                }
            }
        }

        // Generate tight constraints for now.
        let mut cts = Constraints::new(self.expand);
        cts.exact = self.full.to_spec().map(Some);
        cts.base = self.regions.base.to_spec().map(Some);

        // Advance to the next region.
        self.regions.next();
        self.full = self.regions.current;
        self.used = Size::zero();
        self.fr = Fractional::zero();
        self.finished.push(output.constrain(cts));
    }
}
