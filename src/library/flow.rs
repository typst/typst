use std::fmt::{self, Debug, Formatter};

use super::prelude::*;
use super::{AlignNode, ParNode, PlacedNode, SpacingKind, SpacingNode, TextNode};

/// A vertical flow of content consisting of paragraphs and other layout nodes.
///
/// This node is reponsible for layouting both the top-level content flow and
/// the contents of boxes.
#[derive(Hash)]
pub struct FlowNode(pub Vec<FlowChild>);

impl Layout for FlowNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        FlowLayouter::new(self, regions).layout(ctx)
    }
}

impl Debug for FlowNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Flow ")?;
        f.debug_list().entries(&self.0).finish()
    }
}

/// A child of a flow node.
#[derive(Hash)]
pub enum FlowChild {
    /// A paragraph/block break.
    Break(Styles),
    /// Vertical spacing between other children.
    Spacing(SpacingNode),
    /// An arbitrary node.
    Node(PackedNode),
}

impl FlowChild {
    /// A reference to the child's styles.
    pub fn styles(&self) -> &Styles {
        match self {
            Self::Break(styles) => styles,
            Self::Spacing(node) => &node.styles,
            Self::Node(node) => &node.styles,
        }
    }

    /// A mutable reference to the child's styles.
    pub fn styles_mut(&mut self) -> &mut Styles {
        match self {
            Self::Break(styles) => styles,
            Self::Spacing(node) => &mut node.styles,
            Self::Node(node) => &mut node.styles,
        }
    }
}

impl Debug for FlowChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Break(styles) => {
                if f.alternate() {
                    styles.fmt(f)?;
                }
                write!(f, "Break")
            }
            Self::Spacing(node) => node.fmt(f),
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
    /// The regions to layout children into.
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
    /// A frame for a layouted child node and how to align it.
    Frame(Rc<Frame>, Spec<Align>),
    /// An absolutely placed frame.
    Placed(Rc<Frame>),
}

impl<'a> FlowLayouter<'a> {
    /// Create a new flow layouter.
    fn new(flow: &'a FlowNode, regions: &Regions) -> Self {
        let expand = regions.expand;
        let full = regions.current;

        // Disable vertical expansion for children.
        let mut regions = regions.clone();
        regions.expand.y = false;

        Self {
            children: &flow.0,
            expand,
            full,
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
            match child {
                FlowChild::Break(styles) => {
                    let chain = styles.chain(&ctx.styles);
                    let em = chain.get(TextNode::SIZE).abs;
                    let amount = chain.get(ParNode::SPACING).resolve(em);
                    self.layout_absolute(amount.into());
                }
                FlowChild::Spacing(node) => match node.kind {
                    SpacingKind::Linear(v) => self.layout_absolute(v),
                    SpacingKind::Fractional(v) => {
                        self.items.push(FlowItem::Fractional(v));
                        self.fr += v;
                    }
                },
                FlowChild::Node(node) => {
                    if self.regions.is_full() {
                        self.finish_region();
                    }

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
        let resolved = amount.resolve(self.full.y);
        let limited = resolved.min(self.regions.current.y);
        self.regions.current.y -= limited;
        self.used.y += limited;
        self.items.push(FlowItem::Absolute(resolved));
    }

    /// Layout a node.
    fn layout_node(&mut self, ctx: &mut LayoutContext, node: &PackedNode) {
        // Placed nodes that are out of flow produce placed items which aren't
        // aligned later.
        if let Some(placed) = node.downcast::<PlacedNode>() {
            if placed.out_of_flow() {
                let frame = node.layout(ctx, &self.regions).remove(0);
                self.items.push(FlowItem::Placed(frame.item));
                return;
            }
        }

        // How to align the node.
        let aligns = Spec::new(
            // For non-expanding paragraphs it is crucial that we align the
            // whole paragraph according to its internal alignment.
            if node.is::<ParNode>() {
                node.styles.chain(&ctx.styles).get(ParNode::ALIGN)
            } else {
                Align::Left
            },
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
            self.used.y += size.y;
            self.used.x.set_max(size.x);
            self.regions.current.y -= size.y;
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
        let mut size = self.expand.select(self.full, self.used);

        // Account for fractional spacing in the size calculation.
        let remaining = self.full.y - self.used.y;
        if self.fr.get() > 0.0 && self.full.y.is_finite() {
            self.used.y = self.full.y;
            size.y = self.full.y;
        }

        let mut output = Frame::new(size);
        let mut offset = Length::zero();
        let mut ruler = Align::Top;

        // Place all frames.
        for item in self.items.drain(..) {
            match item {
                FlowItem::Absolute(v) => {
                    offset += v;
                }
                FlowItem::Fractional(v) => {
                    offset += v.resolve(self.fr, remaining);
                }
                FlowItem::Frame(frame, aligns) => {
                    ruler = ruler.max(aligns.y);
                    let x = aligns.x.resolve(size.x - frame.size.x);
                    let y = offset + ruler.resolve(size.y - self.used.y);
                    let pos = Point::new(x, y);
                    offset += frame.size.y;
                    output.push_frame(pos, frame);
                }
                FlowItem::Placed(frame) => {
                    output.push_frame(Point::zero(), frame);
                }
            }
        }

        // Generate tight constraints for now.
        let mut cts = Constraints::new(self.expand);
        cts.exact = self.full.map(Some);
        cts.base = self.regions.base.map(Some);

        // Advance to the next region.
        self.regions.next();
        self.full = self.regions.current;
        self.used = Size::zero();
        self.fr = Fractional::zero();
        self.finished.push(output.constrain(cts));
    }
}
