use super::{AlignElem, BlockElem, ColbreakElem, ParElem, PlaceElem, Spacing, VElem};
use crate::prelude::*;
use crate::visualize::{
    CircleElem, EllipseElem, ImageElem, PathElem, PolygonElem, RectElem, SquareElem,
};

/// Arrange spacing, paragraphs and block-level elements into a flow.
///
/// This element is responsible for layouting both the top-level content flow and
/// the contents of boxes.
///
/// Display: Flow
/// Category: layout
#[element(Layout)]
pub struct FlowElem {
    /// The children that will be arranges into a flow.
    #[variadic]
    pub children: Vec<Content>,
}

impl Layout for FlowElem {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut layouter = FlowLayouter::new(regions);

        for mut child in &self.children() {
            let outer = styles;
            let mut styles = styles;
            if let Some((elem, map)) = child.to_styled() {
                child = elem;
                styles = outer.chain(map);
            }

            if let Some(elem) = child.to::<VElem>() {
                layouter.layout_spacing(elem, styles);
            } else if let Some(elem) = child.to::<ParElem>() {
                layouter.layout_par(vt, elem, styles)?;
            } else if child.is::<RectElem>()
                || child.is::<SquareElem>()
                || child.is::<EllipseElem>()
                || child.is::<CircleElem>()
                || child.is::<ImageElem>()
                || child.is::<PolygonElem>()
                || child.is::<PathElem>()
            {
                let layoutable = child.with::<dyn Layout>().unwrap();
                layouter.layout_single(vt, layoutable, styles)?;
            } else if child.is::<MetaElem>() {
                let mut frame = Frame::new(Size::zero());
                frame.meta(styles, true);
                layouter.items.push(FlowItem::Frame(
                    frame,
                    Axes::new(Align::Top, Align::Left),
                    true,
                ));
            } else if child.can::<dyn Layout>() {
                layouter.layout_multiple(vt, child, styles)?;
            } else if child.is::<ColbreakElem>() {
                if !layouter.regions.backlog.is_empty() || layouter.regions.last.is_some()
                {
                    layouter.finish_region();
                }
            } else {
                bail!(child.span(), "unexpected flow child");
            }
        }

        Ok(layouter.finish())
    }
}

/// Performs flow layout.
struct FlowLayouter<'a> {
    /// The regions to layout children into.
    regions: Regions<'a>,
    /// Whether the flow should expand to fill the region.
    expand: Axes<bool>,
    /// The initial size of `regions.size` that was available before we started
    /// subtracting.
    initial: Size,
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

        // Disable vertical expansion for children.
        regions.expand.y = false;

        Self {
            regions,
            expand,
            initial: regions.size,
            last_was_par: false,
            items: vec![],
            finished: vec![],
        }
    }

    /// Layout vertical spacing.
    fn layout_spacing(&mut self, v: &VElem, styles: StyleChain) {
        self.layout_item(match v.amount() {
            Spacing::Rel(rel) => FlowItem::Absolute(
                rel.resolve(styles).relative_to(self.initial.y),
                v.weakness(styles) > 0,
            ),
            Spacing::Fr(fr) => FlowItem::Fractional(fr),
        });
    }

    /// Layout a paragraph.
    fn layout_par(
        &mut self,
        vt: &mut Vt,
        par: &ParElem,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let aligns = AlignElem::alignment_in(styles).resolve(styles);
        let leading = ParElem::leading_in(styles);
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
        content: &dyn Layout,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let aligns = AlignElem::alignment_in(styles).resolve(styles);
        let sticky = BlockElem::sticky_in(styles);
        let pod = Regions::one(self.regions.base(), Axes::splat(false));
        let frame = content.layout(vt, styles, pod)?.into_frame();
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
        // Placed elements that are out of flow produce placed items which
        // aren't aligned later.
        if let Some(placed) = block.to::<PlaceElem>() {
            if placed.out_of_flow(styles) {
                let frame = block.layout(vt, styles, self.regions)?.into_frame();
                self.layout_item(FlowItem::Placed(frame));
                return Ok(());
            }
        }

        // How to align the block.
        let aligns = if let Some(align) = block.to::<AlignElem>() {
            align.alignment(styles)
        } else if let Some((_, local)) = block.to_styled() {
            AlignElem::alignment_in(styles.chain(local))
        } else {
            AlignElem::alignment_in(styles)
        }
        .resolve(styles);

        // Layout the block itself.
        let sticky = BlockElem::sticky_in(styles);
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
            match item {
                FlowItem::Absolute(v, _) => used.y += *v,
                FlowItem::Fractional(v) => fr += *v,
                FlowItem::Frame(frame, ..) => {
                    let size = frame.size();
                    used.y += size.y;
                    used.x.set_max(size.x);
                }
                FlowItem::Placed(_) => {}
            }
        }

        // Determine the size of the flow in this region depending on whether
        // the region expands. Also account for fractional spacing.
        let mut size = self.expand.select(self.initial, used).min(self.initial);
        if fr.get() > 0.0 && self.initial.y.is_finite() {
            size.y = self.initial.y;
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
                    let remaining = self.initial.y - used.y;
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
        self.initial = self.regions.size;
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
