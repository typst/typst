//! A flow of paragraphs and other block-level nodes.

use std::fmt::{self, Debug, Formatter};

use super::prelude::*;
use super::{AlignNode, ParNode, PlaceNode, SpacingKind, TextNode};

/// A vertical flow of content consisting of paragraphs and other layout nodes.
///
/// This node is reponsible for layouting both the top-level content flow and
/// the contents of boxes.
#[derive(Hash)]
pub struct FlowNode(pub Vec<Styled<FlowChild>>);

impl Layout for FlowNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Rc<Frame>>> {
        FlowLayouter::new(self, regions.clone()).layout(ctx, styles)
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
    Break,
    /// Skip the rest of the region and move to the next.
    Skip,
    /// Vertical spacing between other children.
    Spacing(SpacingKind),
    /// An arbitrary node.
    Node(PackedNode),
}

impl Debug for FlowChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Break => f.pad("Break"),
            Self::Skip => f.pad("Skip"),
            Self::Spacing(node) => node.fmt(f),
            Self::Node(node) => node.fmt(f),
        }
    }
}

/// Performs flow layout.
struct FlowLayouter<'a> {
    /// The flow node to layout.
    children: &'a [Styled<FlowChild>],
    /// The regions to layout children into.
    regions: Regions,
    /// Whether the flow should expand to fill the region.
    expand: Spec<bool>,
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
    fn new(flow: &'a FlowNode, mut regions: Regions) -> Self {
        let expand = regions.expand;
        let full = regions.current;

        // Disable vertical expansion for children.
        regions.expand.y = false;

        Self {
            children: &flow.0,
            regions,
            expand,
            full,
            used: Size::zero(),
            fr: Fractional::zero(),
            items: vec![],
            finished: vec![],
        }
    }

    /// Layout all children.
    fn layout(
        mut self,
        ctx: &mut LayoutContext,
        styles: StyleChain,
    ) -> Vec<Constrained<Rc<Frame>>> {
        for styled in self.children {
            let styles = styled.map.chain(&styles);
            match styled.item {
                FlowChild::Break => {
                    let em = styles.get(TextNode::SIZE).abs;
                    let amount = styles.get(ParNode::SPACING).resolve(em);
                    self.layout_absolute(amount.into());
                }
                FlowChild::Skip => {
                    self.finish_region();
                }
                FlowChild::Spacing(kind) => {
                    self.layout_spacing(kind);
                }
                FlowChild::Node(ref node) => {
                    if self.regions.is_full() {
                        self.finish_region();
                    }

                    self.layout_node(ctx, node, styles);
                }
            }
        }

        if self.expand.y {
            while self.regions.backlog.len() > 0 {
                self.finish_region();
            }
        }

        self.finish_region();
        self.finished
    }

    /// Layout spacing.
    fn layout_spacing(&mut self, spacing: SpacingKind) {
        match spacing {
            SpacingKind::Linear(v) => self.layout_absolute(v),
            SpacingKind::Fractional(v) => {
                self.items.push(FlowItem::Fractional(v));
                self.fr += v;
            }
        }
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
    fn layout_node(
        &mut self,
        ctx: &mut LayoutContext,
        node: &PackedNode,
        styles: StyleChain,
    ) {
        // Placed nodes that are out of flow produce placed items which aren't
        // aligned later.
        if let Some(placed) = node.downcast::<PlaceNode>() {
            if placed.out_of_flow() {
                let frame = node.layout(ctx, &self.regions, styles).remove(0);
                self.items.push(FlowItem::Placed(frame.item));
                return;
            }
        }

        // How to align the node.
        let aligns = Spec::new(
            // For non-expanding paragraphs it is crucial that we align the
            // whole paragraph as it is itself aligned.
            if node.is::<ParNode>() {
                styles.get(ParNode::ALIGN)
            } else {
                Align::Left
            },
            // Vertical align node alignment is respected by the flow node.
            node.downcast::<AlignNode>()
                .and_then(|aligned| aligned.aligns.y)
                .unwrap_or(Align::Top),
        );

        let frames = node.layout(ctx, &self.regions, styles);
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
