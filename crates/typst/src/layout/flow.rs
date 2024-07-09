//! Layout flows.
//!
//! A *flow* is a collection of block-level layoutable elements.
//! This is analogous to a paragraph, which is a collection of
//! inline-level layoutable elements.

use std::fmt::{self, Debug, Formatter};

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, Args, Construct, Content, NativeElement, Packed, Resolve, Smart, StyleChain,
};
use crate::introspection::{Locator, SplitLocator, Tag, TagElem};
use crate::layout::{
    Abs, AlignElem, Axes, BlockElem, ColbreakElem, FixedAlignment, FlushElem, Fr,
    Fragment, Frame, FrameItem, PlaceElem, Point, Regions, Rel, Size, Spacing, VElem,
};
use crate::model::{FootnoteElem, FootnoteEntry, ParElem};
use crate::realize::StyleVec;
use crate::utils::Numeric;

/// Arranges spacing, paragraphs and block-level elements into a flow.
///
/// This element is responsible for layouting both the top-level content flow
/// and the contents of boxes.
#[elem(Debug, Construct)]
pub struct FlowElem {
    /// The children that will be arranged into a flow.
    #[internal]
    #[variadic]
    pub children: StyleVec,
}

impl Construct for FlowElem {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}

impl Packed<FlowElem> {
    #[typst_macros::time(name = "flow", span = self.span())]
    pub fn layout(
        &self,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        FlowLayouter::new(engine, self, locator, &styles, regions).layout()
    }
}

impl Debug for FlowElem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Flow ")?;
        self.children.fmt(f)
    }
}

/// Performs flow layout.
struct FlowLayouter<'a, 'e> {
    /// The engine.
    engine: &'a mut Engine<'e>,
    /// The children that will be arranged into a flow.
    flow: &'a Packed<FlowElem>,
    /// Whether this is the root flow.
    root: bool,
    /// Provides unique locations to the flow's children.
    locator: SplitLocator<'a>,
    /// The shared styles.
    styles: &'a StyleChain<'a>,
    /// The regions to layout children into.
    regions: Regions<'a>,
    /// Whether the flow should expand to fill the region.
    expand: Axes<bool>,
    /// The initial size of `regions.size` that was available before we started
    /// subtracting.
    initial: Size,
    /// Whether the last block was a paragraph.
    ///
    /// Used for indenting paragraphs after the first in a block.
    last_was_par: bool,
    /// Spacing and layouted blocks for the current region.
    items: Vec<FlowItem>,
    /// A queue of tags that will be attached to the next frame.
    pending_tags: Vec<&'a Tag>,
    /// A queue of floating elements.
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
    /// A frame for a layouted block.
    Frame {
        /// The frame itself.
        frame: Frame,
        /// How to align the frame.
        align: Axes<FixedAlignment>,
        /// Whether the frame sticks to the item after it (for orphan prevention).
        sticky: bool,
        /// Whether the frame is movable; that is, kept together with its
        /// footnotes.
        ///
        /// This is true for frames created by paragraphs and
        /// [`BlockElem::single_layouter`] elements.
        movable: bool,
    },
    /// An absolutely placed frame.
    Placed {
        /// The layouted content.
        frame: Frame,
        /// Where to place the content horizontally.
        x_align: FixedAlignment,
        /// Where to place the content vertically.
        y_align: Smart<Option<FixedAlignment>>,
        /// A translation to apply to the content.
        delta: Axes<Rel<Abs>>,
        /// Whether the content floats --- i.e. collides with in-flow content.
        float: bool,
        /// The amount of space that needs to be kept between the placed content
        /// and in-flow content. Only relevant if `float` is `true`.
        clearance: Abs,
    },
    /// A footnote frame (can also be the separator).
    Footnote(Frame),
}

impl FlowItem {
    /// Whether this item is out-of-flow.
    ///
    /// Out-of-flow items are guaranteed to have a [zero size][Size::zero()].
    fn is_out_of_flow(&self) -> bool {
        match self {
            Self::Placed { float: false, .. } => true,
            Self::Frame { frame, .. } => {
                frame.size().is_zero()
                    && frame.items().all(|(_, item)| {
                        matches!(item, FrameItem::Link(_, _) | FrameItem::Tag(_))
                    })
            }
            _ => false,
        }
    }
}

impl<'a, 'e> FlowLayouter<'a, 'e> {
    /// Create a new flow layouter.
    fn new(
        engine: &'a mut Engine<'e>,
        flow: &'a Packed<FlowElem>,
        locator: Locator<'a>,
        styles: &'a StyleChain<'a>,
        mut regions: Regions<'a>,
    ) -> Self {
        // Check whether we have just a single multiple-layoutable element. In
        // that case, we do not set `expand.y` to `false`, but rather keep it at
        // its original value (since that element can take the full space).
        //
        // Consider the following code: `block(height: 5cm, pad(10pt,
        // align(bottom, ..)))`. Thanks to the code below, the expansion will be
        // passed all the way through the block & pad and reach the innermost
        // flow, so that things are properly bottom-aligned.
        let mut alone = false;
        if let [child] = flow.children.elements() {
            alone = child.is::<BlockElem>();
        }

        // Disable vertical expansion when there are multiple or not directly
        // layoutable children.
        let expand = regions.expand;
        if !alone {
            regions.expand.y = false;
        }

        // The children aren't root.
        let root = std::mem::replace(&mut regions.root, false);

        Self {
            engine,
            flow,
            root,
            locator: locator.split(),
            styles,
            regions,
            expand,
            initial: regions.size,
            last_was_par: false,
            items: vec![],
            pending_tags: vec![],
            pending_floats: vec![],
            has_footnotes: false,
            footnote_config: FootnoteConfig {
                separator: FootnoteEntry::separator_in(*styles),
                clearance: FootnoteEntry::clearance_in(*styles),
                gap: FootnoteEntry::gap_in(*styles),
            },
            finished: vec![],
        }
    }

    /// Layout the flow.
    fn layout(mut self) -> SourceResult<Fragment> {
        for (child, styles) in self.flow.children.chain(self.styles) {
            if let Some(elem) = child.to_packed::<TagElem>() {
                self.handle_tag(elem);
            } else if let Some(elem) = child.to_packed::<VElem>() {
                self.handle_v(elem, styles)?;
            } else if let Some(elem) = child.to_packed::<ColbreakElem>() {
                self.handle_colbreak(elem)?;
            } else if let Some(elem) = child.to_packed::<ParElem>() {
                self.handle_par(elem, styles)?;
            } else if let Some(elem) = child.to_packed::<BlockElem>() {
                self.handle_block(elem, styles)?;
            } else if let Some(elem) = child.to_packed::<PlaceElem>() {
                self.handle_place(elem, styles)?;
            } else if let Some(elem) = child.to_packed::<FlushElem>() {
                self.handle_flush(elem)?;
            } else {
                bail!(child.span(), "unexpected flow child");
            }
        }

        self.finish()
    }

    /// Place explicit metadata into the flow.
    fn handle_tag(&mut self, elem: &'a Packed<TagElem>) {
        self.pending_tags.push(&elem.tag);
    }

    /// Layout vertical spacing.
    fn handle_v(&mut self, v: &'a Packed<VElem>, styles: StyleChain) -> SourceResult<()> {
        self.handle_item(match v.amount {
            Spacing::Rel(rel) => FlowItem::Absolute(
                // Resolve the spacing relative to the current base height.
                rel.resolve(styles).relative_to(self.initial.y),
                v.weakness(styles) > 0,
            ),
            Spacing::Fr(fr) => FlowItem::Fractional(fr),
        })
    }

    /// Layout a column break.
    fn handle_colbreak(&mut self, _: &'a Packed<ColbreakElem>) -> SourceResult<()> {
        // If there is still an available region, skip to it.
        // TODO: Turn this into a region abstraction.
        if !self.regions.backlog.is_empty() || self.regions.last.is_some() {
            self.finish_region(true)?;
        }
        Ok(())
    }

    /// Layout a paragraph.
    fn handle_par(
        &mut self,
        par: &'a Packed<ParElem>,
        styles: StyleChain,
    ) -> SourceResult<()> {
        // Fetch properties.
        let align = AlignElem::alignment_in(styles).resolve(styles);
        let leading = ParElem::leading_in(styles);

        // Layout the paragraph into lines. This only depends on the base size,
        // not on the Y position.
        let consecutive = self.last_was_par;
        let locator = self.locator.next(&par.span());
        let lines = par
            .layout(
                self.engine,
                locator,
                styles,
                consecutive,
                self.regions.base(),
                self.regions.expand.x,
            )?
            .into_frames();

        // If the first line doesn’t fit in this region, then defer any
        // previous sticky frame to the next region (if available)
        if let Some(first) = lines.first() {
            while !self.regions.size.y.fits(first.height()) && !self.regions.in_last() {
                let in_last = self.finish_region_with_migration()?;
                if in_last {
                    break;
                }
            }
        }

        // Layout the lines.
        for (i, mut frame) in lines.into_iter().enumerate() {
            if i > 0 {
                self.handle_item(FlowItem::Absolute(leading, true))?;
            }

            self.drain_tag(&mut frame);
            self.handle_item(FlowItem::Frame {
                frame,
                align,
                sticky: false,
                movable: true,
            })?;
        }

        self.last_was_par = true;
        Ok(())
    }

    /// Layout into multiple regions.
    fn handle_block(
        &mut self,
        block: &'a Packed<BlockElem>,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        // Fetch properties.
        let sticky = block.sticky(styles);
        let align = AlignElem::alignment_in(styles).resolve(styles);

        // If the block is "rootable" it may host footnotes. In that case, we
        // defer rootness to it temporarily. We disable our own rootness to
        // prevent duplicate footnotes.
        let is_root = self.root;
        if is_root && block.rootable(styles) {
            self.root = false;
            self.regions.root = true;
        }

        // Skip directly if region is already full.
        if self.regions.is_full() {
            self.finish_region(false)?;
        }

        // Layout the block itself.
        let fragment = block.layout(
            self.engine,
            self.locator.next(&block.span()),
            styles,
            self.regions,
        )?;

        let mut notes = Vec::new();
        for (i, mut frame) in fragment.into_iter().enumerate() {
            // Find footnotes in the frame.
            if self.root {
                collect_footnotes(&mut notes, &frame);
            }

            if i > 0 {
                self.finish_region(false)?;
            }

            self.drain_tag(&mut frame);
            frame.post_process(styles);
            self.handle_item(FlowItem::Frame { frame, align, sticky, movable: false })?;
        }

        self.try_handle_footnotes(notes)?;

        self.root = is_root;
        self.regions.root = false;
        self.last_was_par = false;

        Ok(())
    }

    /// Layout a placed element.
    fn handle_place(
        &mut self,
        placed: &'a Packed<PlaceElem>,
        styles: StyleChain,
    ) -> SourceResult<()> {
        // Fetch properties.
        let float = placed.float(styles);
        let clearance = placed.clearance(styles);
        let alignment = placed.alignment(styles);
        let delta = Axes::new(placed.dx(styles), placed.dy(styles)).resolve(styles);

        let x_align = alignment.map_or(FixedAlignment::Center, |align| {
            align.x().unwrap_or_default().resolve(styles)
        });
        let y_align = alignment.map(|align| align.y().map(|y| y.resolve(styles)));

        let mut frame = placed
            .layout(
                self.engine,
                self.locator.next(&placed.span()),
                styles,
                self.regions.base(),
            )?
            .into_frame();

        frame.post_process(styles);

        self.handle_item(FlowItem::Placed {
            frame,
            x_align,
            y_align,
            delta,
            float,
            clearance,
        })
    }

    /// Lays out all floating elements before continuing with other content.
    fn handle_flush(&mut self, _: &'a Packed<FlushElem>) -> SourceResult<()> {
        for item in std::mem::take(&mut self.pending_floats) {
            self.handle_item(item)?;
        }
        while !self.pending_floats.is_empty() {
            self.finish_region(false)?;
        }
        Ok(())
    }

    /// Layout a finished frame.
    fn handle_item(&mut self, mut item: FlowItem) -> SourceResult<()> {
        match item {
            FlowItem::Absolute(v, weak) => {
                if weak
                    && !self
                        .items
                        .iter()
                        .any(|item| matches!(item, FlowItem::Frame { .. },))
                {
                    return Ok(());
                }
                self.regions.size.y -= v
            }
            FlowItem::Fractional(..) => {}
            FlowItem::Frame { ref frame, movable, .. } => {
                let height = frame.height();
                while !self.regions.size.y.fits(height) && !self.regions.in_last() {
                    self.finish_region(false)?;
                }

                let in_last = self.regions.in_last();
                self.regions.size.y -= height;
                if self.root && movable {
                    let mut notes = Vec::new();
                    collect_footnotes(&mut notes, frame);
                    self.items.push(item);

                    // When we are already in_last, we can directly force the
                    // footnotes.
                    if !self.handle_footnotes(&mut notes, true, in_last)? {
                        let item = self.items.pop();
                        self.finish_region(false)?;
                        self.items.extend(item);
                        self.regions.size.y -= height;
                        self.handle_footnotes(&mut notes, true, true)?;
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
                    collect_footnotes(&mut notes, frame);
                    self.try_handle_footnotes(notes)?;
                }
            }
            FlowItem::Footnote(_) => {}
        }

        self.items.push(item);
        Ok(())
    }

    /// Attach currently pending metadata to the frame.
    fn drain_tag(&mut self, frame: &mut Frame) {
        if !self.pending_tags.is_empty() && !frame.is_empty() {
            frame.prepend_multiple(
                self.pending_tags
                    .drain(..)
                    .map(|tag| (Point::zero(), FrameItem::Tag(tag.clone()))),
            );
        }
    }

    /// Finisht the region, migrating all sticky items to the next one.
    ///
    /// Returns whether we migrated into a last region.
    fn finish_region_with_migration(&mut self) -> SourceResult<bool> {
        // Find the suffix of sticky items.
        let mut sticky = self.items.len();
        for (i, item) in self.items.iter().enumerate().rev() {
            match *item {
                FlowItem::Absolute(_, _) => {}
                FlowItem::Frame { sticky: true, .. } => sticky = i,
                _ => break,
            }
        }

        let carry: Vec<_> = self.items.drain(sticky..).collect();
        self.finish_region(false)?;

        let in_last = self.regions.in_last();
        for item in carry {
            self.handle_item(item)?;
        }

        Ok(in_last)
    }

    /// Finish the frame for one region.
    ///
    /// Set `force` to `true` to allow creating a frame for out-of-flow elements
    /// only (this is used to force the creation of a frame in case the
    /// remaining elements are all out-of-flow).
    fn finish_region(&mut self, force: bool) -> SourceResult<()> {
        // Early return if we don't have any relevant items.
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

        if !self.regions.size.x.is_finite() && self.expand.x {
            bail!(self.flow.span(), "cannot expand into infinite width");
        }
        if !self.regions.size.y.is_finite() && self.expand.y {
            bail!(self.flow.span(), "cannot expand into infinite height");
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
                    let length = v.share(fr, remaining);
                    offset += length;
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

        if force && !self.pending_tags.is_empty() {
            let pos = Point::with_y(offset);
            output.push_multiple(
                self.pending_tags
                    .drain(..)
                    .map(|tag| (pos, FrameItem::Tag(tag.clone()))),
            );
        }

        // Advance to the next region.
        self.finished.push(output);
        self.regions.next();
        self.initial = self.regions.size;
        self.has_footnotes = false;

        // Try to place floats into the next region.
        for item in std::mem::take(&mut self.pending_floats) {
            self.handle_item(item)?;
        }

        Ok(())
    }

    /// Finish layouting and return the resulting fragment.
    fn finish(mut self) -> SourceResult<Fragment> {
        if self.expand.y {
            while !self.regions.backlog.is_empty() {
                self.finish_region(true)?;
            }
        }

        self.finish_region(true)?;
        while !self.items.is_empty() {
            self.finish_region(true)?;
        }

        Ok(Fragment::frames(self.finished))
    }

    /// Tries to process all footnotes in the frame, placing them
    /// in the next region if they could not be placed in the current
    /// one.
    fn try_handle_footnotes(
        &mut self,
        mut notes: Vec<Packed<FootnoteElem>>,
    ) -> SourceResult<()> {
        // When we are already in_last, we can directly force the
        // footnotes.
        if self.root
            && !self.handle_footnotes(&mut notes, false, self.regions.in_last())?
        {
            self.finish_region(false)?;
            self.handle_footnotes(&mut notes, false, true)?;
        }
        Ok(())
    }

    /// Processes all footnotes in the frame.
    ///
    /// Returns true if the footnote entries fit in the allotted
    /// regions.
    fn handle_footnotes(
        &mut self,
        notes: &mut Vec<Packed<FootnoteElem>>,
        movable: bool,
        force: bool,
    ) -> SourceResult<bool> {
        let prev_notes_len = notes.len();
        let prev_items_len = self.items.len();
        let prev_size = self.regions.size;
        let prev_has_footnotes = self.has_footnotes;

        // Process footnotes one at a time.
        let mut k = 0;
        while k < notes.len() {
            if notes[k].is_ref() {
                k += 1;
                continue;
            }

            if !self.has_footnotes {
                self.layout_footnote_separator()?;
            }

            self.regions.size.y -= self.footnote_config.gap;
            let frames = FootnoteEntry::new(notes[k].clone())
                .pack()
                .layout(
                    self.engine,
                    Locator::synthesize(notes[k].location().unwrap()),
                    *self.styles,
                    self.regions.with_root(false),
                )?
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
                return Ok(false);
            }

            let prev = notes.len();
            for (i, frame) in frames.into_iter().enumerate() {
                collect_footnotes(notes, &frame);
                if i > 0 {
                    self.finish_region(false)?;
                    self.layout_footnote_separator()?;
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
    fn layout_footnote_separator(&mut self) -> SourceResult<()> {
        let expand = Axes::new(self.regions.expand.x, false);
        let pod = Regions::one(self.regions.base(), expand);
        let separator = &self.footnote_config.separator;

        // FIXME: Shouldn't use `root()` here.
        let mut frame = separator
            .layout(self.engine, Locator::root(), *self.styles, pod)?
            .into_frame();
        frame.size_mut().y += self.footnote_config.clearance;
        frame.translate(Point::with_y(self.footnote_config.clearance));

        self.has_footnotes = true;
        self.regions.size.y -= frame.height();
        self.items.push(FlowItem::Footnote(frame));

        Ok(())
    }
}

/// Collect all footnotes in a frame.
fn collect_footnotes(notes: &mut Vec<Packed<FootnoteElem>>, frame: &Frame) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => collect_footnotes(notes, &group.frame),
            FrameItem::Tag(tag)
                if !notes.iter().any(|note| note.location() == tag.elem.location()) =>
            {
                let Some(footnote) = tag.elem.to_packed::<FootnoteElem>() else {
                    continue;
                };
                notes.push(footnote.clone());
            }
            _ => {}
        }
    }
}
