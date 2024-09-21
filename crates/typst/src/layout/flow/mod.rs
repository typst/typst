//! Layout of content
//! - at the top-level, into a [`Document`].
//! - inside of a container, into a [`Frame`] or [`Fragment`].

mod collect;

use std::collections::HashSet;
use std::num::NonZeroUsize;

use bumpalo::Bump;
use comemo::{Track, Tracked, TrackedMut};

use self::collect::{collect, BlockChild, Child, LineChild, PlacedChild};
use crate::diag::{bail, At, SourceResult};
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{
    Content, NativeElement, Packed, Resolve, SequenceElem, Smart, StyleChain,
};
use crate::introspection::{
    Counter, CounterDisplayElem, CounterKey, CounterState, CounterUpdate, Introspector,
    Location, Locator, LocatorLink, SplitLocator, Tag,
};
use crate::layout::{
    Abs, Axes, BlockElem, Dir, FixedAlignment, Fr, Fragment, Frame, FrameItem,
    OuterHAlignment, Point, Region, Regions, Rel, Size,
};
use crate::model::{
    FootnoteElem, FootnoteEntry, ParLine, ParLineMarker, ParLineNumberingScope,
};
use crate::realize::{realize, Arenas, Pair, RealizationKind};
use crate::syntax::Span;
use crate::text::TextElem;
use crate::utils::{NonZeroExt, Numeric};
use crate::World;

/// Layout content into multiple regions.
///
/// When just layouting into a single region, prefer [`layout_frame`].
pub fn layout_fragment(
    engine: &mut Engine,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    layout_fragment_impl(
        engine.world,
        engine.introspector,
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        locator.track(),
        styles,
        regions,
        NonZeroUsize::ONE,
        Rel::zero(),
    )
}

/// Layout content into regions with columns.
///
/// For now, this just invokes normal layout on cycled smaller regions. However,
/// in the future, columns will be able to interact (e.g. through floating
/// figures), so this is already factored out because it'll be conceptually
/// different from just layouting into more smaller regions.
pub fn layout_fragment_with_columns(
    engine: &mut Engine,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
    count: NonZeroUsize,
    gutter: Rel<Abs>,
) -> SourceResult<Fragment> {
    layout_fragment_impl(
        engine.world,
        engine.introspector,
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        locator.track(),
        styles,
        regions,
        count,
        gutter,
    )
}

/// Layout content into a single region.
pub fn layout_frame(
    engine: &mut Engine,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    layout_fragment(engine, content, locator, styles, region.into())
        .map(Fragment::into_frame)
}

/// The internal implementation of [`layout_fragment`].
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn layout_fragment_impl(
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    locator: Tracked<Locator>,
    styles: StyleChain,
    regions: Regions,
    columns: NonZeroUsize,
    column_gutter: Rel<Abs>,
) -> SourceResult<Fragment> {
    let link = LocatorLink::new(locator);
    let mut locator = Locator::link(&link).split();
    let mut engine = Engine {
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route),
    };

    engine.route.check_layout_depth().at(content.span())?;

    let arenas = Arenas::default();
    let children = realize(
        RealizationKind::Container,
        &mut engine,
        &mut locator,
        &arenas,
        content,
        styles,
    )?;

    layout_flow(
        &mut engine,
        &arenas.bump,
        &children,
        &mut locator,
        styles,
        regions,
        columns,
        column_gutter,
        content.span(),
    )
}

/// Layout flow content.
#[allow(clippy::too_many_arguments)]
pub(crate) fn layout_flow(
    engine: &mut Engine,
    bump: &Bump,
    children: &[Pair],
    locator: &mut SplitLocator,
    shared: StyleChain,
    regions: Regions,
    columns: NonZeroUsize,
    column_gutter: Rel<Abs>,
    span: Span,
) -> SourceResult<Fragment> {
    // Separating the infinite space into infinite columns does not make
    // much sense.
    let mut columns = columns.get();
    if !regions.size.x.is_finite() {
        columns = 1;
    }

    // Determine the width of the gutter and each column.
    let column_gutter = column_gutter.relative_to(regions.base().x);

    let backlog: Vec<Abs>;
    let mut pod = if columns > 1 {
        backlog = std::iter::once(&regions.size.y)
            .chain(regions.backlog)
            .flat_map(|&height| std::iter::repeat(height).take(columns))
            .skip(1)
            .collect();

        let width =
            (regions.size.x - column_gutter * (columns - 1) as f64) / columns as f64;

        // Create the pod regions.
        Regions {
            size: Size::new(width, regions.size.y),
            full: regions.full,
            backlog: &backlog,
            last: regions.last,
            expand: Axes::new(true, regions.expand.y),
            root: regions.root,
        }
    } else {
        regions
    };

    // The children aren't root.
    pod.root = false;

    // Check whether we have just a single multiple-layoutable element. In
    // that case, we do not set `expand.y` to `false`, but rather keep it at
    // its original value (since that element can take the full space).
    //
    // Consider the following code: `block(height: 5cm, pad(10pt,
    // align(bottom, ..)))`. Thanks to the code below, the expansion will be
    // passed all the way through the block & pad and reach the innermost
    // flow, so that things are properly bottom-aligned.
    let mut alone = false;
    if let [(child, _)] = children {
        alone = child.is::<BlockElem>();
    }

    // Disable vertical expansion when there are multiple or not directly
    // layoutable children.
    if !alone {
        pod.expand.y = false;
    }

    let children =
        collect(engine, bump, children, locator.next(&()), pod.base(), pod.expand.x)?;

    let layouter = FlowLayouter {
        engine,
        span,
        root: regions.root,
        locator,
        shared,
        columns,
        column_gutter,
        regions: pod,
        expand: regions.expand,
        initial: pod.size,
        items: vec![],
        pending_tags: vec![],
        pending_floats: vec![],
        has_footnotes: false,
        footnote_config: FootnoteConfig {
            separator: FootnoteEntry::separator_in(shared),
            clearance: FootnoteEntry::clearance_in(shared),
            gap: FootnoteEntry::gap_in(shared),
        },
        visited_footnotes: HashSet::new(),
        finished: vec![],
    };

    layouter.layout(&children, regions)
}

/// Layouts a collection of block-level elements.
struct FlowLayouter<'a, 'b, 'x, 'y> {
    /// The engine.
    engine: &'a mut Engine<'x>,
    /// A span to use for errors.
    span: Span,
    /// Whether this is the root flow.
    root: bool,
    /// Provides unique locations to the flow's children.
    locator: &'a mut SplitLocator<'y>,
    /// The shared styles.
    shared: StyleChain<'a>,
    /// The number of columns.
    columns: usize,
    /// The gutter between columns.
    column_gutter: Abs,
    /// The regions to layout children into. These already incorporate the
    /// columns.
    regions: Regions<'a>,
    /// Whether the flow should expand to fill the region.
    expand: Axes<bool>,
    /// The initial size of `regions.size` that was available before we started
    /// subtracting.
    initial: Size,
    /// Spacing and layouted blocks for the current region.
    items: Vec<FlowItem<'a, 'b>>,
    /// A queue of tags that will be attached to the next frame.
    pending_tags: Vec<&'a Tag>,
    /// A queue of floating elements.
    pending_floats: Vec<FlowItem<'a, 'b>>,
    /// Whether we have any footnotes in the current region.
    has_footnotes: bool,
    /// Footnote configuration.
    footnote_config: FootnoteConfig,
    /// Footnotes that we have already processed.
    visited_footnotes: HashSet<Location>,
    /// Finished frames for previous regions.
    finished: Vec<Frame>,
}

/// Cached footnote configuration.
struct FootnoteConfig {
    separator: Content,
    clearance: Abs,
    gap: Abs,
}

/// Information needed to generate a line number.
struct CollectedParLine {
    y: Abs,
    marker: Packed<ParLineMarker>,
}

/// A prepared item in a flow layout.
enum FlowItem<'a, 'b> {
    /// Spacing between other items and its weakness level.
    Absolute(Abs, u8),
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
        /// Whether the frame comes from a rootable block, which may be laid
        /// out as a root flow and thus display its own line numbers.
        /// Therefore, we do not display line numbers for these frames.
        ///
        /// Currently, this is only used by columns.
        rootable: bool,
        /// Whether the frame is movable; that is, kept together with its
        /// footnotes.
        ///
        /// This is true for frames created by paragraphs and
        /// [`BlockElem::single_layouter`] elements.
        movable: bool,
    },
    /// An absolutely placed frame.
    Placed(&'b PlacedChild<'a>, Frame, Smart<Option<FixedAlignment>>),
    /// A footnote frame (can also be the separator).
    Footnote(Frame),
}

impl FlowItem<'_, '_> {
    /// Whether this item is out-of-flow.
    ///
    /// Out-of-flow items are guaranteed to have a [zero size][Size::zero()].
    fn is_out_of_flow(&self) -> bool {
        match self {
            Self::Placed(placed, ..) => !placed.float,
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

impl<'a, 'b, 'x, 'y> FlowLayouter<'a, 'b, 'x, 'y> {
    /// Layout the flow.
    fn layout(
        mut self,
        children: &'b [Child<'a>],
        regions: Regions,
    ) -> SourceResult<Fragment> {
        for child in children {
            match child {
                Child::Tag(tag) => {
                    self.pending_tags.push(tag);
                }
                Child::Rel(amount, weakness) => {
                    self.handle_rel(*amount, *weakness)?;
                }
                Child::Fr(fr) => {
                    self.handle_item(FlowItem::Fractional(*fr))?;
                }
                Child::Line(line) => {
                    self.handle_line(line)?;
                }
                Child::Block(block) => {
                    self.handle_block(block)?;
                }
                Child::Placed(placed) => {
                    self.handle_placed(placed)?;
                }
                Child::Break(weak) => {
                    self.handle_colbreak(*weak)?;
                }
                Child::Flush => {
                    self.handle_flush()?;
                }
            }
        }

        self.finish(regions)
    }

    /// Layout relative spacing, handling weakness.
    fn handle_rel(&mut self, amount: Rel<Abs>, weakness: u8) -> SourceResult<()> {
        self.handle_item(FlowItem::Absolute(
            // Resolve the spacing relative to the current base height.
            amount.relative_to(self.initial.y),
            weakness,
        ))
    }

    /// Layout a paragraph.
    fn handle_line(&mut self, line: &LineChild) -> SourceResult<()> {
        // If the first line doesn’t fit in this region, then defer any
        // previous sticky frame to the next region (if available)
        if !self.regions.in_last()
            && !self.regions.size.y.fits(line.need)
            && self
                .regions
                .iter()
                .nth(1)
                .is_some_and(|region| region.y.fits(line.need))
        {
            self.finish_region_with_migration()?;
        }

        let mut frame = line.frame.clone();
        self.drain_tag(&mut frame);
        self.handle_item(FlowItem::Frame {
            frame,
            align: line.align,
            sticky: false,
            rootable: false,
            movable: true,
        })
    }

    /// Layout into multiple regions.
    fn handle_block(&mut self, block: &BlockChild) -> SourceResult<()> {
        // If the block is "rootable" it may host footnotes. In that case, we
        // defer rootness to it temporarily. We disable our own rootness to
        // prevent duplicate footnotes.
        let is_root = self.root;
        if is_root && block.rootable {
            self.root = false;
            self.regions.root = true;
        }

        // Skip directly if region is already full.
        if self.regions.is_full() {
            self.finish_region(false)?;
        }

        // Layout the block itself.
        let fragment = block.layout(self.engine, self.regions)?;

        let mut notes = Vec::new();
        for (i, mut frame) in fragment.into_iter().enumerate() {
            // Find footnotes in the frame.
            if self.root {
                self.collect_footnotes(&mut notes, &frame);
            }

            if i > 0 {
                self.finish_region(false)?;
            }

            self.drain_tag(&mut frame);
            self.handle_item(FlowItem::Frame {
                frame,
                align: block.align,
                sticky: block.sticky,
                rootable: block.rootable,
                movable: false,
            })?;
        }

        self.try_handle_footnotes(notes)?;

        self.root = is_root;
        self.regions.root = false;

        Ok(())
    }

    /// Layout a placed element.
    fn handle_placed(&mut self, placed: &'b PlacedChild<'a>) -> SourceResult<()> {
        let frame = placed.layout(self.engine, self.regions.base())?;
        self.handle_item(FlowItem::Placed(placed, frame, placed.align_y))
    }

    /// Layout a column break.
    fn handle_colbreak(&mut self, _weak: bool) -> SourceResult<()> {
        // If there is still an available region, skip to it.
        // TODO: Turn this into a region abstraction.
        if !self.regions.backlog.is_empty() || self.regions.last.is_some() {
            self.finish_region(true)?;
        }
        Ok(())
    }

    /// Lays out all floating elements before continuing with other content.
    fn handle_flush(&mut self) -> SourceResult<()> {
        for item in std::mem::take(&mut self.pending_floats) {
            self.handle_item(item)?;
        }
        while !self.pending_floats.is_empty() {
            self.finish_region(false)?;
        }
        Ok(())
    }

    /// Layout a finished frame.
    fn handle_item(&mut self, mut item: FlowItem<'a, 'b>) -> SourceResult<()> {
        match item {
            FlowItem::Absolute(v, weakness) => {
                if weakness > 0 {
                    let mut has_frame = false;
                    for prev in self.items.iter_mut().rev() {
                        match prev {
                            FlowItem::Frame { .. } => {
                                has_frame = true;
                                break;
                            }
                            FlowItem::Absolute(prev_amount, prev_level)
                                if *prev_level > 0 =>
                            {
                                if *prev_level >= weakness {
                                    let diff = v - *prev_amount;
                                    if *prev_level > weakness || diff > Abs::zero() {
                                        self.regions.size.y -= diff;
                                        *prev = item;
                                    }
                                }
                                return Ok(());
                            }
                            FlowItem::Fractional(_) => return Ok(()),
                            _ => {}
                        }
                    }
                    if !has_frame {
                        return Ok(());
                    }
                }
                self.regions.size.y -= v;
            }
            FlowItem::Fractional(..) => {
                self.trim_weak_spacing();
            }
            FlowItem::Frame { ref frame, movable, .. } => {
                let height = frame.height();
                while !self.regions.size.y.fits(height) && !self.regions.in_last() {
                    self.finish_region(false)?;
                }

                let in_last = self.regions.in_last();
                self.regions.size.y -= height;
                if self.root && movable {
                    let mut notes = Vec::new();
                    self.collect_footnotes(&mut notes, frame);
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
            FlowItem::Placed(placed, ..) if !placed.float => {}
            FlowItem::Placed(placed, ref mut frame, ref mut align_y) => {
                // If there is a queued float in front or if the float doesn't
                // fit, queue it for the next region.
                if !self.pending_floats.is_empty()
                    || (!self.regions.size.y.fits(frame.height() + placed.clearance)
                        && !self.regions.in_last())
                {
                    self.pending_floats.push(item);
                    return Ok(());
                }

                // Select the closer placement, top or bottom.
                if align_y.is_auto() {
                    // When the figure's vertical midpoint would be above the
                    // middle of the page if it were layouted in-flow, we use
                    // top alignment. Otherwise, we use bottom alignment.
                    let used = self.regions.full - self.regions.size.y;
                    let half = (frame.height() + placed.clearance) / 2.0;
                    let ratio = (used + half) / self.regions.full;
                    let better_align = if ratio <= 0.5 {
                        FixedAlignment::Start
                    } else {
                        FixedAlignment::End
                    };
                    *align_y = Smart::Custom(Some(better_align));
                }

                // Add some clearance so that the float doesn't touch the main
                // content.
                frame.size_mut().y += placed.clearance;
                if *align_y == Smart::Custom(Some(FixedAlignment::End)) {
                    frame.translate(Point::with_y(placed.clearance));
                }

                self.regions.size.y -= frame.height();

                // Find footnotes in the frame.
                if self.root {
                    let mut notes = vec![];
                    self.collect_footnotes(&mut notes, frame);
                    self.try_handle_footnotes(notes)?;
                }
            }
            FlowItem::Footnote(_) => {}
        }

        self.items.push(item);
        Ok(())
    }

    /// Trim trailing weak spacing from the items.
    fn trim_weak_spacing(&mut self) {
        for (i, item) in self.items.iter().enumerate().rev() {
            match item {
                FlowItem::Absolute(amount, 1..) => {
                    self.regions.size.y += *amount;
                    self.items.remove(i);
                    return;
                }
                FlowItem::Frame { .. } => return,
                _ => {}
            }
        }
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
    fn finish_region_with_migration(&mut self) -> SourceResult<()> {
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

        for item in carry {
            self.handle_item(item)?;
        }

        Ok(())
    }

    /// Finish the frame for one region.
    ///
    /// Set `force` to `true` to allow creating a frame for out-of-flow elements
    /// only (this is used to force the creation of a frame in case the
    /// remaining elements are all out-of-flow).
    fn finish_region(&mut self, force: bool) -> SourceResult<()> {
        self.trim_weak_spacing();

        // Early return if we don't have any relevant items.
        if !force
            && !self.items.is_empty()
            && self.items.iter().all(FlowItem::is_out_of_flow)
        {
            // Run line number layout here even though we have no line numbers
            // to ensure we reset line numbers at the start of the page if
            // requested, which is still necessary if e.g. the first column is
            // empty when the others aren't.
            let mut output = Frame::soft(self.initial);
            self.layout_line_numbers(&mut output, self.initial, vec![])?;

            self.finished.push(output);
            self.regions.next();
            self.initial = self.regions.size;
            return Ok(());
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
                FlowItem::Placed(placed, ..) if !placed.float => {}
                FlowItem::Placed(_, frame, align_y) => match align_y {
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
            bail!(self.span, "cannot expand into infinite width");
        }
        if !self.regions.size.y.is_finite() && self.expand.y {
            bail!(self.span, "cannot expand into infinite height");
        }

        let mut output = Frame::soft(size);
        let mut ruler = FixedAlignment::Start;
        let mut float_top_offset = Abs::zero();
        let mut offset = float_top_height;
        let mut float_bottom_offset = Abs::zero();
        let mut footnote_offset = Abs::zero();

        let mut lines: Vec<CollectedParLine> = vec![];

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
                FlowItem::Frame { frame, align, rootable, .. } => {
                    ruler = ruler.max(align.y);
                    let x = align.x.position(size.x - frame.width());
                    let y = offset + ruler.position(size.y - used.y);
                    let pos = Point::new(x, y);
                    offset += frame.height();

                    // Do not display line numbers for frames coming from
                    // rootable blocks as they will display their own line
                    // numbers when laid out as a root flow themselves.
                    if self.root && !rootable {
                        collect_par_lines(&mut lines, &frame, pos, Abs::zero());
                    }

                    output.push_frame(pos, frame);
                }
                FlowItem::Placed(placed, frame, align_y) => {
                    let x = placed.align_x.position(size.x - frame.width());
                    let y = if placed.float {
                        match align_y {
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
                        match align_y {
                            Smart::Custom(Some(align)) => {
                                align.position(size.y - frame.height())
                            }
                            _ => offset + ruler.position(size.y - used.y),
                        }
                    };

                    let pos = Point::new(x, y)
                        + placed.delta.zip_map(size, Rel::relative_to).to_point();

                    if self.root {
                        collect_par_lines(&mut lines, &frame, pos, Abs::zero());
                    }

                    output.push_frame(pos, frame);
                }
                FlowItem::Footnote(frame) => {
                    let y = size.y - footnote_height + footnote_offset;
                    footnote_offset += frame.height() + self.footnote_config.gap;
                    output.push_frame(Point::with_y(y), frame);
                }
            }
        }

        // Sort, deduplicate and layout line numbers.
        //
        // We do this after placing all frames since they might not necessarily
        // be ordered by height (e.g. you can have a `place(bottom)` followed
        // by a paragraph, but the paragraph appears at the top), so we buffer
        // all line numbers to later sort and deduplicate them based on how
        // close they are to each other in `layout_line_numbers`.
        self.layout_line_numbers(&mut output, size, lines)?;

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
    fn finish(mut self, regions: Regions) -> SourceResult<Fragment> {
        if self.expand.y {
            while !self.regions.backlog.is_empty() {
                self.finish_region(true)?;
            }
        }

        self.finish_region(true)?;
        while !self.items.is_empty() {
            self.finish_region(true)?;
        }

        if self.columns == 1 {
            return Ok(Fragment::frames(self.finished));
        }

        // Stitch together the column for each region.
        let dir = TextElem::dir_in(self.shared);
        let total = (self.finished.len() as f32 / self.columns as f32).ceil() as usize;

        let mut collected = vec![];
        let mut iter = self.finished.into_iter();
        for region in regions.iter().take(total) {
            // The height should be the parent height if we should expand.
            // Otherwise its the maximum column height for the frame. In that
            // case, the frame is first created with zero height and then
            // resized.
            let height = if regions.expand.y { region.y } else { Abs::zero() };
            let mut output = Frame::hard(Size::new(regions.size.x, height));
            let mut cursor = Abs::zero();

            for _ in 0..self.columns {
                let Some(frame) = iter.next() else { break };
                if !regions.expand.y {
                    output.size_mut().y.set_max(frame.height());
                }

                let width = frame.width();
                let x = if dir == Dir::LTR {
                    cursor
                } else {
                    regions.size.x - cursor - width
                };

                output.push_frame(Point::with_x(x), frame);
                cursor += width + self.column_gutter;
            }

            collected.push(output);
        }

        Ok(Fragment::frames(collected))
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
            let frames = layout_fragment(
                self.engine,
                &FootnoteEntry::new(notes[k].clone()).pack(),
                Locator::synthesize(notes[k].location().unwrap()),
                self.shared,
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
                self.collect_footnotes(notes, &frame);
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
        let pod = Region::new(self.regions.base(), expand);
        let separator = &self.footnote_config.separator;

        // FIXME: Shouldn't use `root()` here.
        let mut frame =
            layout_frame(self.engine, separator, Locator::root(), self.shared, pod)?;
        frame.size_mut().y += self.footnote_config.clearance;
        frame.translate(Point::with_y(self.footnote_config.clearance));

        self.has_footnotes = true;
        self.regions.size.y -= frame.height();
        self.items.push(FlowItem::Footnote(frame));

        Ok(())
    }

    /// Layout the given collected lines' line numbers to an output frame.
    ///
    /// The numbers are placed either on the left margin (left border of the
    /// frame) or on the right margin (right border). Before they are placed,
    /// a line number counter reset is inserted if we're in the first column of
    /// the page being currently laid out and the user requested for line
    /// numbers to be reset at the start of every page.
    fn layout_line_numbers(
        &mut self,
        output: &mut Frame,
        size: Size,
        mut lines: Vec<CollectedParLine>,
    ) -> SourceResult<()> {
        // Reset page-scoped line numbers if currently at the first column.
        if self.root
            && (self.columns == 1 || self.finished.len() % self.columns == 0)
            && ParLine::numbering_scope_in(self.shared) == ParLineNumberingScope::Page
        {
            let reset =
                CounterState::init(&CounterKey::Selector(ParLineMarker::elem().select()));
            let counter = Counter::of(ParLineMarker::elem());
            let update = counter.update(Span::detached(), CounterUpdate::Set(reset));
            let locator = self.locator.next(&update);
            let pod = Region::new(Axes::splat(Abs::zero()), Axes::splat(false));
            let reset_frame =
                layout_frame(self.engine, &update, locator, self.shared, pod)?;
            output.push_frame(Point::zero(), reset_frame);
        }

        if lines.is_empty() {
            // We always stop here if this is not the root flow.
            return Ok(());
        }

        // Assume the line numbers aren't sorted by height.
        // They must be sorted so we can deduplicate line numbers below based
        // on vertical proximity.
        lines.sort_by_key(|line| line.y);

        // Buffer line number frames so we can align them horizontally later
        // before placing, based on the width of the largest line number.
        let mut line_numbers = vec![];
        // Used for horizontal alignment.
        let mut max_number_width = Abs::zero();
        let mut prev_bottom = None;
        for line in lines {
            if prev_bottom.is_some_and(|prev_bottom| line.y < prev_bottom) {
                // Lines are too close together. Display as the same line
                // number.
                continue;
            }

            let current_column = self.finished.len() % self.columns;
            let number_margin = if self.columns >= 2 && current_column + 1 == self.columns
            {
                // The last column will always place line numbers at the end
                // margin. This should become configurable in the future.
                OuterHAlignment::End.resolve(self.shared)
            } else {
                line.marker.number_margin().resolve(self.shared)
            };

            let number_align = line
                .marker
                .number_align()
                .map(|align| align.resolve(self.shared))
                .unwrap_or_else(|| number_margin.inv());

            let number_clearance = line.marker.number_clearance().resolve(self.shared);
            let number = self.layout_line_number(line.marker)?;
            let number_x = match number_margin {
                FixedAlignment::Start => -number_clearance,
                FixedAlignment::End => size.x + number_clearance,

                // Shouldn't be specifiable by the user due to
                // 'OuterHAlignment'.
                FixedAlignment::Center => unreachable!(),
            };
            let number_pos = Point::new(number_x, line.y);

            // Note that this line.y is larger than the previous due to
            // sorting. Therefore, the check at the top of the loop ensures no
            // line numbers will reasonably intersect with each other.
            //
            // We enforce a minimum spacing of 1pt between consecutive line
            // numbers in case a zero-height frame is used.
            prev_bottom = Some(line.y + number.height().max(Abs::pt(1.0)));

            // Collect line numbers and compute the max width so we can align
            // them later.
            max_number_width.set_max(number.width());
            line_numbers.push((number_pos, number, number_align, number_margin));
        }

        for (mut pos, number, align, margin) in line_numbers {
            if matches!(margin, FixedAlignment::Start) {
                // Move the line number backwards the more aligned to the left
                // it is, instead of moving to the right when it's right
                // aligned. We do it this way, without fully overriding the
                // 'x' coordinate, to preserve the original clearance between
                // the line numbers and the text.
                pos.x -=
                    max_number_width - align.position(max_number_width - number.width());
            } else {
                // Move the line number forwards when aligned to the right.
                // Leave as is when aligned to the left.
                pos.x += align.position(max_number_width - number.width());
            }

            output.push_frame(pos, number);
        }

        Ok(())
    }

    /// Layout the line number associated with the given line marker.
    ///
    /// Produces a counter update and counter display with counter key
    /// `ParLineMarker`. We use `ParLineMarker` as it is an element which is
    /// not exposed to the user, as we don't want to expose the line number
    /// counter at the moment, given that its semantics are inconsistent with
    /// that of normal counters (the counter is updated based on height and not
    /// on frame order / layer). When we find a solution to this, we should
    /// switch to a counter on `ParLine` instead, thus exposing the counter as
    /// `counter(par.line)` to the user.
    fn layout_line_number(
        &mut self,
        marker: Packed<ParLineMarker>,
    ) -> SourceResult<Frame> {
        let counter = Counter::of(ParLineMarker::elem());
        let counter_update = counter
            .clone()
            .update(Span::detached(), CounterUpdate::Step(NonZeroUsize::ONE));
        let counter_display = CounterDisplayElem::new(
            counter,
            Smart::Custom(marker.numbering().clone()),
            false,
        );
        let number = SequenceElem::new(vec![counter_update, counter_display.pack()]);
        let locator = self.locator.next(&number);

        let pod = Region::new(Axes::splat(Abs::inf()), Axes::splat(false));
        let mut frame =
            layout_frame(self.engine, &number.pack(), locator, self.shared, pod)?;

        // Ensure the baseline of the line number aligns with the line's own
        // baseline.
        frame.translate(Point::with_y(-frame.baseline()));

        Ok(frame)
    }

    /// Collect all footnotes in a frame.
    fn collect_footnotes(
        &mut self,
        notes: &mut Vec<Packed<FootnoteElem>>,
        frame: &Frame,
    ) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => self.collect_footnotes(notes, &group.frame),
                FrameItem::Tag(tag) => {
                    let Some(footnote) = tag.elem().to_packed::<FootnoteElem>() else {
                        continue;
                    };
                    if self.visited_footnotes.insert(tag.location()) {
                        notes.push(footnote.clone());
                    }
                }
                _ => {}
            }
        }
    }
}

/// Collect all numbered paragraph lines in the frame.
/// The 'prev_y' parameter starts at 0 on the first call to 'collect_par_lines'.
/// On each subframe we encounter, we add that subframe's position to 'prev_y',
/// until we reach a line's tag, at which point we add the tag's position
/// and finish. That gives us the relative height of the line from the start of
/// the initial frame.
fn collect_par_lines(
    lines: &mut Vec<CollectedParLine>,
    frame: &Frame,
    frame_pos: Point,
    prev_y: Abs,
) {
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Group(group) => {
                collect_par_lines(lines, &group.frame, frame_pos, prev_y + pos.y)
            }

            // Unlike footnotes, we don't need to guard against duplicate tags
            // here, since we already deduplicate line markers based on their
            // height later on, in `finish_region`.
            FrameItem::Tag(tag) => {
                let Some(marker) = tag.elem().to_packed::<ParLineMarker>() else {
                    continue;
                };

                // 1. 'prev_y' is the accumulated relative height from the top
                // of the frame we're searching so far;
                // 2. 'prev_y + pos.y' gives us the final relative height of
                // the line we just found from the top of the initial frame;
                // 3. 'frame_pos.y' is the height of the initial frame relative
                // to the root flow (and thus its absolute 'y');
                // 4. Therefore, 'y' will be the line's absolute 'y' in the
                // page based on its marker's position, and thus the 'y' we
                // should use for line numbers. In particular, this represents
                // the 'y' at the line's general baseline, due to the marker
                // placement logic within the 'line::commit()' function in the
                // 'inline' module. We only account for the line number's own
                // baseline later, upon layout.
                let y = frame_pos.y + prev_y + pos.y;

                lines.push(CollectedParLine { y, marker: marker.clone() });
            }
            _ => {}
        }
    }
}
