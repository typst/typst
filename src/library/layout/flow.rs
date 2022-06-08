use std::cmp::Ordering;

use super::{AlignNode, PlaceNode, Spacing};
use crate::library::prelude::*;
use crate::library::text::ParNode;

/// Arrange spacing, paragraphs and other block-level nodes into a flow.
///
/// This node is reponsible for layouting both the top-level content flow and
/// the contents of boxes.
#[derive(Hash)]
pub struct FlowNode(pub StyleVec<FlowChild>);

/// A child of a flow node.
#[derive(Hash, PartialEq)]
pub enum FlowChild {
    /// Vertical spacing between other children.
    Spacing(Spacing),
    /// An arbitrary block-level node.
    Node(LayoutNode),
    /// A column / region break.
    Colbreak,
}

impl Layout for FlowNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let mut layouter = FlowLayouter::new(regions);

        for (child, map) in self.0.iter() {
            let styles = map.chain(&styles);
            match child {
                FlowChild::Spacing(kind) => {
                    layouter.layout_spacing(*kind, styles);
                }
                FlowChild::Node(ref node) => {
                    layouter.layout_node(ctx, node, styles)?;
                }
                FlowChild::Colbreak => {
                    layouter.finish_region();
                }
            }
        }

        Ok(layouter.finish())
    }
}

impl Debug for FlowNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Flow ")?;
        self.0.fmt(f)
    }
}

impl Debug for FlowChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(kind) => write!(f, "{:?}", kind),
            Self::Node(node) => node.fmt(f),
            Self::Colbreak => f.pad("Colbreak"),
        }
    }
}

impl PartialOrd for FlowChild {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Spacing(a), Self::Spacing(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

/// Performs flow layout.
pub struct FlowLayouter {
    /// The regions to layout children into.
    regions: Regions,
    /// Whether the flow should expand to fill the region.
    expand: Spec<bool>,
    /// The full size of `regions.size` that was available before we started
    /// subtracting.
    full: Size,
    /// The size used by the frames for the current region.
    used: Size,
    /// The sum of fractions in the current region.
    fr: Fraction,
    /// Spacing and layouted nodes.
    items: Vec<FlowItem>,
    /// Finished frames for previous regions.
    finished: Vec<Arc<Frame>>,
}

/// A prepared item in a flow layout.
enum FlowItem {
    /// Absolute spacing between other items.
    Absolute(Length),
    /// Fractional spacing between other items.
    Fractional(Fraction),
    /// A frame for a layouted child node and how to align it.
    Frame(Arc<Frame>, Spec<Align>),
    /// An absolutely placed frame.
    Placed(Arc<Frame>),
}

impl FlowLayouter {
    /// Create a new flow layouter.
    pub fn new(regions: &Regions) -> Self {
        let expand = regions.expand;
        let full = regions.first;

        // Disable vertical expansion for children.
        let mut regions = regions.clone();
        regions.expand.y = false;

        Self {
            regions,
            expand,
            full,
            used: Size::zero(),
            fr: Fraction::zero(),
            items: vec![],
            finished: vec![],
        }
    }

    /// Layout spacing.
    pub fn layout_spacing(&mut self, spacing: Spacing, styles: StyleChain) {
        match spacing {
            Spacing::Relative(v) => {
                // Resolve the spacing and limit it to the remaining space.
                let resolved = v.resolve(styles).relative_to(self.full.y);
                let limited = resolved.min(self.regions.first.y);
                self.regions.first.y -= limited;
                self.used.y += limited;
                self.items.push(FlowItem::Absolute(resolved));
            }
            Spacing::Fractional(v) => {
                self.items.push(FlowItem::Fractional(v));
                self.fr += v;
            }
        }
    }

    /// Layout a node.
    pub fn layout_node(
        &mut self,
        ctx: &mut Context,
        node: &LayoutNode,
        styles: StyleChain,
    ) -> TypResult<()> {
        // Don't even try layouting into a full region.
        if self.regions.is_full() {
            self.finish_region();
        }

        // Placed nodes that are out of flow produce placed items which aren't
        // aligned later.
        if let Some(placed) = node.downcast::<PlaceNode>() {
            if placed.out_of_flow() {
                let frame = node.layout(ctx, &self.regions, styles)?.remove(0);
                self.items.push(FlowItem::Placed(frame));
                return Ok(());
            }
        }

        // How to align the node.
        let aligns = Spec::new(
            // For non-expanding paragraphs it is crucial that we align the
            // whole paragraph as it is itself aligned.
            styles.get(ParNode::ALIGN),
            // Vertical align node alignment is respected by the flow node.
            node.downcast::<AlignNode>()
                .and_then(|aligned| aligned.aligns.y)
                .map(|align| align.resolve(styles))
                .unwrap_or(Align::Top),
        );

        let frames = node.layout(ctx, &self.regions, styles)?;
        let len = frames.len();
        for (i, mut frame) in frames.into_iter().enumerate() {
            // Set the generic block role.
            if frame.role().map_or(true, Role::is_weak) {
                Arc::make_mut(&mut frame).apply_role(Role::GenericBlock);
            }

            // Grow our size, shrink the region and save the frame for later.
            let size = frame.size;
            self.used.y += size.y;
            self.used.x.set_max(size.x);
            self.regions.first.y -= size.y;
            self.items.push(FlowItem::Frame(frame, aligns));

            if i + 1 < len {
                self.finish_region();
            }
        }

        Ok(())
    }

    /// Finish the frame for one region.
    pub fn finish_region(&mut self) {
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
                    offset += v.share(self.fr, remaining);
                }
                FlowItem::Frame(frame, aligns) => {
                    ruler = ruler.max(aligns.y);
                    let x = aligns.x.position(size.x - frame.size.x);
                    let y = offset + ruler.position(size.y - self.used.y);
                    let pos = Point::new(x, y);
                    offset += frame.size.y;
                    output.push_frame(pos, frame);
                }
                FlowItem::Placed(frame) => {
                    output.push_frame(Point::zero(), frame);
                }
            }
        }

        // Advance to the next region.
        self.regions.next();
        self.full = self.regions.first;
        self.used = Size::zero();
        self.fr = Fraction::zero();
        self.finished.push(Arc::new(output));
    }

    /// Finish layouting and return the resulting frames.
    pub fn finish(mut self) -> Vec<Arc<Frame>> {
        if self.expand.y {
            while self.regions.backlog.len() > 0 {
                self.finish_region();
            }
        }

        self.finish_region();
        self.finished
    }
}
