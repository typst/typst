use std::fmt::{self, Debug, Formatter};

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, NativeElement, Packed, Resolve, Smart, StyleChain, StyledElem,
};
use crate::introspection::{Meta, MetaElem};
use crate::layout::{
    Abs, AlignElem, Axes, BlockElem, ColbreakElem, ColumnsElem, FixedAlignment, Fr,
    Fragment, Frame, FrameItem, LayoutMultiple, LayoutSingle, PlaceElem, Point, Regions,
    Rel, Size, Spacing, VElem,
};
use crate::model::{FootnoteElem, FootnoteEntry, ParElem};
use crate::util::Numeric;

/// Arranges spacing, paragraphs and block-level elements into a flow.
///
/// This element is responsible for layouting both the top-level content flow
/// and the contents of boxes.
#[elem(Debug, LayoutMultiple)]
pub struct FlowElem {
    /// The children that will be arranges into a flow.
    #[variadic]
    pub children: Vec<Content>,
}

impl LayoutMultiple for Packed<FlowElem> {
    #[typst_macros::time(name = "flow", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        if !regions.size.x.is_finite() && regions.expand.x {
            bail!(self.span(), "cannot expand into infinite width");
        }
        if !regions.size.y.is_finite() && regions.expand.y {
            bail!(self.span(), "cannot expand into infinite height");
        }

        let mut layouter = FlowLayouter::new(regions, styles);
        for mut child in self.children().iter() {
            let outer = styles;
            let mut styles = styles;
            if let Some(styled) = child.to_packed::<StyledElem>() {
                child = &styled.child;
                styles = outer.chain(&styled.styles);
            }

            if child.is::<MetaElem>() {
                layouter.layout_meta(styles);
            } else if let Some(elem) = child.to_packed::<VElem>() {
                layouter.layout_spacing(engine, elem, styles)?;
            } else if let Some(placed) = child.to_packed::<PlaceElem>() {
                layouter.layout_placed(engine, placed, styles)?;
            } else if child.is::<ColbreakElem>() {
                if !layouter.regions.backlog.is_empty() || layouter.regions.last.is_some()
                {
                    layouter.finish_region(engine, true)?;
                }
            } else if let Some(elem) = child.to_packed::<ParElem>() {
                layouter.layout_par(engine, elem, styles)?;
            } else if let Some(layoutable) = child.with::<dyn LayoutSingle>() {
                layouter.layout_single(engine, layoutable, styles)?;
            } else if child.can::<dyn LayoutMultiple>() {
                layouter.layout_multiple(engine, child, styles)?;
            } else {
                bail!(child.span(), "unexpected flow child");
            }
        }

        layouter.finish(engine)
    }
}

impl Debug for FlowElem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Flow ")?;
        f.debug_list().entries(&self.children).finish()
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
    Frame { frame: Frame, align: Axes<FixedAlignment>, sticky: bool, movable: bool },
    /// An absolutely placed frame.
    Placed {
        frame: Frame,
        x_align: FixedAlignment,
        y_align: Smart<Option<FixedAlignment>>,
        delta: Axes<Rel<Abs>>,
        float: bool,
        clearance: Abs,
    },
    /// A footnote frame (can also be the separator).
    Footnote(Frame),
}

impl FlowItem {
    /// Whether this item is out-of-flow.
    ///
    /// Out-of-flow items are guaranteed to have a [`Size::zero()`].
    fn is_out_of_flow(&self) -> bool {
        match self {
            Self::Placed { float: false, .. } => true,
            Self::Frame { frame, .. } => {
                frame.size().is_zero()
                    && frame.items().all(|(_, item)| matches!(item, FrameItem::Meta(..)))
            }
            _ => false,
        }
    }
}

impl<'a> FlowLayouter<'a> {
    /// Create a new flow layouter.
    fn new(mut regions: Regions<'a>, styles: StyleChain<'a>) -> Self {
        let expand = regions.expand;

        // Disable vertical expansion & root for children.
        regions.expand.y = false;
        let root = std::mem::replace(&mut regions.root, false);

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

    /// Place explicit metadata into the flow.
    fn layout_meta(&mut self, styles: StyleChain) {
        let mut frame = Frame::soft(Size::zero());
        frame.meta(styles, true);
        self.items.push(FlowItem::Frame {
            frame,
            align: Axes::splat(FixedAlignment::Start),
            sticky: true,
            movable: false,
        });
    }

    /// Layout vertical spacing.
    fn layout_spacing(
        &mut self,
        engine: &mut Engine,
        v: &Packed<VElem>,
        styles: StyleChain,
    ) -> SourceResult<()> {
        self.layout_item(
            engine,
            match v.amount() {
                Spacing::Rel(rel) => FlowItem::Absolute(
                    rel.resolve(styles).relative_to(self.initial.y),
                    v.weakness(styles) > 0,
                ),
                Spacing::Fr(fr) => FlowItem::Fractional(*fr),
            },
        )
    }

    /// Layout a paragraph.
    fn layout_par(
        &mut self,
        engine: &mut Engine,
        par: &Packed<ParElem>,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let align = AlignElem::alignment_in(styles).resolve(styles);
        let leading = ParElem::leading_in(styles);
        let consecutive = self.last_was_par;
        let lines = par
            .layout(
                engine,
                styles,
                consecutive,
                self.regions.base(),
                self.regions.expand.x,
            )?
            .into_frames();

        if let Some(first) = lines.first() {
            while !self.regions.size.y.fits(first.height()) && !self.regions.in_last() {
                let mut sticky = self.items.len();
                for (i, item) in self.items.iter().enumerate().rev() {
                    match *item {
                        FlowItem::Absolute(_, _) => {}
                        FlowItem::Frame { sticky: true, .. } => sticky = i,
                        _ => break,
                    }
                }

                let carry: Vec<_> = self.items.drain(sticky..).collect();
                self.finish_region(engine, false)?;
                let in_last = self.regions.in_last();

                for item in carry {
                    self.layout_item(engine, item)?;
                }

                if in_last {
                    break;
                }
            }
        }

        for (i, frame) in lines.into_iter().enumerate() {
            if i > 0 {
                self.layout_item(engine, FlowItem::Absolute(leading, true))?;
            }

            self.layout_item(
                engine,
                FlowItem::Frame { frame, align, sticky: false, movable: true },
            )?;
        }

        self.last_was_par = true;
        Ok(())
    }

    /// Layout into a single region.
    fn layout_single(
        &mut self,
        engine: &mut Engine,
        layoutable: &dyn LayoutSingle,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let align = AlignElem::alignment_in(styles).resolve(styles);
        let sticky = BlockElem::sticky_in(styles);
        let pod = Regions::one(self.regions.base(), Axes::splat(false));
        let mut frame = layoutable.layout(engine, styles, pod)?;
        frame.meta(styles, false);
        self.layout_item(
            engine,
            FlowItem::Frame { frame, align, sticky, movable: true },
        )?;
        self.last_was_par = false;
        Ok(())
    }

    /// Layout a placed element.
    fn layout_placed(
        &mut self,
        engine: &mut Engine,
        placed: &Packed<PlaceElem>,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let float = placed.float(styles);
        let clearance = placed.clearance(styles);
        let alignment = placed.alignment(styles);
        let delta = Axes::new(placed.dx(styles), placed.dy(styles)).resolve(styles);
        let x_align = alignment.map_or(FixedAlignment::Center, |align| {
            align.x().unwrap_or_default().resolve(styles)
        });
        let y_align = alignment.map(|align| align.y().map(|y| y.resolve(styles)));
        let mut frame = placed.layout(engine, styles, self.regions.base())?.into_frame();
        frame.meta(styles, false);
        let item = FlowItem::Placed { frame, x_align, y_align, delta, float, clearance };
        self.layout_item(engine, item)
    }

    /// Layout into multiple regions.
    fn layout_multiple(
        &mut self,
        engine: &mut Engine,
        child: &Content,
        styles: StyleChain,
    ) -> SourceResult<()> {
        // Temporarily delegerate rootness to the columns.
        let is_root = self.root;
        if is_root && child.is::<ColumnsElem>() {
            self.root = false;
            self.regions.root = true;
        }

        let mut notes = Vec::new();

        if self.regions.is_full() {
            // Skip directly if region is already full.
            self.finish_region(engine, false)?;
        }

        // How to align the block.
        let align = if let Some(align) = child.to_packed::<AlignElem>() {
            align.alignment(styles)
        } else if let Some(styled) = child.to_packed::<StyledElem>() {
            AlignElem::alignment_in(styles.chain(&styled.styles))
        } else {
            AlignElem::alignment_in(styles)
        }
        .resolve(styles);

        // Layout the block itself.
        let sticky = BlockElem::sticky_in(styles);
        let fragment = child.layout(engine, styles, self.regions)?;

        for (i, mut frame) in fragment.into_iter().enumerate() {
            // Find footnotes in the frame.
            if self.root {
                find_footnotes(&mut notes, &frame);
            }

            if i > 0 {
                self.finish_region(engine, false)?;
            }

            frame.meta(styles, false);
            self.layout_item(
                engine,
                FlowItem::Frame { frame, align, sticky, movable: false },
            )?;
        }

        self.try_handle_footnotes(engine, notes)?;

        self.root = is_root;
        self.regions.root = false;
        self.last_was_par = false;

        Ok(())
    }

    /// Layout a finished frame.
    fn layout_item(
        &mut self,
        engine: &mut Engine,
        mut item: FlowItem,
    ) -> SourceResult<()> {
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
                while !self.regions.size.y.fits(height) && !self.regions.in_last() {
                    self.finish_region(engine, false)?;
                }

                let in_last = self.regions.in_last();
                self.regions.size.y -= height;
                if self.root && movable {
                    let mut notes = Vec::new();
                    find_footnotes(&mut notes, frame);
                    self.items.push(item);

                    // When we are already in_last, we can directly force the
                    // footnotes.
                    if !self.handle_footnotes(engine, &mut notes, true, in_last)? {
                        let item = self.items.pop();
                        self.finish_region(engine, false)?;
                        self.items.extend(item);
                        self.regions.size.y -= height;
                        self.handle_footnotes(engine, &mut notes, true, true)?;
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
                // If there is a queued float in front or if the float doesn't
                // fit, queue it for the next region.
                if !self.pending_floats.is_empty()
                    || (!self.regions.size.y.fits(frame.height() + clearance)
                        && !self.regions.in_last())
                {
                    self.pending_floats.push(item);
                    return Ok(());
                }

                // Select the closer placement, top or bottom.
                if y_align.is_auto() {
                    let ratio = (self.regions.size.y
                        - (frame.height() + clearance) / 2.0)
                        / self.regions.full;
                    let better_align = if ratio <= 0.5 {
                        FixedAlignment::End
                    } else {
                        FixedAlignment::Start
                    };
                    *y_align = Smart::Custom(Some(better_align));
                }

                // Add some clearance so that the float doesn't touch the main
                // content.
                frame.size_mut().y += clearance;
                if *y_align == Smart::Custom(Some(FixedAlignment::End)) {
                    frame.translate(Point::with_y(clearance));
                }

                self.regions.size.y -= frame.height();

                // Find footnotes in the frame.
                if self.root {
                    let mut notes = vec![];
                    find_footnotes(&mut notes, frame);
                    self.try_handle_footnotes(engine, notes)?;
                }
            }
            FlowItem::Footnote(_) => {}
        }

        self.items.push(item);
        Ok(())
    }

    /// Finish the frame for one region.
    ///
    /// Set `force` to `true` to allow creating a frame for out-of-flow elements
    /// only (this is used to force the creation of a frame in case the
    /// remaining elements are all out-of-flow).
    fn finish_region(&mut self, engine: &mut Engine, force: bool) -> SourceResult<()> {
        if !force
            && !self.items.is_empty()
            && self.items.iter().all(FlowItem::is_out_of_flow)
        {
            self.finished.push(Frame::soft(self.initial));
            self.regions.next();
            self.initial = self.regions.size;
            return Ok(());
        }

        // Trim weak spacing.
        while self
            .items
            .last()
            .is_some_and(|item| matches!(item, FlowItem::Absolute(_, true)))
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
                    Smart::Custom(Some(FixedAlignment::Start)) => {
                        float_top_height += frame.height()
                    }
                    Smart::Custom(Some(FixedAlignment::End)) => {
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
        let mut ruler = FixedAlignment::Start;
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
                            Smart::Custom(Some(FixedAlignment::Start)) => {
                                let y = float_top_offset;
                                float_top_offset += frame.height();
                                y
                            }
                            Smart::Custom(Some(FixedAlignment::End)) => {
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

        // Try to place floats into the next region.
        for item in std::mem::take(&mut self.pending_floats) {
            self.layout_item(engine, item)?;
        }

        Ok(())
    }

    /// Finish layouting and return the resulting fragment.
    fn finish(mut self, engine: &mut Engine) -> SourceResult<Fragment> {
        if self.expand.y {
            while !self.regions.backlog.is_empty() {
                self.finish_region(engine, true)?;
            }
        }

        self.finish_region(engine, true)?;
        while !self.items.is_empty() {
            self.finish_region(engine, true)?;
        }

        Ok(Fragment::frames(self.finished))
    }
}

impl FlowLayouter<'_> {
    fn try_handle_footnotes(
        &mut self,
        engine: &mut Engine,
        mut notes: Vec<Packed<FootnoteElem>>,
    ) -> SourceResult<()> {
        // When we are already in_last, we can directly force the
        // footnotes.
        if self.root
            && !self.handle_footnotes(
                engine,
                &mut notes,
                false,
                self.regions.in_last(),
            )?
        {
            self.finish_region(engine, false)?;
            self.handle_footnotes(engine, &mut notes, false, true)?;
        }
        Ok(())
    }

    /// Processes all footnotes in the frame.
    fn handle_footnotes(
        &mut self,
        engine: &mut Engine,
        notes: &mut Vec<Packed<FootnoteElem>>,
        movable: bool,
        force: bool,
    ) -> SourceResult<bool> {
        let prev_notes_len = notes.len();
        let prev_items_len = self.items.len();
        let prev_size = self.regions.size;
        let prev_has_footnotes = self.has_footnotes;
        let prev_locator = engine.locator.clone();

        // Process footnotes one at a time.
        let mut k = 0;
        while k < notes.len() {
            if notes[k].is_ref() {
                k += 1;
                continue;
            }

            if !self.has_footnotes {
                self.layout_footnote_separator(engine)?;
            }

            self.regions.size.y -= self.footnote_config.gap;
            let frames = FootnoteEntry::new(notes[k].clone())
                .pack()
                .layout(engine, self.styles, self.regions.with_root(false))?
                .into_frames();

            // If the entries didn't fit, abort (to keep footnote and entry
            // together).
            if !force
                && (k == 0 || movable)
                && frames.first().is_some_and(Frame::is_empty)
            {
                // Undo everything.
                notes.truncate(prev_notes_len);
                self.items.truncate(prev_items_len);
                self.regions.size = prev_size;
                self.has_footnotes = prev_has_footnotes;
                *engine.locator = prev_locator;
                return Ok(false);
            }

            let prev = notes.len();
            for (i, frame) in frames.into_iter().enumerate() {
                find_footnotes(notes, &frame);
                if i > 0 {
                    self.finish_region(engine, false)?;
                    self.layout_footnote_separator(engine)?;
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
    fn layout_footnote_separator(&mut self, engine: &mut Engine) -> SourceResult<()> {
        let expand = Axes::new(self.regions.expand.x, false);
        let pod = Regions::one(self.regions.base(), expand);
        let separator = &self.footnote_config.separator;

        let mut frame = separator.layout(engine, self.styles, pod)?.into_frame();
        frame.size_mut().y += self.footnote_config.clearance;
        frame.translate(Point::with_y(self.footnote_config.clearance));

        self.has_footnotes = true;
        self.regions.size.y -= frame.height();
        self.items.push(FlowItem::Footnote(frame));

        Ok(())
    }
}

/// Finds all footnotes in the frame.
fn find_footnotes(notes: &mut Vec<Packed<FootnoteElem>>, frame: &Frame) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => find_footnotes(notes, &group.frame),
            FrameItem::Meta(Meta::Elem(content), _)
                if !notes.iter().any(|note| note.location() == content.location()) =>
            {
                let Some(footnote) = content.to_packed::<FootnoteElem>() else {
                    continue;
                };
                notes.push(footnote.clone());
            }
            _ => {}
        }
    }
}
