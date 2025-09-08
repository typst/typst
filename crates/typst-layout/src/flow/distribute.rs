use typst_library::introspection::Tag;
use typst_library::layout::{
    Abs, Axes, FixedAlignment, Fr, Frame, FrameItem, Point, Region, Regions, Rel, Size,
};
use typst_utils::Numeric;

use super::{
    Child, Composer, FlowResult, LineChild, MultiChild, MultiSpill, PlacedChild,
    SingleChild, Stop, Work,
};

/// Distributes as many children as fit from `composer.work` into the first
/// region and returns the resulting frame.
pub fn distribute(composer: &mut Composer, regions: Regions) -> FlowResult<Frame> {
    let mut distributor = Distributor {
        composer,
        regions,
        items: vec![],
        sticky: None,
        stickable: None,
    };
    let init = distributor.snapshot();
    let forced = match distributor.run() {
        Ok(()) => distributor.composer.work.done(),
        Err(Stop::Finish(forced)) => forced,
        Err(err) => return Err(err),
    };
    let region = Region::new(regions.size, regions.expand);
    distributor.finalize(region, init, forced)
}

/// State for distribution.
///
/// See [Composer] regarding lifetimes.
struct Distributor<'a, 'b, 'x, 'y, 'z> {
    /// The composer that is used to handle insertions.
    composer: &'z mut Composer<'a, 'b, 'x, 'y>,
    /// Regions which are continuously shrunk as new items are added.
    regions: Regions<'z>,
    /// Already laid out items, not yet aligned.
    items: Vec<Item<'a, 'b>>,
    /// A snapshot which can be restored to migrate a suffix of sticky blocks to
    /// the next region.
    sticky: Option<DistributionSnapshot<'a, 'b>>,
    /// Whether the current group of consecutive sticky blocks are still sticky
    /// and may migrate with the attached frame. This is `None` while we aren't
    /// processing sticky blocks. On the first sticky block, this will become
    /// `Some(true)` if migrating sticky blocks as usual would make a
    /// difference - this is given by `regions.may_progress()`. Otherwise, it
    /// is set to `Some(false)`, which is usually the case when the first
    /// sticky block in the group is at the very top of the page (then,
    /// migrating it would just lead us back to the top of the page, leading
    /// to an infinite loop). In that case, all sticky blocks of the group are
    /// also disabled, until this is reset to `None` on the first non-sticky
    /// frame we find.
    ///
    /// While this behavior of disabling stickiness of sticky blocks at the
    /// very top of the page may seem non-ideal, it is only problematic (that
    /// is, may lead to orphaned sticky blocks / headings) if the combination
    /// of 'sticky blocks + attached frame' doesn't fit in one page, in which
    /// case there is nothing Typst can do to improve the situation, as sticky
    /// blocks are supposed to always be in the same page as the subsequent
    /// frame, but that is impossible in that case, which is thus pathological.
    stickable: Option<bool>,
}

/// A snapshot of the distribution state.
struct DistributionSnapshot<'a, 'b> {
    work: Work<'a, 'b>,
    items: usize,
}

/// A laid out item in a distribution.
enum Item<'a, 'b> {
    /// An introspection tag.
    Tag(&'a Tag),
    /// Absolute spacing and its weakness level.
    Abs(Abs, u8),
    /// Fractional spacing or a fractional block.
    Fr(Fr, Option<&'b SingleChild<'a>>),
    /// A frame for a laid out line or block.
    Frame(Frame, Axes<FixedAlignment>),
    /// A frame for an absolutely (not floatingly) placed child.
    Placed(Frame, &'b PlacedChild<'a>),
}

impl Item<'_, '_> {
    /// Whether this item should be migrated to the next region if the region
    /// consists solely of such items.
    fn migratable(&self) -> bool {
        match self {
            Self::Tag(_) => true,
            Self::Frame(frame, _) => {
                frame.size().is_zero()
                    && frame.items().all(|(_, item)| {
                        matches!(item, FrameItem::Link(_, _) | FrameItem::Tag(_))
                    })
            }
            Self::Placed(_, placed) => !placed.float,
            _ => false,
        }
    }
}

impl<'a, 'b> Distributor<'a, 'b, '_, '_, '_> {
    /// Distributes content into the region.
    fn run(&mut self) -> FlowResult<()> {
        // First, handle spill of a breakable block.
        if let Some(spill) = self.composer.work.spill.take() {
            self.multi_spill(spill)?;
        }

        // If spill are taken care of, process children until no space is left
        // or no children are left.
        while let Some(child) = self.composer.work.head() {
            self.child(child)?;
            self.composer.work.advance();
        }

        Ok(())
    }

    /// Processes a single child.
    ///
    /// - Returns `Ok(())` if the child was successfully processed.
    /// - Returns `Err(Stop::Finish)` if a region break should be triggered.
    /// - Returns `Err(Stop::Relayout(_))` if the region needs to be relayouted
    ///   due to an insertion (float/footnote).
    /// - Returns `Err(Stop::Error(_))` if there was a fatal error.
    fn child(&mut self, child: &'b Child<'a>) -> FlowResult<()> {
        match child {
            Child::Tag(tag) => self.tag(tag),
            Child::Rel(amount, weakness) => self.rel(*amount, *weakness),
            Child::Fr(fr) => self.fr(*fr),
            Child::Line(line) => self.line(line)?,
            Child::Single(single) => self.single(single)?,
            Child::Multi(multi) => self.multi(multi)?,
            Child::Placed(placed) => self.placed(placed)?,
            Child::Flush => self.flush()?,
            Child::Break(weak) => self.break_(*weak)?,
        }
        Ok(())
    }

    /// Processes a tag.
    fn tag(&mut self, tag: &'a Tag) {
        self.composer.work.tags.push(tag);
    }

    /// Generate items for pending tags.
    fn flush_tags(&mut self) {
        if !self.composer.work.tags.is_empty() {
            let tags = &mut self.composer.work.tags;
            self.items.extend(tags.iter().copied().map(Item::Tag));
            tags.clear();
        }
    }

    /// Processes relative spacing.
    fn rel(&mut self, amount: Rel<Abs>, weakness: u8) {
        let amount = amount.relative_to(self.regions.base().y);
        if weakness > 0 && !self.keep_spacing(amount, weakness) {
            return;
        }

        self.regions.size.y -= amount;
        self.items.push(Item::Abs(amount, weakness));
    }

    /// Processes fractional spacing.
    fn fr(&mut self, fr: Fr) {
        self.trim_spacing();
        self.items.push(Item::Fr(fr, None));
    }

    /// Decides whether to keep weak spacing based on previous items. If there
    /// is a preceding weak spacing, it might be patched in place.
    fn keep_spacing(&mut self, amount: Abs, weakness: u8) -> bool {
        for item in self.items.iter_mut().rev() {
            match *item {
                Item::Abs(prev_amount, prev_weakness @ 1..) => {
                    if weakness <= prev_weakness
                        && (weakness < prev_weakness || amount > prev_amount)
                    {
                        self.regions.size.y -= amount - prev_amount;
                        *item = Item::Abs(amount, weakness);
                    }
                    return false;
                }
                Item::Tag(_) | Item::Abs(..) | Item::Placed(..) => {}
                Item::Fr(.., None) => return false,
                Item::Frame(..) | Item::Fr(.., Some(_)) => return true,
            }
        }
        false
    }

    /// Trims trailing weak spacing from the items.
    fn trim_spacing(&mut self) {
        for (i, item) in self.items.iter().enumerate().rev() {
            match *item {
                Item::Abs(amount, 1..) => {
                    self.regions.size.y += amount;
                    self.items.remove(i);
                    break;
                }
                Item::Tag(_) | Item::Abs(..) | Item::Placed(..) => {}
                Item::Frame(..) | Item::Fr(..) => break,
            }
        }
    }

    /// The amount of trailing weak spacing.
    fn weak_spacing(&mut self) -> Abs {
        for item in self.items.iter().rev() {
            match *item {
                Item::Abs(amount, 1..) => return amount,
                Item::Tag(_) | Item::Abs(..) | Item::Placed(..) => {}
                Item::Frame(..) | Item::Fr(..) => break,
            }
        }
        Abs::zero()
    }

    /// Processes a line of a paragraph.
    fn line(&mut self, line: &'b LineChild) -> FlowResult<()> {
        // If the line doesn't fit and a followup region may improve things,
        // finish the region.
        if !self.regions.size.y.fits(line.frame.height()) && self.regions.may_progress() {
            return Err(Stop::Finish(false));
        }

        // If the line's need, which includes its own height and that of
        // following lines grouped by widow/orphan prevention, does not fit into
        // the current region, but does fit into the next region, finish the
        // region.
        if !self.regions.size.y.fits(line.need)
            && self
                .regions
                .iter()
                .nth(1)
                .is_some_and(|region| region.y.fits(line.need))
        {
            return Err(Stop::Finish(false));
        }

        self.frame(line.frame.clone(), line.align, false, false)
    }

    /// Processes an unbreakable block.
    fn single(&mut self, single: &'b SingleChild<'a>) -> FlowResult<()> {
        // Lay out the block.
        let frame = single.layout(
            self.composer.engine,
            Region::new(self.regions.base(), self.regions.expand),
        )?;

        // Handle fractionally sized blocks.
        if let Some(fr) = single.fr {
            self.composer
                .footnotes(&self.regions, &frame, Abs::zero(), false, true)?;
            self.flush_tags();
            self.items.push(Item::Fr(fr, Some(single)));
            return Ok(());
        }

        // If the block doesn't fit and a followup region may improve things,
        // finish the region.
        if !self.regions.size.y.fits(frame.height()) && self.regions.may_progress() {
            return Err(Stop::Finish(false));
        }

        self.frame(frame, single.align, single.sticky, false)
    }

    /// Processes a breakable block.
    fn multi(&mut self, multi: &'b MultiChild<'a>) -> FlowResult<()> {
        // Skip directly if the region is already (over)full. `line` and
        // `single` implicitly do this through their `fits` checks.
        if self.regions.is_full() {
            return Err(Stop::Finish(false));
        }

        // Lay out the block.
        let (frame, spill) = multi.layout(self.composer.engine, self.regions)?;
        if frame.is_empty()
            && spill.as_ref().is_some_and(|s| s.exist_non_empty_frame)
            && self.regions.may_progress()
        {
            // If the first frame is empty, but there are non-empty frames in
            // the spill, the whole child should be put in the next region to
            // avoid any invisible orphans at the end of this region.
            return Err(Stop::Finish(false));
        }

        self.frame(frame, multi.align, multi.sticky, true)?;

        // If the block didn't fully fit into the current region, save it into
        // the `spill` and finish the region.
        if let Some(spill) = spill {
            self.composer.work.spill = Some(spill);
            self.composer.work.advance();
            return Err(Stop::Finish(false));
        }

        Ok(())
    }

    /// Processes spillover from a breakable block.
    fn multi_spill(&mut self, spill: MultiSpill<'a, 'b>) -> FlowResult<()> {
        // Skip directly if the region is already (over)full.
        if self.regions.is_full() {
            self.composer.work.spill = Some(spill);
            return Err(Stop::Finish(false));
        }

        // Lay out the spilled remains.
        let align = spill.align();
        let (frame, spill) = spill.layout(self.composer.engine, self.regions)?;
        self.frame(frame, align, false, true)?;

        // If there's still more, save it into the `spill` and finish the
        // region.
        if let Some(spill) = spill {
            self.composer.work.spill = Some(spill);
            return Err(Stop::Finish(false));
        }

        Ok(())
    }

    /// Processes an in-flow frame, generated from a line or block.
    fn frame(
        &mut self,
        frame: Frame,
        align: Axes<FixedAlignment>,
        sticky: bool,
        breakable: bool,
    ) -> FlowResult<()> {
        if sticky {
            // If the frame is sticky and we haven't remembered a preceding
            // sticky element, make a checkpoint which we can restore should we
            // end on this sticky element.
            //
            // The first sticky block within consecutive sticky blocks
            // determines whether this group of sticky blocks has stickiness
            // disabled or not.
            //
            // The criteria used here is: if migrating this group of sticky
            // blocks together with the "attached" block can't improve the lack
            // of space, since we're at the start of the region, then we don't
            // do so, and stickiness is disabled (at least, for this region).
            // Otherwise, migration is allowed.
            //
            // Note that, since the whole region is checked, this ensures sticky
            // blocks at the top of a block - but not necessarily of the page -
            // can still be migrated.
            if self.sticky.is_none()
                && *self.stickable.get_or_insert_with(|| self.regions.may_progress())
            {
                self.sticky = Some(self.snapshot());
            }
        } else if !frame.is_empty() {
            // If the frame isn't sticky, we can forget a previous snapshot. We
            // interrupt a group of sticky blocks, if there was one, so we reset
            // the saved stickable check for the next group of sticky blocks.
            self.sticky = None;
            self.stickable = None;
        }

        // Handle footnotes.
        self.composer.footnotes(
            &self.regions,
            &frame,
            frame.height(),
            breakable,
            true,
        )?;

        // Push an item for the frame.
        self.regions.size.y -= frame.height();
        self.flush_tags();
        self.items.push(Item::Frame(frame, align));
        Ok(())
    }

    /// Processes an absolutely or floatingly placed child.
    fn placed(&mut self, placed: &'b PlacedChild<'a>) -> FlowResult<()> {
        if placed.float {
            // If the element is floatingly placed, let the composer handle it.
            // It might require relayout because the area available for
            // distribution shrinks. We make the spacing occupied by weak
            // spacing temporarily available again because it can collapse if it
            // ends up at a break due to the float.
            let weak_spacing = self.weak_spacing();
            self.regions.size.y += weak_spacing;
            self.composer.float(
                placed,
                &self.regions,
                self.items.iter().any(|item| matches!(item, Item::Frame(..))),
                true,
            )?;
            self.regions.size.y -= weak_spacing;
        } else {
            let frame = placed.layout(self.composer.engine, self.regions.base())?;
            self.composer
                .footnotes(&self.regions, &frame, Abs::zero(), true, true)?;
            self.flush_tags();
            self.items.push(Item::Placed(frame, placed));
        }
        Ok(())
    }

    /// Processes a float flush.
    fn flush(&mut self) -> FlowResult<()> {
        // If there are still pending floats, finish the region instead of
        // adding more content to it.
        if !self.composer.work.floats.is_empty() {
            return Err(Stop::Finish(false));
        }
        Ok(())
    }

    /// Processes a column break.
    fn break_(&mut self, weak: bool) -> FlowResult<()> {
        // If there is a region to break into, break into it.
        if (!weak || !self.items.is_empty())
            && (!self.regions.backlog.is_empty() || self.regions.last.is_some())
        {
            self.composer.work.advance();
            return Err(Stop::Finish(true));
        }
        Ok(())
    }

    /// Arranges the produced items into an output frame.
    ///
    /// This performs alignment and resolves fractional spacing and blocks.
    fn finalize(
        mut self,
        region: Region,
        init: DistributionSnapshot<'a, 'b>,
        forced: bool,
    ) -> FlowResult<Frame> {
        if forced {
            // If this is the very end of the flow, flush pending tags.
            self.flush_tags();
        } else if !self.items.is_empty() && self.items.iter().all(Item::migratable) {
            // Restore the initial state of all items are migratable.
            self.restore(init);
        } else {
            // If we ended on a sticky block, but are not yet at the end of
            // the flow, restore the saved checkpoint to move the sticky
            // suffix to the next region.
            if let Some(snapshot) = self.sticky.take() {
                self.restore(snapshot)
            }
        }

        self.trim_spacing();

        let mut frs = Fr::zero();
        let mut used = Size::zero();
        let mut has_fr_child = false;

        // Determine the amount of used space and the sum of fractionals.
        for item in &self.items {
            match item {
                Item::Abs(v, _) => used.y += *v,
                Item::Fr(v, child) => {
                    frs += *v;
                    has_fr_child |= child.is_some();
                }
                Item::Frame(frame, _) => {
                    used.y += frame.height();
                    used.x.set_max(frame.width());
                }
                Item::Tag(_) | Item::Placed(..) => {}
            }
        }

        // When we have fractional spacing, occupy the remaining space with it.
        let mut fr_space = Abs::zero();
        if frs.get() > 0.0 && region.size.y.is_finite() {
            fr_space = region.size.y - used.y;
            used.y = region.size.y;
        }

        // Lay out fractionally sized blocks.
        let mut fr_frames = vec![];
        if has_fr_child {
            for item in &self.items {
                let Item::Fr(v, Some(single)) = item else { continue };
                let length = v.share(frs, fr_space);
                let pod = Region::new(Size::new(region.size.x, length), region.expand);
                let frame = single.layout(self.composer.engine, pod)?;
                used.x.set_max(frame.width());
                fr_frames.push(frame);
            }
        }

        // Also consider the width of insertions for alignment.
        if !region.expand.x {
            used.x.set_max(self.composer.insertion_width());
        }

        // Determine the region's size.
        let size = region.expand.select(region.size, used.min(region.size));
        let free = size.y - used.y;

        let mut output = Frame::soft(size);
        let mut ruler = FixedAlignment::Start;
        let mut offset = Abs::zero();
        let mut fr_frames = fr_frames.into_iter();

        // Position all items.
        for item in self.items {
            match item {
                Item::Tag(tag) => {
                    let y = offset + ruler.position(free);
                    let pos = Point::with_y(y);
                    output.push(pos, FrameItem::Tag(tag.clone()));
                }
                Item::Abs(v, _) => {
                    offset += v;
                }
                Item::Fr(v, single) => {
                    let length = v.share(frs, fr_space);
                    if let Some(single) = single {
                        let frame = fr_frames.next().unwrap();
                        let x = single.align.x.position(size.x - frame.width());
                        let pos = Point::new(x, offset);
                        output.push_frame(pos, frame);
                    }
                    offset += length;
                }
                Item::Frame(frame, align) => {
                    ruler = ruler.max(align.y);

                    let x = align.x.position(size.x - frame.width());
                    let y = offset + ruler.position(free);
                    let pos = Point::new(x, y);
                    offset += frame.height();

                    output.push_frame(pos, frame);
                }
                Item::Placed(frame, placed) => {
                    let x = placed.align_x.position(size.x - frame.width());
                    let y = match placed.align_y.unwrap_or_default() {
                        Some(align) => align.position(size.y - frame.height()),
                        _ => offset + ruler.position(free),
                    };

                    let pos = Point::new(x, y)
                        + placed.delta.zip_map(size, Rel::relative_to).to_point();

                    output.push_frame(pos, frame);
                }
            }
        }

        Ok(output)
    }

    /// Create a snapshot of the work and items.
    fn snapshot(&self) -> DistributionSnapshot<'a, 'b> {
        DistributionSnapshot {
            work: self.composer.work.clone(),
            items: self.items.len(),
        }
    }

    /// Restore a snapshot of the work and items.
    fn restore(&mut self, snapshot: DistributionSnapshot<'a, 'b>) {
        *self.composer.work = snapshot.work;
        self.items.truncate(snapshot.items);
    }
}
