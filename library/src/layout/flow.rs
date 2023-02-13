use typst::model::Style;

use super::{AlignNode, BlockNode, ColbreakNode, ParNode, PlaceNode, Spacing, VNode};
use crate::prelude::*;
use crate::visualize::{CircleNode, EllipseNode, ImageNode, RectNode, SquareNode};

/// Arrange spacing, paragraphs and block-level nodes into a flow.
///
/// This node is responsible for layouting both the top-level content flow and
/// the contents of boxes.
#[capable(Layout)]
#[derive(Hash)]
pub struct FlowNode(pub StyleVec<Content>);

#[node]
impl FlowNode {}

impl Layout for FlowNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut layouter = FlowLayouter::new(regions);

        for (child, map) in self.0.iter() {
            let styles = styles.chain(&map);
            if let Some(&node) = child.to::<VNode>() {
                layouter.layout_spacing(node, styles);
            } else if let Some(node) = child.to::<ParNode>() {
                let barrier = Style::Barrier(child.id());
                let styles = styles.chain_one(&barrier);
                layouter.layout_par(vt, node, styles)?;
            } else if child.is::<RectNode>()
                || child.is::<SquareNode>()
                || child.is::<EllipseNode>()
                || child.is::<CircleNode>()
                || child.is::<ImageNode>()
            {
                let barrier = Style::Barrier(child.id());
                let styles = styles.chain_one(&barrier);
                layouter.layout_single(vt, child, styles)?;
            } else if child.has::<dyn Layout>() {
                layouter.layout_multiple(vt, child, styles)?;
            } else if child.is::<ColbreakNode>() {
                if !layouter.regions.backlog.is_empty() || layouter.regions.last.is_some()
                {
                    layouter.finish_region();
                }
            } else {
                panic!("unexpected flow child: {child:?}");
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

/// Performs flow layout.
struct FlowLayouter<'a> {
    /// The regions to layout children into.
    regions: Regions<'a>,
    /// Whether the flow should expand to fill the region.
    expand: Axes<bool>,
    /// The full size of `regions.size` that was available before we started
    /// subtracting.
    full: Size,
    /// Whether the last block was a paragraph.
    last_was_par: bool,
    /// Spacing and layouted blocks.
    items: Vec<FlowItem>,
    /// Finished frames for previous regions.
    finished: Vec<Frame>,
}

/// A prepared item in a flow layout.
#[derive(Debug)]
enum FlowItem {
    /// Spacing between other items and whether it is weak.
    Absolute(Abs, bool),
    /// Fractional spacing between other items.
    Fractional(Fr),
    /// A frame for a layouted block, how to align it, and whether it is sticky.
    Frame(Frame, Axes<Align>, bool),
    /// An absolutely placed frame.
    Placed(Frame),
}

impl<'a> FlowLayouter<'a> {
    /// Create a new flow layouter.
    fn new(mut regions: Regions<'a>) -> Self {
        let expand = regions.expand;
        let full = regions.size;

        // Disable vertical expansion for children.
        regions.expand.y = false;

        Self {
            regions,
            expand,
            full,
            last_was_par: false,
            items: vec![],
            finished: vec![],
        }
    }

    /// Layout vertical spacing.
    fn layout_spacing(&mut self, node: VNode, styles: StyleChain) {
        self.layout_item(match node.amount {
            Spacing::Rel(v) => FlowItem::Absolute(
                v.resolve(styles).relative_to(self.full.y),
                node.weakness > 0,
            ),
            Spacing::Fr(v) => FlowItem::Fractional(v),
        });
    }

    /// Layout a paragraph.
    fn layout_par(
        &mut self,
        vt: &mut Vt,
        par: &ParNode,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let aligns = styles.get(AlignNode::ALIGNS).resolve(styles);
        let leading = styles.get(ParNode::LEADING);
        let consecutive = self.last_was_par;
        let frames = par
            .layout(vt, styles, consecutive, self.regions.base(), self.regions.expand.x)?
            .into_frames();

        let mut sticky = self.items.len();
        for (i, item) in self.items.iter().enumerate().rev() {
            match *item {
                FlowItem::Absolute(_, _) => {}
                FlowItem::Frame(.., true) => sticky = i,
                _ => break,
            }
        }

        if let [first, ..] = frames.as_slice() {
            if !self.regions.size.y.fits(first.height()) && !self.regions.in_last() {
                let carry: Vec<_> = self.items.drain(sticky..).collect();
                self.finish_region();
                for item in carry {
                    self.layout_item(item);
                }
            }
        }

        for (i, frame) in frames.into_iter().enumerate() {
            if i > 0 {
                self.layout_item(FlowItem::Absolute(leading, true));
            }

            self.layout_item(FlowItem::Frame(frame, aligns, false));
        }

        self.last_was_par = true;

        Ok(())
    }

    /// Layout into a single region.
    fn layout_single(
        &mut self,
        vt: &mut Vt,
        content: &Content,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let aligns = styles.get(AlignNode::ALIGNS).resolve(styles);
        let sticky = styles.get(BlockNode::STICKY);
        let pod = Regions::one(self.regions.base(), Axes::splat(false));
        let layoutable = content.with::<dyn Layout>().unwrap();
        let frame = layoutable.layout(vt, styles, pod)?.into_frame();
        self.layout_item(FlowItem::Frame(frame, aligns, sticky));
        self.last_was_par = false;
        Ok(())
    }

    /// Layout into multiple regions.
    fn layout_multiple(
        &mut self,
        vt: &mut Vt,
        block: &Content,
        styles: StyleChain,
    ) -> SourceResult<()> {
        // Placed nodes that are out of flow produce placed items which aren't
        // aligned later.
        if let Some(placed) = block.to::<PlaceNode>() {
            if placed.out_of_flow() {
                let frame = block.layout(vt, styles, self.regions)?.into_frame();
                self.layout_item(FlowItem::Placed(frame));
                return Ok(());
            }
        }

        // How to align the block.
        let aligns = styles.get(AlignNode::ALIGNS).resolve(styles);

        // Layout the block itself.
        let sticky = styles.get(BlockNode::STICKY);
        let fragment = block.layout(vt, styles, self.regions)?;
        for (i, frame) in fragment.into_iter().enumerate() {
            if i > 0 {
                self.finish_region();
            }
            self.layout_item(FlowItem::Frame(frame, aligns, sticky));
        }

        self.last_was_par = false;

        Ok(())
    }

    /// Layout a finished frame.
    fn layout_item(&mut self, item: FlowItem) {
        match item {
            FlowItem::Absolute(v, _) => self.regions.size.y -= v,
            FlowItem::Fractional(_) => {}
            FlowItem::Frame(ref frame, ..) => {
                let size = frame.size();
                if !self.regions.size.y.fits(size.y) && !self.regions.in_last() {
                    self.finish_region();
                }

                self.regions.size.y -= size.y;
            }
            FlowItem::Placed(_) => {}
        }

        self.items.push(item);
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self) {
        // Trim weak spacing.
        while self
            .items
            .last()
            .map_or(false, |item| matches!(item, FlowItem::Absolute(_, true)))
        {
            self.items.pop();
        }

        // Determine the used size.
        let mut fr = Fr::zero();
        let mut used = Size::zero();
        for item in &self.items {
            match *item {
                FlowItem::Absolute(v, _) => used.y += v,
                FlowItem::Fractional(v) => fr += v,
                FlowItem::Frame(ref frame, ..) => {
                    let size = frame.size();
                    used.y += size.y;
                    used.x.set_max(size.x);
                }
                FlowItem::Placed(_) => {}
            }
        }

        // Determine the size of the flow in this region depending on whether
        // the region expands.
        let mut size = self.expand.select(self.full, used).min(self.full);

        // Account for fractional spacing in the size calculation.
        let remaining = self.full.y - used.y;
        if fr.get() > 0.0 && self.full.y.is_finite() {
            used.y = self.full.y;
            size.y = self.full.y;
        }

        let mut output = Frame::new(size);
        let mut offset = Abs::zero();
        let mut ruler = Align::Top;

        // Place all frames.
        for item in self.items.drain(..) {
            match item {
                FlowItem::Absolute(v, _) => {
                    offset += v;
                }
                FlowItem::Fractional(v) => {
                    offset += v.share(fr, remaining);
                }
                FlowItem::Frame(frame, aligns, _) => {
                    ruler = ruler.max(aligns.y);
                    let x = aligns.x.position(size.x - frame.width());
                    let y = offset + ruler.position(size.y - used.y);
                    let pos = Point::new(x, y);
                    offset += frame.height();
                    output.push_frame(pos, frame);
                }
                FlowItem::Placed(frame) => {
                    output.push_frame(Point::zero(), frame);
                }
            }
        }

        // Advance to the next region.
        self.finished.push(output);
        self.regions.next();
        self.full = self.regions.size;
    }

    /// Finish layouting and return the resulting fragment.
    fn finish(mut self) -> Fragment {
        if self.expand.y {
            while !self.regions.backlog.is_empty() {
                self.finish_region();
            }
        }

        self.finish_region();
        Fragment::frames(self.finished)
    }
}
