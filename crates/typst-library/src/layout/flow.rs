use std::mem;

use super::{
    AlignElem, BlockElem, ColbreakElem, ColumnsElem, ParElem, PlaceElem, Spacing, VElem,
};
use crate::meta::{FootnoteElem, FootnoteEntry};
use crate::prelude::*;
use crate::visualize::{
    CircleElem, EllipseElem, ImageElem, LineElem, PathElem, PolygonElem, RectElem,
    SquareElem,
};

/// Arranges spacing, paragraphs and block-level elements into a flow.
///
/// This element is responsible for layouting both the top-level content flow
/// and the contents of boxes.
#[elem(Layout)]
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
        if !regions.size.x.is_finite() && regions.expand.x {
            bail!(error!(self.span(), "cannot expand into infinite width"));
        }
        if !regions.size.y.is_finite() && regions.expand.y {
            bail!(error!(self.span(), "cannot expand into infinite height"));
        }
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
            } else if child.is::<LineElem>()
                || child.is::<RectElem>()
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
                let mut frame = Frame::soft(Size::zero());
                frame.meta(styles, true);
                layouter.items.push(FlowItem::Frame {
                    frame,
                    align: Axes::splat(FixedAlign::Start),
                    sticky: true,
                    movable: false,
                });
            } else if let Some(placed) = child.to::<PlaceElem>() {
                layouter.layout_placed(vt, placed, styles)?;
            } else if child.can::<dyn Layout>() {
                layouter.layout_multiple(vt, child, styles)?;
            } else if child.is::<ColbreakElem>() {
                if !layouter.regions.backlog.is_empty() || layouter.regions.last.is_some()
                {
                    layouter.finish_region(vt)?;
                }
            } else {
                bail!(child.span(), "unexpected flow child");
            }
        }

        layouter.finish(vt)
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
    /// A queue of floats.
    pending_floats: Vec<FlowItem>,
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
    /// A frame for a layouted block, how to align it, whether it sticks to the
    /// item after it (for orphan prevention), and whether it is movable
    /// (to keep it together with its footnotes).
    Frame { frame: Frame, align: Axes<FixedAlign>, sticky: bool, movable: bool },
    /// An absolutely placed frame.
    Placed {
        frame: Frame,
        x_align: FixedAlign,
        y_align: Smart<Option<FixedAlign>>,
        delta: Axes<Rel<Abs>>,
        float: bool,
        clearance: Abs,
    },
    /// A footnote frame (can also be the separator).
    Footnote(Frame),
}

impl FlowItem {
    /// The inherent height of the item.
    fn height(&self) -> Abs {
        match self {
            Self::Absolute(v, _) => *v,
            Self::Fractional(_) | Self::Placed { .. } => Abs::zero(),
            Self::Frame { frame, .. } | Self::Footnote(frame) => frame.height(),
        }
    }
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
            pending_floats: vec![],
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
        let align = AlignElem::alignment_in(styles).resolve(styles);
        let leading = ParElem::leading_in(styles);
        let consecutive = self.last_was_par;
        let lines = par
            .layout(vt, styles, consecutive, self.regions.base(), self.regions.expand.x)?
            .into_frames();

        let mut sticky = self.items.len();
        for (i, item) in self.items.iter().enumerate().rev() {
            match *item {
                FlowItem::Absolute(_, _) => {}
                FlowItem::Frame { sticky: true, .. } => sticky = i,
                _ => break,
            }
        }

        if let Some(first) = lines.first() {
            if !self.regions.size.y.fits(first.height()) && !self.regions.in_last() {
                let carry: Vec<_> = self.items.drain(sticky..).collect();
                self.finish_region(vt)?;
                for item in carry {
                    self.layout_item(vt, item)?;
                }
            }
        }

        for (i, frame) in lines.into_iter().enumerate() {
            if i > 0 {
                self.layout_item(vt, FlowItem::Absolute(leading, true))?;
            }

            self.layout_item(
                vt,
                FlowItem::Frame { frame, align, sticky: false, movable: true },
            )?;
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
        let align = AlignElem::alignment_in(styles).resolve(styles);
        let sticky = BlockElem::sticky_in(styles);
        let pod = Regions::one(self.regions.base(), Axes::splat(false));
        let frame = content.layout(vt, styles, pod)?.into_frame();
        self.layout_item(vt, FlowItem::Frame { frame, align, sticky, movable: true })?;
        self.last_was_par = false;
        Ok(())
    }

    /// Layout a placed element.
    fn layout_placed(
        &mut self,
        vt: &mut Vt,
        placed: &PlaceElem,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let float = placed.float(styles);
        let clearance = placed.clearance(styles);
        let alignment = placed.alignment(styles);
        let delta = Axes::new(placed.dx(styles), placed.dy(styles)).resolve(styles);
        let x_align = alignment.map_or(FixedAlign::Center, |align| {
            align.x().unwrap_or_default().resolve(styles)
        });
        let y_align = alignment.map(|align| align.y().map(VAlign::fix));
        let frame = placed.layout(vt, styles, self.regions)?.into_frame();
        let item = FlowItem::Placed { frame, x_align, y_align, delta, float, clearance };
        self.layout_item(vt, item)
    }

    /// Layout into multiple regions.
    fn layout_multiple(
        &mut self,
        vt: &mut Vt,
        block: &Content,
        styles: StyleChain,
    ) -> SourceResult<()> {
        // Temporarily delegerate rootness to the columns.
        let is_root = self.root;
        if is_root && block.is::<ColumnsElem>() {
            self.root = false;
            self.regions.root = true;
        }

        let mut notes = Vec::new();

        if self.regions.is_full() {
            // Skip directly if region is already full.
            self.finish_region(vt)?;
        }

        // How to align the block.
        let align = if let Some(align) = block.to::<AlignElem>() {
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
            // Find footnotes in the frame.
            if self.root {
                find_footnotes(&mut notes, &frame);
            }

            if i > 0 {
                self.finish_region(vt)?;
            }

            let item = FlowItem::Frame { frame, align, sticky, movable: false };
            self.layout_item(vt, item)?;
        }

        self.try_handle_footnotes(vt, notes)?;

        self.root = is_root;
        self.regions.root = false;
        self.last_was_par = false;

        Ok(())
    }

    /// Layout a finished frame.
    #[tracing::instrument(name = "FlowLayouter::layout_item", skip_all)]
    fn layout_item(&mut self, vt: &mut Vt, mut item: FlowItem) -> SourceResult<()> {
        match item {
            FlowItem::Absolute(v, weak) => {
                if weak
                    && !self
                        .items
                        .iter()
                        .any(|item| matches!(item, FlowItem::Frame { .. }))
                {
                    return Ok(());
                }
                self.regions.size.y -= v
            }
            FlowItem::Fractional(_) => {}
            FlowItem::Frame { ref frame, movable, .. } => {
                let height = frame.height();
                if !self.regions.size.y.fits(height) && !self.regions.in_last() {
                    self.finish_region(vt)?;
                }

                self.regions.size.y -= height;
                if self.root && movable {
                    let mut notes = Vec::new();
                    find_footnotes(&mut notes, frame);
                    self.items.push(item);
                    if !self.handle_footnotes(vt, &mut notes, true, false)? {
                        let item = self.items.pop();
                        self.finish_region(vt)?;
                        self.items.extend(item);
                        self.regions.size.y -= height;
                        self.handle_footnotes(vt, &mut notes, true, true)?;
                    }
                    return Ok(());
                }
            }
            FlowItem::Placed { float: false, .. } => {}
            FlowItem::Placed {
                ref mut frame,
                ref mut y_align,
                float: true,
                clearance,
                ..
            } => {
                // If the float doesn't fit, queue it for the next region.
                if !self.regions.size.y.fits(frame.height() + clearance)
                    && !self.regions.in_last()
                {
                    self.pending_floats.push(item);
                    return Ok(());
                }

                // Select the closer placement, top or bottom.
                if y_align.is_auto() {
                    let ratio = (self.regions.size.y
                        - (frame.height() + clearance) / 2.0)
                        / self.regions.full;
                    let better_align =
                        if ratio <= 0.5 { FixedAlign::End } else { FixedAlign::Start };
                    *y_align = Smart::Custom(Some(better_align));
                }

                // Add some clearance so that the float doesn't touch the main
                // content.
                frame.size_mut().y += clearance;
                if *y_align == Smart::Custom(Some(FixedAlign::End)) {
                    frame.translate(Point::with_y(clearance));
                }

                self.regions.size.y -= frame.height();

                // Find footnotes in the frame.
                if self.root {
                    let mut notes = vec![];
                    find_footnotes(&mut notes, frame);
                    self.try_handle_footnotes(vt, notes)?;
                }
            }
            FlowItem::Footnote(_) => {}
        }

        self.items.push(item);
        Ok(())
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self, vt: &mut Vt) -> SourceResult<()> {
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
        let mut float_top_height = Abs::zero();
        let mut float_bottom_height = Abs::zero();
        let mut first_footnote = true;
        for item in &self.items {
            match item {
                FlowItem::Absolute(v, _) => used.y += *v,
                FlowItem::Fractional(v) => fr += *v,
                FlowItem::Frame { frame, .. } => {
                    used.y += frame.height();
                    used.x.set_max(frame.width());
                }
                FlowItem::Placed { float: false, .. } => {}
                FlowItem::Placed { frame, float: true, y_align, .. } => match y_align {
                    Smart::Custom(Some(FixedAlign::Start)) => {
                        float_top_height += frame.height()
                    }
                    Smart::Custom(Some(FixedAlign::End)) => {
                        float_bottom_height += frame.height()
                    }
                    _ => {}
                },
                FlowItem::Footnote(frame) => {
                    footnote_height += frame.height();
                    if !first_footnote {
                        footnote_height += self.footnote_config.gap;
                    }
                    first_footnote = false;
                    used.x.set_max(frame.width());
                }
            }
        }
        used.y += footnote_height + float_top_height + float_bottom_height;

        // Determine the size of the flow in this region depending on whether
        // the region expands. Also account for fractional spacing and
        // footnotes.
        let mut size = self.expand.select(self.initial, used).min(self.initial);
        if (fr.get() > 0.0 || self.has_footnotes) && self.initial.y.is_finite() {
            size.y = self.initial.y;
        }

        let mut output = Frame::soft(size);
        let mut ruler = FixedAlign::Start;
        let mut float_top_offset = Abs::zero();
        let mut offset = float_top_height;
        let mut float_bottom_offset = Abs::zero();
        let mut footnote_offset = Abs::zero();

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
                FlowItem::Frame { frame, align, .. } => {
                    ruler = ruler.max(align.y);
                    let x = align.x.position(size.x - frame.width());
                    let y = offset + ruler.position(size.y - used.y);
                    let pos = Point::new(x, y);
                    offset += frame.height();
                    output.push_frame(pos, frame);
                }
                FlowItem::Placed { frame, x_align, y_align, delta, float, .. } => {
                    let x = x_align.position(size.x - frame.width());
                    let y = if float {
                        match y_align {
                            Smart::Custom(Some(FixedAlign::Start)) => {
                                let y = float_top_offset;
                                float_top_offset += frame.height();
                                y
                            }
                            Smart::Custom(Some(FixedAlign::End)) => {
                                let y = size.y - footnote_height - float_bottom_height
                                    + float_bottom_offset;
                                float_bottom_offset += frame.height();
                                y
                            }
                            _ => unreachable!("float must be y aligned"),
                        }
                    } else {
                        match y_align {
                            Smart::Custom(Some(align)) => {
                                align.position(size.y - frame.height())
                            }
                            _ => offset + ruler.position(size.y - used.y),
                        }
                    };

                    let pos = Point::new(x, y)
                        + delta.zip_map(size, Rel::relative_to).to_point();

                    output.push_frame(pos, frame);
                }
                FlowItem::Footnote(frame) => {
                    let y = size.y - footnote_height + footnote_offset;
                    footnote_offset += frame.height() + self.footnote_config.gap;
                    output.push_frame(Point::with_y(y), frame);
                }
            }
        }

        // Advance to the next region.
        self.finished.push(output);
        self.regions.next();
        self.initial = self.regions.size;
        self.has_footnotes = false;

        // Try to place floats.
        for item in mem::take(&mut self.pending_floats) {
            self.layout_item(vt, item)?;
        }

        Ok(())
    }

    /// Finish layouting and return the resulting fragment.
    fn finish(mut self, vt: &mut Vt) -> SourceResult<Fragment> {
        if self.expand.y {
            while !self.regions.backlog.is_empty() {
                self.finish_region(vt)?;
            }
        }

        self.finish_region(vt)?;
        while !self.items.is_empty() {
            self.finish_region(vt)?;
        }

        Ok(Fragment::frames(self.finished))
    }
}

impl FlowLayouter<'_> {
    fn try_handle_footnotes(
        &mut self,
        vt: &mut Vt,
        mut notes: Vec<FootnoteElem>,
    ) -> SourceResult<()> {
        if self.root && !self.handle_footnotes(vt, &mut notes, false, false)? {
            self.finish_region(vt)?;
            self.handle_footnotes(vt, &mut notes, false, true)?;
        }
        Ok(())
    }

    /// Processes all footnotes in the frame.
    #[tracing::instrument(skip_all)]
    fn handle_footnotes(
        &mut self,
        vt: &mut Vt,
        notes: &mut Vec<FootnoteElem>,
        movable: bool,
        force: bool,
    ) -> SourceResult<bool> {
        let items_len = self.items.len();
        let notes_len = notes.len();

        // Process footnotes one at a time.
        let mut k = 0;
        while k < notes.len() {
            if notes[k].is_ref() {
                k += 1;
                continue;
            }

            if !self.has_footnotes {
                self.layout_footnote_separator(vt)?;
            }

            self.regions.size.y -= self.footnote_config.gap;
            let checkpoint = vt.locator.clone();
            let frames = FootnoteEntry::new(notes[k].clone())
                .pack()
                .layout(vt, self.styles, self.regions.with_root(false))?
                .into_frames();

            // If the entries didn't fit, abort (to keep footnote and entry
            // together).
            if !force
                && (k == 0 || movable)
                && frames.first().map_or(false, Frame::is_empty)
            {
                // Remove existing footnotes attempts because we need to
                // move the item to the next page.
                notes.truncate(notes_len);

                // Undo region modifications.
                for item in self.items.drain(items_len..) {
                    self.regions.size.y -= item.height();
                }

                // Undo Vt modifications.
                *vt.locator = checkpoint;

                return Ok(false);
            }

            let prev = notes.len();
            for (i, frame) in frames.into_iter().enumerate() {
                find_footnotes(notes, &frame);
                if i > 0 {
                    self.finish_region(vt)?;
                    self.layout_footnote_separator(vt)?;
                    self.regions.size.y -= self.footnote_config.gap;
                }
                self.regions.size.y -= frame.height();
                self.items.push(FlowItem::Footnote(frame));
            }

            k += 1;

            // Process the nested notes before dealing with further top-level
            // notes.
            let nested = notes.len() - prev;
            if nested > 0 {
                notes[k..].rotate_right(nested);
            }
        }

        Ok(true)
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
