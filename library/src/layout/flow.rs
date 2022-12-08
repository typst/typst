use typst::model::Style;

use super::{AlignNode, BlockNode, ColbreakNode, ParNode, PlaceNode, Spacing, VNode};
use crate::prelude::*;

/// Arrange spacing, paragraphs and block-level nodes into a flow.
///
/// This node is reponsible for layouting both the top-level content flow and
/// the contents of boxes.
#[derive(Hash)]
pub struct FlowNode(pub StyleVec<Content>, pub bool);

#[node(Layout)]
impl FlowNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(BlockNode(args.expect("body")?).pack())
    }
}

impl Layout for FlowNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut layouter = FlowLayouter::new(regions, self.1);

        for (child, map) in self.0.iter() {
            let styles = styles.chain(&map);
            if let Some(&node) = child.to::<VNode>() {
                layouter.layout_spacing(node.amount, styles);
            } else if let Some(node) = child.to::<ParNode>() {
                let barrier = Style::Barrier(child.id());
                let styles = styles.chain_one(&barrier);
                layouter.layout_par(vt, node, styles)?;
            } else if child.has::<dyn Layout>() {
                layouter.layout_block(vt, child, styles)?;
            } else if child.is::<ColbreakNode>() {
                layouter.finish_region(false);
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
    /// Whether this is a root page-level flow.
    root: bool,
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
    /// Absolute spacing between other items.
    Absolute(Abs),
    /// Leading between paragraph lines.
    Leading(Abs),
    /// Fractional spacing between other items.
    Fractional(Fr),
    /// A frame for a layouted block and how to align it.
    Frame(Frame, Axes<Align>, bool),
    /// An absolutely placed frame.
    Placed(Frame),
}

impl<'a> FlowLayouter<'a> {
    /// Create a new flow layouter.
    fn new(mut regions: Regions<'a>, root: bool) -> Self {
        let expand = regions.expand;
        let full = regions.first;

        // Disable vertical expansion for children.
        regions.expand.y = false;

        Self {
            root,
            regions,
            expand,
            full,
            last_was_par: false,
            items: vec![],
            finished: vec![],
        }
    }

    /// Layout vertical spacing.
    fn layout_spacing(&mut self, spacing: Spacing, styles: StyleChain) {
        self.layout_item(match spacing {
            Spacing::Relative(v) => {
                FlowItem::Absolute(v.resolve(styles).relative_to(self.full.y))
            }
            Spacing::Fractional(v) => FlowItem::Fractional(v),
        });
    }

    /// Layout a paragraph.
    fn layout_par(
        &mut self,
        vt: &mut Vt,
        par: &ParNode,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let aligns = Axes::new(styles.get(ParNode::ALIGN), Align::Top);
        let leading = styles.get(ParNode::LEADING);
        let consecutive = self.last_was_par;
        let fragment = par.layout(
            vt,
            styles,
            consecutive,
            self.regions.first.x,
            self.regions.base,
            self.regions.expand.x,
        )?;

        let len = fragment.len();
        for (i, frame) in fragment.into_iter().enumerate() {
            if i > 0 {
                self.layout_item(FlowItem::Leading(leading));
            }

            // Prevent widows and orphans.
            let border = (i == 0 && len >= 2) || i + 2 == len;
            let sticky = self.root && !frame.is_empty() && border;
            self.layout_item(FlowItem::Frame(frame, aligns, sticky));
        }

        self.last_was_par = true;

        Ok(())
    }

    /// Layout a block.
    fn layout_block(
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
        let aligns = Axes::new(
            // For non-expanding paragraphs it is crucial that we align the
            // whole paragraph as it is itself aligned.
            styles.get(ParNode::ALIGN),
            // Vertical align node alignment is respected by the flow.
            block
                .to::<AlignNode>()
                .and_then(|aligned| aligned.aligns.y)
                .map(|align| align.resolve(styles))
                .unwrap_or(Align::Top),
        );

        // Layout the block itself.
        let sticky = styles.get(BlockNode::STICKY);
        let fragment = block.layout(vt, styles, self.regions)?;
        for frame in fragment {
            self.layout_item(FlowItem::Frame(frame, aligns, sticky));
        }

        self.last_was_par = false;

        Ok(())
    }

    /// Layout a finished frame.
    fn layout_item(&mut self, item: FlowItem) {
        match item {
            FlowItem::Absolute(v) | FlowItem::Leading(v) => self.regions.first.y -= v,
            FlowItem::Fractional(_) => {}
            FlowItem::Frame(ref frame, ..) => {
                let size = frame.size();
                if !self.regions.first.y.fits(size.y)
                    && !self.regions.in_last()
                    && self.items.iter().any(|item| !matches!(item, FlowItem::Leading(_)))
                {
                    self.finish_region(true);
                }

                self.regions.first.y -= size.y;
            }
            FlowItem::Placed(_) => {}
        }

        self.items.push(item);
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self, something_follows: bool) {
        let mut end = self.items.len();
        if something_follows {
            for (i, item) in self.items.iter().enumerate().rev() {
                match *item {
                    FlowItem::Absolute(_)
                    | FlowItem::Leading(_)
                    | FlowItem::Fractional(_) => {}
                    FlowItem::Frame(.., true) => end = i,
                    _ => break,
                }
            }
            if end == 0 {
                return;
            }
        }

        let carry: Vec<_> = self.items.drain(end..).collect();

        while let Some(FlowItem::Leading(_)) = self.items.last() {
            self.items.pop();
        }

        let mut fr = Fr::zero();
        let mut used = Size::zero();
        for item in &self.items {
            match *item {
                FlowItem::Absolute(v) | FlowItem::Leading(v) => used.y += v,
                FlowItem::Fractional(v) => fr += v,
                FlowItem::Frame(ref frame, ..) => {
                    let size = frame.size();
                    used.y += size.y;
                    used.x.set_max(size.x);
                }
                FlowItem::Placed(_) => {}
            }
        }

        // Determine the size of the flow in this region dependening on whether
        // the region expands.
        let mut size = self.expand.select(self.full, used);

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
                FlowItem::Absolute(v) | FlowItem::Leading(v) => {
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
        self.full = self.regions.first;

        for item in carry {
            self.layout_item(item);
        }
    }

    /// Finish layouting and return the resulting fragment.
    fn finish(mut self) -> Fragment {
        if self.expand.y {
            while !self.regions.backlog.is_empty() {
                self.finish_region(false);
            }
        }

        self.finish_region(false);
        Fragment::frames(self.finished)
    }
}
