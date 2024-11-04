use std::num::NonZeroUsize;

use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Content, NativeElement, Packed, Resolve, Smart};
use typst_library::introspection::{
    Counter, CounterDisplayElem, CounterState, CounterUpdate, Location, Locator,
    SplitLocator, Tag,
};
use typst_library::layout::{
    Abs, Axes, Dir, FixedAlignment, Fragment, Frame, FrameItem, OuterHAlignment,
    PlacementScope, Point, Region, Regions, Rel, Size,
};
use typst_library::model::{
    FootnoteElem, FootnoteEntry, LineNumberingScope, Numbering, ParLineMarker,
};
use typst_syntax::Span;
use typst_utils::NonZeroExt;

use super::{distribute, Config, FlowResult, LineNumberConfig, PlacedChild, Stop, Work};

/// Composes the contents of a single page/region. A region can have multiple
/// columns/subregions.
///
/// The composer is primarily concerned with layout of out-of-flow insertions
/// (floats and footnotes).  It does this in per-page and per-column loops that
/// rerun when a new float is added (since it affects the regions available to
/// the distributor).
///
/// To lay out the in-flow contents of individual subregions, the composer
/// invokes [distribution](distribute).
pub fn compose(
    engine: &mut Engine,
    work: &mut Work,
    config: &Config,
    locator: Locator,
    regions: Regions,
) -> SourceResult<Frame> {
    Composer {
        engine,
        config,
        page_base: regions.base(),
        column: 0,
        page_insertions: Insertions::default(),
        column_insertions: Insertions::default(),
        work,
        footnote_spill: None,
        footnote_queue: vec![],
    }
    .page(locator, regions)
}

/// State for composition.
///
/// Sadly, we need that many lifetimes because &mut references are invariant and
/// it would force the lifetimes of various things to be equal if they
/// shared a lifetime.
///
/// The only interesting lifetimes are 'a and 'b. See [Work] for more details
/// about them.
pub struct Composer<'a, 'b, 'x, 'y> {
    pub engine: &'x mut Engine<'y>,
    pub work: &'x mut Work<'a, 'b>,
    pub config: &'x Config<'x>,
    column: usize,
    page_base: Size,
    page_insertions: Insertions<'a, 'b>,
    column_insertions: Insertions<'a, 'b>,
    // These are here because they have to survive relayout (we could lose the
    // footnotes otherwise). For floats, we revisit them anyway, so it's okay to
    // use `work.floats` directly. This is not super clean; probably there's a
    // better way.
    footnote_spill: Option<std::vec::IntoIter<Frame>>,
    footnote_queue: Vec<Packed<FootnoteElem>>,
}

impl<'a, 'b> Composer<'a, 'b, '_, '_> {
    /// Lay out a container/page region, including container/page insertions.
    fn page(mut self, locator: Locator, regions: Regions) -> SourceResult<Frame> {
        // This loop can restart region layout when requested to do so by a
        // `Stop`. This happens when there is a parent-scoped float.
        let checkpoint = self.work.clone();
        let output = loop {
            // Shrink the available space by the space used by page
            // insertions.
            let mut pod = regions;
            pod.size.y -= self.page_insertions.height();

            match self.page_contents(locator.relayout(), pod) {
                Ok(frame) => break frame,
                Err(Stop::Finish(_)) => unreachable!(),
                Err(Stop::Relayout(PlacementScope::Column)) => unreachable!(),
                Err(Stop::Relayout(PlacementScope::Parent)) => {
                    *self.work = checkpoint.clone();
                    continue;
                }
                Err(Stop::Error(err)) => return Err(err),
            };
        };
        drop(checkpoint);

        Ok(self.page_insertions.finalize(self.work, self.config, output))
    }

    /// Lay out the inner contents of a container/page.
    fn page_contents(&mut self, locator: Locator, regions: Regions) -> FlowResult<Frame> {
        // No point in create column regions, if there's just one!
        if self.config.columns.count == 1 {
            return self.column(locator, regions);
        }

        // Create a backlog for multi-column layout.
        let column_height = regions.size.y;
        let backlog: Vec<_> = std::iter::once(&column_height)
            .chain(regions.backlog)
            .flat_map(|&h| std::iter::repeat(h).take(self.config.columns.count))
            .skip(1)
            .collect();

        // Subregions for column layout.
        let mut inner = Regions {
            size: Size::new(self.config.columns.width, column_height),
            backlog: &backlog,
            expand: Axes::new(true, regions.expand.y),
            ..regions
        };

        // The size of the merged frame hosting multiple columns.
        let size = Size::new(
            regions.size.x,
            if regions.expand.y { regions.size.y } else { Abs::zero() },
        );

        let mut output = Frame::hard(size);
        let mut offset = Abs::zero();
        let mut locator = locator.split();

        // Lay out the columns and stitch them together.
        for i in 0..self.config.columns.count {
            self.column = i;
            let frame = self.column(locator.next(&()), inner)?;

            if !regions.expand.y {
                output.size_mut().y.set_max(frame.height());
            }

            let width = frame.width();
            let x = if self.config.columns.dir == Dir::LTR {
                offset
            } else {
                regions.size.x - offset - width
            };
            offset += width + self.config.columns.gutter;

            output.push_frame(Point::with_x(x), frame);
            inner.next();
        }

        Ok(output)
    }

    /// Lay out a column, including column insertions.
    fn column(&mut self, locator: Locator, regions: Regions) -> FlowResult<Frame> {
        // Reset column insertion when starting a new column.
        self.column_insertions = Insertions::default();

        // Process footnote spill.
        if let Some(spill) = self.work.footnote_spill.take() {
            self.footnote_spill(spill, regions.base())?;
        }

        // This loop can restart column layout when requested to do so by a
        // `Stop`. This happens when there is a column-scoped float.
        let checkpoint = self.work.clone();
        let inner = loop {
            // Shrink the available space by the space used by column
            // insertions.
            let mut pod = regions;
            pod.size.y -= self.column_insertions.height();

            match self.column_contents(pod) {
                Ok(frame) => break frame,
                Err(Stop::Finish(_)) => unreachable!(),
                Err(Stop::Relayout(PlacementScope::Column)) => {
                    *self.work = checkpoint.clone();
                    continue;
                }
                err => return err,
            }
        };
        drop(checkpoint);

        self.work.footnotes.extend(self.footnote_queue.drain(..));
        if let Some(spill) = self.footnote_spill.take() {
            self.work.footnote_spill = Some(spill);
        }

        let insertions = std::mem::take(&mut self.column_insertions);
        let mut output = insertions.finalize(self.work, self.config, inner);

        // Lay out per-column line numbers.
        if let Some(line_config) = &self.config.line_numbers {
            layout_line_numbers(
                self.engine,
                self.config,
                line_config,
                locator,
                self.column,
                &mut output,
            )?;
        }

        Ok(output)
    }

    /// Lay out the inner contents of a column.
    fn column_contents(&mut self, regions: Regions) -> FlowResult<Frame> {
        // Process pending footnotes.
        for note in std::mem::take(&mut self.work.footnotes) {
            self.footnote(note, &mut regions.clone(), Abs::zero(), false)?;
        }

        // Process pending floats.
        for placed in std::mem::take(&mut self.work.floats) {
            self.float(placed, &regions, false)?;
        }

        distribute(self, regions)
    }

    /// Lays out an item with floating placement.
    ///
    /// This is called from within [`distribute`]. When the float fits, this
    /// returns an `Err(Stop::Relayout(..))`, which bubbles all the way through
    /// distribution and is handled in [`Self::page`] or [`Self::column`]
    /// (depending on `placed.scope`).
    ///
    /// When the float does not fit, it is queued into `work.floats`. The
    /// value of `clearance` that between the float and flow content is needed
    /// --- it is set if there are already distributed items.
    pub fn float(
        &mut self,
        placed: &'b PlacedChild<'a>,
        regions: &Regions,
        clearance: bool,
    ) -> FlowResult<()> {
        // If the float is already processed, skip it.
        let loc = placed.location();
        if self.skipped(loc) {
            return Ok(());
        }

        // If there is already a queued float, queue this one as well. We
        // don't want to disrupt the order.
        if !self.work.floats.is_empty() {
            self.work.floats.push(placed);
            return Ok(());
        }

        // Determine the base size of the chosen scope.
        let base = match placed.scope {
            PlacementScope::Column => regions.base(),
            PlacementScope::Parent => self.page_base,
        };

        // Lay out the placed element.
        let frame = placed.layout(self.engine, base)?;

        // Determine the remaining space in the scope. This is exact for column
        // placement, but only an approximation for page placement.
        let remaining = match placed.scope {
            PlacementScope::Column => regions.size.y,
            PlacementScope::Parent => {
                let remaining: Abs = regions
                    .iter()
                    .map(|size| size.y)
                    .take(self.config.columns.count - self.column)
                    .sum();
                remaining / self.config.columns.count as f64
            }
        };

        // We only require clearance if there is other content.
        let clearance = if clearance { placed.clearance } else { Abs::zero() };
        let need = frame.height() + clearance;

        // If the float doesn't fit, queue it for the next region.
        if !remaining.fits(need) && regions.may_progress() {
            self.work.floats.push(placed);
            return Ok(());
        }

        // Handle footnotes in the float.
        self.footnotes(regions, &frame, need, false)?;

        // Determine the float's vertical alignment. We can unwrap the inner
        // `Option` because `Custom(None)` is checked for during collection.
        let align_y = placed.align_y.map(Option::unwrap).unwrap_or_else(|| {
            // When the float's vertical midpoint would be above the middle of
            // the page if it were layouted in-flow, we use top alignment.
            // Otherwise, we use bottom alignment.
            let used = base.y - remaining;
            let half = need / 2.0;
            let ratio = (used + half) / base.y;
            if ratio <= 0.5 {
                FixedAlignment::Start
            } else {
                FixedAlignment::End
            }
        });

        // Select the insertion area where we'll put this float.
        let area = match placed.scope {
            PlacementScope::Column => &mut self.column_insertions,
            PlacementScope::Parent => &mut self.page_insertions,
        };

        // Put the float there.
        area.push_float(placed, frame, align_y);
        area.skips.push(loc);

        // Trigger relayout.
        Err(Stop::Relayout(placed.scope))
    }

    /// Lays out footnotes in the `frame` if this is the root flow and there are
    /// any. The value of `breakable` indicates whether the element that
    /// produced the frame is breakable. If not, the frame is treated as atomic.
    pub fn footnotes(
        &mut self,
        regions: &Regions,
        frame: &Frame,
        flow_need: Abs,
        breakable: bool,
    ) -> FlowResult<()> {
        // Footnotes are only supported at the root level.
        if !self.config.root {
            return Ok(());
        }

        // Search for footnotes.
        let mut notes = vec![];
        for tag in &self.work.tags {
            let Tag::Start(elem) = tag else { continue };
            let Some(note) = elem.to_packed::<FootnoteElem>() else { continue };
            notes.push((Abs::zero(), note.clone()));
        }
        find_in_frame_impl::<FootnoteElem>(&mut notes, frame, Abs::zero());
        if notes.is_empty() {
            return Ok(());
        }

        let mut relayout = false;
        let mut regions = *regions;
        let mut migratable = !breakable && regions.may_progress();

        for (y, elem) in notes {
            // The amount of space used by the in-flow content that contains the
            // footnote marker. For a breakable frame, it's the y position of
            // the marker. For an unbreakable frame, it's the full height.
            let flow_need = if breakable { y } else { flow_need };

            // Process the footnote.
            match self.footnote(elem, &mut regions, flow_need, migratable) {
                // The footnote was already processed or queued.
                Ok(()) => {}
                // First handle more footnotes before relayouting.
                Err(Stop::Relayout(_)) => relayout = true,
                // Either of
                // - A `Stop::Finish` indicating that the frame's origin element
                //   should migrate to uphold the footnote invariant.
                // - A fatal error.
                err => return err,
            }

            // We only migrate the origin frame if the first footnote's first
            // line didn't fit.
            migratable = false;
        }

        // If this is set, we laid out at least one footnote, so we need a
        // relayout.
        if relayout {
            return Err(Stop::Relayout(PlacementScope::Column));
        }

        Ok(())
    }

    /// Handles a single footnote.
    fn footnote(
        &mut self,
        elem: Packed<FootnoteElem>,
        regions: &mut Regions,
        flow_need: Abs,
        migratable: bool,
    ) -> FlowResult<()> {
        // Ignore reference footnotes and already processed ones.
        let loc = elem.location().unwrap();
        if elem.is_ref() || self.skipped(loc) {
            return Ok(());
        }

        // If there is already a queued spill or footnote, queue this one as
        // well. We don't want to disrupt the order.
        let area = &mut self.column_insertions;
        if self.footnote_spill.is_some() || !self.footnote_queue.is_empty() {
            self.footnote_queue.push(elem);
            return Ok(());
        }

        // If there weren't any footnotes so far, account for the footnote
        // separator.
        let mut separator = None;
        let mut separator_need = Abs::zero();
        if area.footnotes.is_empty() {
            let frame =
                layout_footnote_separator(self.engine, self.config, regions.base())?;
            separator_need += self.config.footnote.clearance + frame.height();
            separator = Some(frame);
        }

        // Prepare regions for the footnote.
        let mut pod = *regions;
        pod.expand.y = false;
        pod.size.y -= flow_need + separator_need + self.config.footnote.gap;

        // Layout the footnote entry.
        let frames = layout_footnote(self.engine, self.config, &elem, pod)?.into_frames();

        // Find nested footnotes in the entry.
        let nested = find_in_frames::<FootnoteElem>(&frames);

        // Check if there are any non-empty frames.
        let mut exist_non_empty_frame = false;
        for i in &frames {
            if !i.is_empty() {
                exist_non_empty_frame = true;
                break;
            }
        }

        // Extract the first frame.
        let mut iter = frames.into_iter();
        let first = iter.next().unwrap();
        let note_need = self.config.footnote.gap + first.height();

        // If the first frame is empty, then none of its content fit. If
        // possible, we then migrate the origin frame to the next region to
        // uphold the footnote invariant (that marker and entry are on the same
        // page). If not, we just queue the footnote for the next page.
        if first.is_empty() && exist_non_empty_frame {
            if migratable {
                return Err(Stop::Finish(false));
            } else {
                self.footnote_queue.push(elem);
                return Ok(());
            }
        }

        // Save the separator.
        if let Some(frame) = separator {
            area.push_footnote_separator(self.config, frame);
            regions.size.y -= separator_need;
        }

        // Save the footnote's frame.
        area.push_footnote(self.config, first);
        area.skips.push(loc);
        regions.size.y -= note_need;

        // Save the spill.
        if !iter.as_slice().is_empty() {
            self.footnote_spill = Some(iter);
        }

        // Lay out nested footnotes.
        for (_, note) in nested {
            self.footnote(note, regions, flow_need, migratable)?;
        }

        // Since we laid out a footnote, we need a relayout.
        Err(Stop::Relayout(PlacementScope::Column))
    }

    /// Handles spillover from a footnote.
    fn footnote_spill(
        &mut self,
        mut iter: std::vec::IntoIter<Frame>,
        base: Size,
    ) -> SourceResult<()> {
        let area = &mut self.column_insertions;

        // Create and save the separator.
        let separator = layout_footnote_separator(self.engine, self.config, base)?;
        area.push_footnote_separator(self.config, separator);

        // Save the footnote's frame.
        let frame = iter.next().unwrap();
        area.push_footnote(self.config, frame);

        // Save the spill.
        if !iter.as_slice().is_empty() {
            self.footnote_spill = Some(iter);
        }

        Ok(())
    }

    /// Checks whether an insertion was already processed and doesn't need to be
    /// handled again.
    fn skipped(&self, loc: Location) -> bool {
        self.work.skips.contains(&loc)
            || self.page_insertions.skips.contains(&loc)
            || self.column_insertions.skips.contains(&loc)
    }

    /// The amount of width needed by insertions.
    pub fn insertion_width(&self) -> Abs {
        self.column_insertions.width.max(self.page_insertions.width)
    }
}

/// Lay out the footnote separator, typically a line.
fn layout_footnote_separator(
    engine: &mut Engine,
    config: &Config,
    base: Size,
) -> SourceResult<Frame> {
    crate::layout_frame(
        engine,
        &config.footnote.separator,
        Locator::root(),
        config.shared,
        Region::new(base, Axes::new(config.footnote.expand, false)),
    )
}

/// Lay out a footnote.
fn layout_footnote(
    engine: &mut Engine,
    config: &Config,
    elem: &Packed<FootnoteElem>,
    pod: Regions,
) -> SourceResult<Fragment> {
    let loc = elem.location().unwrap();
    crate::layout_fragment(
        engine,
        &FootnoteEntry::new(elem.clone()).pack(),
        Locator::synthesize(loc),
        config.shared,
        pod,
    )
    .map(|mut fragment| {
        for frame in &mut fragment {
            frame.set_parent(loc);
        }
        fragment
    })
}

/// An additive list of insertions.
#[derive(Default)]
struct Insertions<'a, 'b> {
    top_floats: Vec<(&'b PlacedChild<'a>, Frame)>,
    bottom_floats: Vec<(&'b PlacedChild<'a>, Frame)>,
    footnotes: Vec<Frame>,
    footnote_separator: Option<Frame>,
    top_size: Abs,
    bottom_size: Abs,
    width: Abs,
    skips: Vec<Location>,
}

impl<'a, 'b> Insertions<'a, 'b> {
    /// Add a float to the top or bottom area.
    fn push_float(
        &mut self,
        placed: &'b PlacedChild<'a>,
        frame: Frame,
        align_y: FixedAlignment,
    ) {
        self.width.set_max(frame.width());

        let amount = frame.height() + placed.clearance;
        let pair = (placed, frame);

        if align_y == FixedAlignment::Start {
            self.top_size += amount;
            self.top_floats.push(pair);
        } else {
            self.bottom_size += amount;
            self.bottom_floats.push(pair);
        }
    }

    /// Add a footnote to the bottom area.
    fn push_footnote(&mut self, config: &Config, frame: Frame) {
        self.width.set_max(frame.width());
        self.bottom_size += config.footnote.gap + frame.height();
        self.footnotes.push(frame);
    }

    /// Add a footnote separator to the bottom area.
    fn push_footnote_separator(&mut self, config: &Config, frame: Frame) {
        self.width.set_max(frame.width());
        self.bottom_size += config.footnote.clearance + frame.height();
        self.footnote_separator = Some(frame);
    }

    /// The combined height of the top and bottom area (includings clearances).
    /// Subtracting this from the total region size yields the available space
    /// for distribution.
    fn height(&self) -> Abs {
        self.top_size + self.bottom_size
    }

    /// Produce a frame for the full region based on the `inner` frame produced
    /// by distribution or column layout.
    fn finalize(self, work: &mut Work, config: &Config, inner: Frame) -> Frame {
        work.extend_skips(&self.skips);

        if self.top_floats.is_empty()
            && self.bottom_floats.is_empty()
            && self.footnote_separator.is_none()
            && self.footnotes.is_empty()
        {
            return inner;
        }

        let size = inner.size() + Size::with_y(self.height());

        let mut output = Frame::soft(size);
        let mut offset_top = Abs::zero();
        let mut offset_bottom = size.y - self.bottom_size;

        for (placed, frame) in self.top_floats {
            let x = placed.align_x.position(size.x - frame.width());
            let y = offset_top;
            let delta = placed.delta.zip_map(size, Rel::relative_to).to_point();
            offset_top += frame.height() + placed.clearance;
            output.push_frame(Point::new(x, y) + delta, frame);
        }

        output.push_frame(Point::with_y(self.top_size), inner);

        // We put floats first and then footnotes. This differs from what LaTeX
        // does and is a little inconsistent w.r.t column vs page floats (page
        // floats are below footnotes because footnotes are per column), but
        // it's what most people (including myself) seem to intuitively expect.
        // We experimented with the LaTeX ordering in 0.12.0-rc1, but folks were
        // surprised and considered this strange. In LaTeX, it can be changed
        // with `\usepackage[bottom]{footmisc}`. We could also consider adding
        // configuration in the future.
        for (placed, frame) in self.bottom_floats {
            offset_bottom += placed.clearance;
            let x = placed.align_x.position(size.x - frame.width());
            let y = offset_bottom;
            let delta = placed.delta.zip_map(size, Rel::relative_to).to_point();
            offset_bottom += frame.height();
            output.push_frame(Point::new(x, y) + delta, frame);
        }

        if let Some(frame) = self.footnote_separator {
            offset_bottom += config.footnote.clearance;
            let y = offset_bottom;
            offset_bottom += frame.height();
            output.push_frame(Point::with_y(y), frame);
        }

        for frame in self.footnotes {
            offset_bottom += config.footnote.gap;
            let y = offset_bottom;
            offset_bottom += frame.height();
            output.push_frame(Point::with_y(y), frame);
        }

        output
    }
}

/// Lay out the given collected lines' line numbers to an output frame.
///
/// The numbers are placed either on the left margin (left border of the frame)
/// or on the right margin (right border). Before they are placed, a line number
/// counter reset is inserted if we're in the first column of the page being
/// currently laid out and the user requested for line numbers to be reset at
/// the start of every page.
fn layout_line_numbers(
    engine: &mut Engine,
    config: &Config,
    line_config: &LineNumberConfig,
    locator: Locator,
    column: usize,
    output: &mut Frame,
) -> SourceResult<()> {
    let mut locator = locator.split();

    // Reset page-scoped line numbers if currently at the first column.
    if column == 0 && line_config.scope == LineNumberingScope::Page {
        let reset = layout_line_number_reset(engine, config, &mut locator)?;
        output.push_frame(Point::zero(), reset);
    }

    // Find all line markers.
    let mut lines = find_in_frame::<ParLineMarker>(output);
    if lines.is_empty() {
        return Ok(());
    }

    // Assume the line numbers aren't sorted by height. They must be sorted so
    // we can deduplicate line numbers below based on vertical proximity.
    lines.sort_by_key(|&(y, _)| y);

    // Used for horizontal alignment.
    let mut max_number_width = Abs::zero();

    // This is used to skip lines that are too close together.
    let mut prev_bottom = None;

    // Buffer line number frames so we can align them horizontally later before
    // placing, based on the width of the largest line number.
    let mut line_numbers = vec![];

    // Layout the lines.
    for &(y, ref marker) in &lines {
        if prev_bottom.is_some_and(|bottom| y < bottom) {
            // Lines are too close together. Display as the same line number.
            continue;
        }

        // Layout the number and record its width in search of the maximium.
        let frame = layout_line_number(engine, config, &mut locator, &marker.numbering)?;

        // Note that this line.y is larger than the previous due to sorting.
        // Therefore, the check at the top of the loop ensures no line numbers
        // will reasonably intersect with each other. We enforce a minimum
        // spacing of 1pt between consecutive line numbers in case a zero-height
        // frame is used.
        prev_bottom = Some(y + frame.height().max(Abs::pt(1.0)));
        max_number_width.set_max(frame.width());
        line_numbers.push((y, marker, frame));
    }

    for (y, marker, frame) in line_numbers {
        // The last column will always place line numbers at the end
        // margin. This should become configurable in the future.
        let margin = {
            let opposite =
                config.columns.count >= 2 && column + 1 == config.columns.count;
            if opposite { OuterHAlignment::End } else { marker.number_margin }
                .resolve(config.shared)
        };

        // Determine how much space to leave between the column and the number.
        let clearance = match marker.number_clearance {
            Smart::Auto => line_config.default_clearance,
            Smart::Custom(rel) => rel.resolve(config.shared),
        };

        // Compute the base X position.
        let x = match margin {
            // Move the number to the left of the left edge (at 0pt) by the maximum
            // width and the clearance.
            FixedAlignment::Start => -max_number_width - clearance,
            // Move the number to the right edge and add clearance.
            FixedAlignment::End => output.width() + clearance,
            // Can't happen due to `OuterHAlignment`.
            FixedAlignment::Center => unreachable!(),
        };

        // Determine how much to shift the number due to its alignment.
        let shift = {
            let align = marker
                .number_align
                .map(|align| align.resolve(config.shared))
                .unwrap_or_else(|| margin.inv());
            align.position(max_number_width - frame.width())
        };

        // Compute the final position of the number and add it to the output.
        let pos = Point::new(x + shift, y);
        output.push_frame(pos, frame);
    }

    Ok(())
}

/// Creates a frame that resets the line number counter.
fn layout_line_number_reset(
    engine: &mut Engine,
    config: &Config,
    locator: &mut SplitLocator,
) -> SourceResult<Frame> {
    let counter = Counter::of(ParLineMarker::elem());
    let update = CounterUpdate::Set(CounterState::init(false));
    let content = counter.update(Span::detached(), update);
    crate::layout_frame(
        engine,
        &content,
        locator.next(&()),
        config.shared,
        Region::new(Axes::splat(Abs::zero()), Axes::splat(false)),
    )
}

/// Layout the line number associated with the given line marker.
///
/// Produces a counter update and counter display with counter key
/// `ParLineMarker`. We use `ParLineMarker` as it is an element which is not
/// exposed to the user and we don't want to expose the line number counter at
/// the moment, given that its semantics are inconsistent with that of normal
/// counters (the counter is updated based on height and not on frame order /
/// layer). When we find a solution to this, we should switch to a counter on
/// `ParLine` instead, thus exposing the counter as `counter(par.line)` to the
/// user.
fn layout_line_number(
    engine: &mut Engine,
    config: &Config,
    locator: &mut SplitLocator,
    numbering: &Numbering,
) -> SourceResult<Frame> {
    let counter = Counter::of(ParLineMarker::elem());
    let update = CounterUpdate::Step(NonZeroUsize::ONE);
    let numbering = Smart::Custom(numbering.clone());

    // Combine counter update and display into the content we'll layout.
    let content = Content::sequence(vec![
        counter.clone().update(Span::detached(), update),
        CounterDisplayElem::new(counter, numbering, false).pack(),
    ]);

    // Layout the number.
    let mut frame = crate::layout_frame(
        engine,
        &content,
        locator.next(&()),
        config.shared,
        Region::new(Axes::splat(Abs::inf()), Axes::splat(false)),
    )?;

    // Ensure the baseline of the line number aligns with the line's baseline.
    frame.translate(Point::with_y(-frame.baseline()));

    Ok(frame)
}

/// Collect all matching elements and their vertical positions in the frame.
///
/// On each subframe we encounter, we add that subframe's position to `prev_y`,
/// until we reach a tag, at which point we add the tag's position and finish.
/// That gives us the absolute height of the tag from the start of the root
/// frame.
fn find_in_frame<T: NativeElement>(frame: &Frame) -> Vec<(Abs, Packed<T>)> {
    let mut output = vec![];
    find_in_frame_impl(&mut output, frame, Abs::zero());
    output
}

/// Collect all matching elements and their vertical positions in the frames.
fn find_in_frames<T: NativeElement>(frames: &[Frame]) -> Vec<(Abs, Packed<T>)> {
    let mut output = vec![];
    for frame in frames {
        find_in_frame_impl(&mut output, frame, Abs::zero());
    }
    output
}

fn find_in_frame_impl<T: NativeElement>(
    output: &mut Vec<(Abs, Packed<T>)>,
    frame: &Frame,
    y_offset: Abs,
) {
    for (pos, item) in frame.items() {
        let y = y_offset + pos.y;
        match item {
            FrameItem::Group(group) => find_in_frame_impl(output, &group.frame, y),
            FrameItem::Tag(Tag::Start(elem)) => {
                if let Some(elem) = elem.to_packed::<T>() {
                    output.push((y, elem.clone()));
                }
            }
            _ => {}
        }
    }
}
