use std::mem;

use super::{
    AlignElem, BlockElem, ColbreakElem, ColumnsElem, ParElem, PlaceElem, Spacing, VElem,
};
use crate::meta::{FootnoteElem, FootnoteEntry};
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
    #[tracing::instrument(name = "FlowElem::layout", skip_all)]
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut layouter = FlowLayouter::new(regions, styles);

        for mut child in &self.children() {
            let outer = styles;
            let mut styles = styles;
            if let Some((elem, map)) = child.to_styled() {
                child = elem;
                styles = outer.chain(map);
            }

            if let Some(elem) = child.to::<VElem>() {
                layouter.layout_spacing(vt, elem, styles)?;
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
                    layouter.finish_region()?;
                }
            } else {
                bail!(child.span(), "unexpected flow child");
            }
        }

        layouter.finish()
    }
}

/// Performs flow layout.
struct FlowLayouter<'a> {
    /// Whether this is the root flow.
    root: bool,
    /// The regions to layout children into.
    regions: Regions<'a>,
    /// The shared styles.
    styles: StyleChain<'a>,
    /// Whether the flow should expand to fill the region.
    expand: Axes<bool>,
    /// The initial size of `regions.size` that was available before we started
    /// subtracting.
    initial: Size,
    /// Whether the last block was a paragraph.
    last_was_par: bool,
    /// Spacing and layouted blocks for the current region.
    items: Vec<FlowItem>,
    /// Whether we have any footnotes in the current region.
    has_footnotes: bool,
    /// Footnote configuration.
    footnote_config: FootnoteConfig,
    /// Finished frames for previous regions.
    finished: Vec<Frame>,
}

/// Cached footnote configuration.
struct FootnoteConfig {
    separator: Content,
    clearance: Abs,
    gap: Abs,
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
    /// A footnote frame (can also be the separator).
    Footnote(Frame),
}

impl<'a> FlowLayouter<'a> {
    /// Create a new flow layouter.
    fn new(mut regions: Regions<'a>, styles: StyleChain<'a>) -> Self {
        let expand = regions.expand;

        // Disable vertical expansion & root for children.
        regions.expand.y = false;
        let root = mem::replace(&mut regions.root, false);

        Self {
            root,
            regions,
            styles,
            expand,
            initial: regions.size,
            last_was_par: false,
            items: vec![],
            has_footnotes: false,
            footnote_config: FootnoteConfig {
                separator: FootnoteEntry::separator_in(styles),
                clearance: FootnoteEntry::clearance_in(styles),
                gap: FootnoteEntry::gap_in(styles),
            },
            finished: vec![],
        }
    }

    /// Layout vertical spacing.
    #[tracing::instrument(name = "FlowLayouter::layout_spacing", skip_all)]
    fn layout_spacing(
        &mut self,
        vt: &mut Vt,
        v: &VElem,
        styles: StyleChain,
    ) -> SourceResult<()> {
        self.layout_item(
            vt,
            match v.amount() {
                Spacing::Rel(rel) => FlowItem::Absolute(
                    rel.resolve(styles).relative_to(self.initial.y),
                    v.weakness(styles) > 0,
                ),
                Spacing::Fr(fr) => FlowItem::Fractional(fr),
            },
        )
    }

    /// Layout a paragraph.
    #[tracing::instrument(name = "FlowLayouter::layout_par", skip_all)]
    fn layout_par(
        &mut self,
        vt: &mut Vt,
        par: &ParElem,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let aligns = AlignElem::alignment_in(styles).resolve(styles);
        let leading = ParElem::leading_in(styles);
        let consecutive = self.last_was_par;
        let lines = par
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

        if let Some(first) = lines.first() {
            if !self.regions.size.y.fits(first.height()) && !self.regions.in_last() {
                let carry: Vec<_> = self.items.drain(sticky..).collect();
                self.finish_region()?;
                for item in carry {
                    self.layout_item(vt, item)?;
                }
            }
        }

        for (i, frame) in lines.into_iter().enumerate() {
            if i > 0 {
                self.layout_item(vt, FlowItem::Absolute(leading, true))?;
            }

            self.layout_item(vt, FlowItem::Frame(frame, aligns, false))?;
        }

        self.last_was_par = true;
        Ok(())
    }

    /// Layout into a single region.
    #[tracing::instrument(name = "FlowLayouter::layout_single", skip_all)]
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
        self.layout_item(vt, FlowItem::Frame(frame, aligns, sticky))?;
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
        // Skip directly if region is already full.
        if self.regions.is_full() {
            self.finish_region()?;
        }

        // Placed elements that are out of flow produce placed items which
        // aren't aligned later.
        if let Some(placed) = block.to::<PlaceElem>() {
            if placed.out_of_flow(styles) {
                let frame = block.layout(vt, styles, self.regions)?.into_frame();
                self.layout_item(vt, FlowItem::Placed(frame))?;
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

        let is_columns = block.is::<ColumnsElem>();

        // Layout the block itself.
        let sticky = BlockElem::sticky_in(styles);
        let fragment = block.layout(vt, styles, self.regions)?;
        self.regions.root = self.root && is_columns;

        for (i, frame) in fragment.into_iter().enumerate() {
            if i > 0 {
                self.finish_region()?;
            }

            self.layout_item(vt, FlowItem::Frame(frame, aligns, sticky))?;
        }

        self.regions.root = false;
        self.last_was_par = false;

        Ok(())
    }

    /// Layout a finished frame.
    #[tracing::instrument(name = "FlowLayouter::layout_item", skip_all)]
    fn layout_item(&mut self, vt: &mut Vt, item: FlowItem) -> SourceResult<()> {
        match item {
            FlowItem::Absolute(v, weak) => {
                if weak
                    && !self.items.iter().any(|item| matches!(item, FlowItem::Frame(..)))
                {
                    return Ok(());
                }
                self.regions.size.y -= v
            }
            FlowItem::Fractional(_) => {}
            FlowItem::Frame(ref frame, ..) => {
                let size = frame.size();
                if !self.regions.size.y.fits(size.y) && !self.regions.in_last() {
                    self.finish_region()?;
                }

                self.regions.size.y -= size.y;
                if self.root {
                    return self.handle_footnotes(vt, item, size.y);
                }
            }
            FlowItem::Placed(_) => {}
            FlowItem::Footnote(_) => {}
        }

        self.items.push(item);
        Ok(())
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self) -> SourceResult<()> {
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
        let mut footnote_height = Abs::zero();
        let mut first_footnote = true;
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
                FlowItem::Footnote(frame) => {
                    let size = frame.size();
                    footnote_height += size.y;
                    if !first_footnote {
                        footnote_height += self.footnote_config.gap;
                    }
                    first_footnote = false;
                    used.x.set_max(size.x);
                }
            }
        }
        used.y += footnote_height;

        // Determine the size of the flow in this region depending on whether
        // the region expands. Also account for fractional spacing and
        // footnotes.
        let mut size = self.expand.select(self.initial, used).min(self.initial);
        if (fr.get() > 0.0 || self.has_footnotes) && self.initial.y.is_finite() {
            size.y = self.initial.y;
        }

        let mut output = Frame::new(size);
        let mut offset = Abs::zero();
        let mut ruler = Align::Top;
        let mut footnote_offset = size.y - footnote_height;

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
                FlowItem::Footnote(frame) => {
                    let pos = Point::with_y(footnote_offset);
                    footnote_offset += frame.height() + self.footnote_config.gap;
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
        self.has_footnotes = false;
        Ok(())
    }

    /// Finish layouting and return the resulting fragment.
    fn finish(mut self) -> SourceResult<Fragment> {
        if self.expand.y {
            while !self.regions.backlog.is_empty() {
                self.finish_region()?;
            }
        }

        self.finish_region()?;
        Ok(Fragment::frames(self.finished))
    }
}

impl FlowLayouter<'_> {
    /// Processes all footnotes in the frame.
    #[tracing::instrument(skip_all)]
    fn handle_footnotes(
        &mut self,
        vt: &mut Vt,
        item: FlowItem,
        height: Abs,
    ) -> SourceResult<()> {
        // Find footnotes in the frame.
        let mut notes = Vec::new();
        if let FlowItem::Frame(frame, ..) = &item {
            find_footnotes(&mut notes, frame);
        }

        self.items.push(item);

        // No new footnotes.
        if notes.is_empty() {
            return Ok(());
        }

        // The currently handled footnote.
        let mut k = 0;

        // Whether we can still skip one region to ensure that the footnote
        // and its entry are on the same page.
        let mut can_skip = true;

        // Process footnotes.
        'outer: while k < notes.len() {
            let had_footnotes = self.has_footnotes;
            if !self.has_footnotes {
                self.layout_footnote_separator(vt)?;
            }

            self.regions.size.y -= self.footnote_config.gap;
            let frames = FootnoteEntry::new(notes[k].clone())
                .pack()
                .layout(vt, self.styles, self.regions.with_root(false))?
                .into_frames();

            // If the entries didn't fit, undo the separator layout, move the
            // item into the next region (to keep footnote and entry together)
            // and try again.
            if can_skip && frames.first().map_or(false, Frame::is_empty) {
                // Remove separator
                if !had_footnotes {
                    self.items.pop();
                }
                let item = self.items.pop();
                self.finish_region()?;
                self.items.extend(item);
                self.regions.size.y -= height;
                can_skip = false;
                continue 'outer;
            }

            let prev = notes.len();
            for (i, frame) in frames.into_iter().enumerate() {
                find_footnotes(&mut notes, &frame);
                if i > 0 {
                    self.finish_region()?;
                    self.layout_footnote_separator(vt)?;
                    self.regions.size.y -= self.footnote_config.gap;
                }
                self.regions.size.y -= frame.height();
                self.items.push(FlowItem::Footnote(frame));
            }

            k += 1;

            // Process the nested notes before dealing with further notes.
            let nested = notes.len() - prev;
            if nested > 0 {
                notes[k..].rotate_right(nested);
            }
        }

        Ok(())
    }

    /// Layout and save the footnote separator, typically a line.
    #[tracing::instrument(skip_all)]
    fn layout_footnote_separator(&mut self, vt: &mut Vt) -> SourceResult<()> {
        let expand = Axes::new(self.regions.expand.x, false);
        let pod = Regions::one(self.regions.base(), expand);
        let separator = &self.footnote_config.separator;

        let mut frame = separator.layout(vt, self.styles, pod)?.into_frame();
        frame.size_mut().y += self.footnote_config.clearance;
        frame.translate(Point::with_y(self.footnote_config.clearance));

        self.has_footnotes = true;
        self.regions.size.y -= frame.height();
        self.items.push(FlowItem::Footnote(frame));

        Ok(())
    }
}

/// Finds all footnotes in the frame.
#[tracing::instrument(skip_all)]
fn find_footnotes(notes: &mut Vec<FootnoteElem>, frame: &Frame) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => find_footnotes(notes, &group.frame),
            FrameItem::Meta(Meta::Elem(content), _)
                if !notes.iter().any(|note| note.0.location() == content.location()) =>
            {
                let Some(footnote) = content.to::<FootnoteElem>() else { continue };
                notes.push(footnote.clone());
            }
            _ => {}
        }
    }
}
