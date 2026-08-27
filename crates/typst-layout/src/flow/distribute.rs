use ecow::EcoVec;
use typst_library::diag::{SourceDiagnostic, SourceResult};
use typst_library::introspection::Tag;
use typst_library::layout::{
    Abs, Axes, FixedAlignment, Fr, Frame, FrameItem, PlacementScope, Point, Region,
    Regions, Rel, Size,
};
use typst_utils::Numeric;

use super::compose::{
    Composer, FloatStop, FootnoteStop, Migration, RelayoutResult, RelayoutStop,
};
use super::{Child, LineChild, MultiChild, MultiSpill, PlacedChild, SingleChild, Work};

/// The result type for internal distributor control flow.
///
/// The `Err(_)` variant incorporate control flow events for finishing and
/// relayouting regions.
type FlowResult<T> = Result<T, Stop>;

/// A control flow event during distribution.
enum Stop {
    /// Indicates that the current subregion should be finished.
    Finish(Finish),
    /// Indicates that the given scope should be relayouted.
    Relayout(PlacementScope),
    /// Indicates that non-empty in-flow content was reached at or after the
    /// target child.
    TargetReached,
    /// A fatal error.
    Error(EcoVec<SourceDiagnostic>),
}

/// The reason why the current region should finish.
enum Finish {
    /// A lack of space.
    Soft(StopPoint),
    /// An explicit break.
    Forced,
}

/// The point at which distribution stopped, insofar as it can be identified.
enum StopPoint {
    /// The current work head is the child that stopped distribution.
    Child,
    /// The stopping point cannot be reproduced safely, e.g. partway through the
    /// spill of a breakable block.
    Unknown,
}

impl From<EcoVec<SourceDiagnostic>> for Stop {
    fn from(error: EcoVec<SourceDiagnostic>) -> Self {
        Self::Error(error)
    }
}

impl From<FootnoteStop> for Stop {
    fn from(stop: FootnoteStop) -> Self {
        match stop {
            FootnoteStop::Relayout(()) => Self::Relayout(PlacementScope::Column),
            FootnoteStop::MigrateOrigin(()) => {
                Self::Finish(Finish::Soft(StopPoint::Child))
            }
            FootnoteStop::Error(error) => Self::Error(error),
        }
    }
}

impl From<FloatStop> for Stop {
    fn from(stop: FloatStop) -> Self {
        match stop {
            FloatStop::Relayout(scope) => Self::Relayout(scope),
            FloatStop::MigrateOrigin(()) => Self::Finish(Finish::Soft(StopPoint::Child)),
            FloatStop::Error(error) => Self::Error(error),
        }
    }
}

/// A distribution target to stop after.
///
/// The target is whichever child stopped distribution, not necessarily the
/// first non-sticky child after the suffix of sticky blocks. If a sticky block
/// within the suffix overflowed, it is itself the target. Each stopping child
/// is judged on its own, so the sticky "chain" (each block staying with the
/// content right after it) is handled one stopping child at a time.
enum Target<'a, 'b> {
    /// The target child to stop after.
    Child(&'b Child<'a>),
    /// The target child was empty, so the next non-empty in-flow frame is the
    /// target.
    Following,
}

/// An optional target that stops distribution once it or a following child
/// produces non-empty in-flow content.
struct TargetState<'a, 'b>(Option<Target<'a, 'b>>);

impl<'a, 'b> TargetState<'a, 'b> {
    /// Advance the target after processing its child.
    fn advance(&mut self, child: &'b Child<'a>) {
        if let Some(Target::Child(target)) = self.0
            && std::ptr::eq(target, child)
        {
            self.0 = Some(Target::Following);
        }
    }

    /// Stop if a non-empty in-flow frame satisfies the target.
    fn stop_if_reached(
        &mut self,
        head: Option<&'b Child<'a>>,
        frame: &Frame,
        sticky: bool,
        fits: bool,
    ) -> FlowResult<()> {
        // Regardless of whether we have reached the target, we need to be at a
        // non-empty frame.
        if frame.is_empty() {
            return Ok(());
        }

        // Only proceed if there is a target and we have reached it.
        match self.0 {
            Some(Target::Child(target))
                if head.is_some_and(|head| std::ptr::eq(target, head)) => {}
            Some(Target::Following) => {}
            _ => return Ok(()),
        }

        // If the frame is sticky it is part of the suffix being migrated, so
        // it can't prove migration helped. A non-sticky target, by contrast, is
        // accepted by a final region through its normal overflow path.
        if sticky && !fits {
            return Err(Stop::Finish(Finish::Soft(StopPoint::Child)));
        }

        Err(Stop::TargetReached)
    }
}

/// Distributes as many children as fit from `composer.work` into the first
/// region and returns the resulting frame and the height actually used
/// by the inner contents (for column balancing).
pub fn distribute(
    composer: &mut Composer,
    regions: Regions,
    balancing_target: Option<Abs>,
) -> RelayoutResult<(Frame, Abs)> {
    let mut distributor = Distributor {
        composer,
        regions,
        items: vec![],
        used: Size::zero(),
        balancing_target,
        sticky: None,
        stickable: None,
        target: TargetState(None),
    };
    let init = distributor.snapshot();
    let (forced, stop) = match distributor.run() {
        Ok(()) => (distributor.composer.work.done(), StopPoint::Unknown),
        Err(Stop::Finish(Finish::Soft(stop))) => (false, stop),
        Err(Stop::Finish(Finish::Forced)) => (true, StopPoint::Unknown),
        Err(Stop::Relayout(scope)) => {
            return Err(RelayoutStop::Relayout(scope));
        }
        Err(Stop::Error(error)) => {
            return Err(RelayoutStop::Error(error));
        }
        Err(Stop::TargetReached) => unreachable!(),
    };
    let region = Region::new(regions.size, regions.expand);
    distributor
        .finalize(region, init, forced, stop)
        .map_err(RelayoutStop::Error)
}

/// Distribute until the target child or, if it is empty, a following child
/// produces non-empty in-flow content.
pub fn distribute_until<'a, 'b>(
    composer: &mut Composer<'a, 'b, '_, '_>,
    regions: Regions,
    target: &'b Child<'a>,
) -> RelayoutResult<bool> {
    let mut distributor = Distributor {
        composer,
        regions,
        items: vec![],
        used: Size::zero(),
        balancing_target: None,
        sticky: None,
        stickable: None,
        target: TargetState(Some(Target::Child(target))),
    };
    match distributor.run() {
        Err(Stop::TargetReached) => Ok(true),
        Ok(()) | Err(Stop::Finish(_)) => Ok(false),
        Err(Stop::Relayout(scope)) => Err(RelayoutStop::Relayout(scope)),
        Err(Stop::Error(error)) => Err(RelayoutStop::Error(error)),
    }
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
    /// Size used by laid out items.
    used: Size,
    /// The target height for column balancing.
    balancing_target: Option<Abs>,
    /// A snapshot which can be restored to migrate a suffix of sticky blocks to
    /// the next region.
    sticky: Option<DistributionSnapshot<'a, 'b>>,
    /// Whether the current group of consecutive sticky blocks are still sticky
    /// and may migrate with the attached frame. This is `None` while we aren't
    /// processing sticky blocks. On the first sticky block, this will become
    /// `Some(true)` if migrating sticky blocks as usual would make a
    /// difference - if content was already placed in this region, this is given
    /// by `regions.may_progress()`, but at the very top of the region it
    /// instead requires a following region to be strictly taller, since
    /// migrating would otherwise just leave this region empty. Otherwise, it
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
    /// The target that can stop distribution early.
    target: TargetState<'a, 'b>,
}

/// A snapshot of the distribution state.
struct DistributionSnapshot<'a, 'b> {
    work: Work<'a, 'b>,
    items: usize,
    used: Size,
}

/// A laid out item in a distribution.
enum Item<'a, 'b> {
    /// An introspection tag.
    Tag(&'a Tag),
    /// Absolute spacing and its weakness level.
    Abs(Abs, u8),
    /// Fractional spacing or a fractional block.
    Fr(Fr, u8, Option<&'b SingleChild<'a>>),
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
            self.target.advance(child);
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
    /// - Returns `Err(Stop::TargetReached)` if a simulation reached its target.
    /// - Returns `Err(Stop::Error(_))` if there was a fatal error.
    fn child(&mut self, child: &'b Child<'a>) -> FlowResult<()> {
        match child {
            Child::Tag(tag) => self.tag(tag),
            Child::Rel(amount, weakness) => self.rel(*amount, *weakness),
            Child::Fr(fr, weakness) => self.fr(*fr, *weakness),
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

    /// Mark the amount of height used and reduce the region height accordingly.
    fn use_height(&mut self, amount: Abs) {
        self.regions.size.y -= amount;
        self.used.y += amount;
    }

    /// Processes relative spacing.
    fn rel(&mut self, amount: Rel<Abs>, weakness: u8) {
        let amount = amount.relative_to(self.regions.base().y);
        if weakness > 0 && !self.keep_weak_rel_spacing(amount, weakness) {
            return;
        }

        self.use_height(amount);
        self.items.push(Item::Abs(amount, weakness));
    }

    /// Processes fractional spacing.
    fn fr(&mut self, fr: Fr, weakness: u8) {
        if weakness > 0 && !self.keep_weak_fr_spacing(fr, weakness) {
            return;
        }

        // If we decided to keep the fr spacing, it's safe to trim previous
        // spacing as no stronger fr spacing can exist.
        self.trim_spacing();

        self.items.push(Item::Fr(fr, weakness, None));
    }

    /// Decides whether to keep weak spacing based on previous items. If there
    /// is a preceding weak spacing, it might be patched in place.
    fn keep_weak_rel_spacing(&mut self, amount: Abs, weakness: u8) -> bool {
        for item in self.items.iter_mut().rev() {
            match *item {
                // When previous weak relative spacing exists that's at most as
                // weak, we reuse the old item, set it to the maximum of both,
                // and discard the new item.
                Item::Abs(prev_amount, prev_weakness @ 1..) => {
                    if weakness <= prev_weakness
                        && (weakness < prev_weakness || amount > prev_amount)
                    {
                        *item = Item::Abs(amount, weakness);
                        self.use_height(amount - prev_amount);
                    }
                    return false;
                }
                // These are "peeked beyond" for spacing collapsing purposes.
                Item::Tag(_) | Item::Abs(_, 0) | Item::Placed(..) => {}
                // Any kind of fractional spacing destructs weak relative
                // spacing.
                Item::Fr(.., None) => return false,
                // These naturally support the spacing.
                Item::Frame(..) | Item::Fr(.., Some(_)) => return true,
            }
        }
        false
    }

    /// Decides whether to keep weak fractional spacing based on previous items.
    /// If there is a preceding weak spacing, it might be patched in place.
    fn keep_weak_fr_spacing(&mut self, fr: Fr, weakness: u8) -> bool {
        for item in self.items.iter_mut().rev() {
            match *item {
                // When previous weak fr spacing exists that's at most as weak,
                // we reuse the old item, set it to the maximum of both, and
                // discard the new item.
                Item::Fr(prev_fr, prev_weakness @ 1.., None) => {
                    if weakness <= prev_weakness
                        && (weakness < prev_weakness || fr > prev_fr)
                    {
                        *item = Item::Fr(fr, weakness, None);
                    }
                    return false;
                }
                // These are "peeked beyond" for spacing collapsing purposes.
                // Weak absolute spacing, in particular, will be trimmed once
                // we push the fractional spacing.
                Item::Tag(_) | Item::Abs(..) | Item::Placed(..) => {}
                // For weak + strong fr spacing, we keep both, same as for
                // weak + strong rel spacing.
                Item::Fr(.., None) => return true,
                // These naturally support the spacing.
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
                    self.use_height(-amount);
                    self.items.remove(i);
                    break;
                }
                Item::Fr(_, 1.., None) => {
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

    /// Whether the amount fits into the remaining region, taking into account
    /// column balancing limits.
    pub fn fits(&self, amount: Abs) -> bool {
        self.regions.size.y.fits(amount)
            && self
                .balancing_target
                // Add elements as long as the balancing target is not reached. By not including
                // the amount itself here, we avoid protruding items to cumulate in the last column.
                .is_none_or(|target| target.fits(self.used.y))
    }

    /// Processes a line of a paragraph.
    fn line(&mut self, line: &'b LineChild) -> FlowResult<()> {
        // If the line doesn't fit and a followup region may improve things,
        // finish the region.
        if !self.fits(line.frame.height()) && self.regions.may_progress() {
            return Err(Stop::Finish(Finish::Soft(StopPoint::Child)));
        }

        // If the line's need, which includes its own height and that of
        // following lines grouped by widow/orphan prevention, does not fit into
        // the current region, but does fit into the next region, finish the
        // region.
        if !self.fits(line.need)
            && self
                .regions
                .iter()
                .nth(1)
                .is_some_and(|region| region.y.fits(line.need))
        {
            return Err(Stop::Finish(Finish::Soft(StopPoint::Child)));
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
            self.composer.footnotes(
                &self.regions,
                &frame,
                Abs::zero(),
                false,
                Migration::ALLOW,
            )?;

            self.target.stop_if_reached(
                self.composer.work.head(),
                &frame,
                single.sticky,
                true,
            )?;

            self.flush_tags();
            self.items.push(Item::Fr(fr, 0, Some(single)));
            return Ok(());
        }

        // If the block doesn't fit and a followup region may improve things,
        // finish the region.
        if !self.fits(frame.height()) && self.regions.may_progress() {
            return Err(Stop::Finish(Finish::Soft(StopPoint::Child)));
        }

        self.frame(frame, single.align, single.sticky, false)
    }

    /// Processes a breakable block.
    fn multi(&mut self, multi: &'b MultiChild<'a>) -> FlowResult<()> {
        let mut pod = self.regions;

        // For column balancing, reduce the region size for layout.
        if let Some(lim) = self.balancing_target {
            let remaining = lim - self.used.y;
            pod.size.y.set_min(remaining);
        }

        // Skip directly if the region is already (over)full. `line` and
        // `single` implicitly do this through their `fits` checks.
        if pod.is_full() {
            return Err(Stop::Finish(Finish::Soft(StopPoint::Child)));
        }

        // Lay out the block.
        let (frame, spill) = multi.layout(self.composer.engine, pod)?;
        if frame.is_empty()
            && spill.as_ref().is_some_and(|s| s.exist_non_empty_frame)
            && self.regions.may_progress()
        {
            // If the first frame is empty, but there are non-empty frames in
            // the spill, the whole child should be put in the next region to
            // avoid any invisible orphans at the end of this region.
            return Err(Stop::Finish(Finish::Soft(StopPoint::Child)));
        }

        self.frame(frame, multi.align, multi.sticky, true)?;

        // If the block didn't fully fit into the current region, save it into
        // the `spill` and finish the region.
        if let Some(spill) = spill {
            self.composer.work.spill = Some(spill);
            self.composer.work.advance();

            // If this block is sticky, migrating the sticky suffix would
            // relayout the whole block from scratch. This is only worthwhile
            // if there is following in-flow content that the suffix must stay
            // attached to. Without it, stickiness is moot.
            if self.sticky.is_some()
                && !self
                    .composer
                    .work
                    .children
                    .iter()
                    // A forced break severs the stickiness relationship.
                    .take_while(|child| !matches!(child, Child::Break(_)))
                    .any(|child| {
                        matches!(
                            child,
                            Child::Line(_) | Child::Single(_) | Child::Multi(_)
                        )
                    })
            {
                self.sticky = None;
            }

            return Err(Stop::Finish(Finish::Soft(StopPoint::Unknown)));
        }

        Ok(())
    }

    /// Processes spillover from a breakable block.
    fn multi_spill(&mut self, spill: MultiSpill<'a, 'b>) -> FlowResult<()> {
        let mut pod = self.regions;

        // For column balancing, reduce the region size for layout.
        if let Some(lim) = self.balancing_target {
            let remaining = lim - self.used.y;
            pod.size.y.set_min(remaining);
        }

        // Skip directly if the region is already (over)full.
        if pod.is_full() {
            self.composer.work.spill = Some(spill);
            return Err(Stop::Finish(Finish::Soft(StopPoint::Unknown)));
        }

        // Lay out the spilled remains.
        let align = spill.align();
        let (frame, spill) = spill.layout(self.composer.engine, pod)?;
        self.frame(frame, align, false, true)?;

        // If there's still more, save it into the `spill` and finish the
        // region.
        if let Some(spill) = spill {
            self.composer.work.spill = Some(spill);
            return Err(Stop::Finish(Finish::Soft(StopPoint::Unknown)));
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
        // If the frame is sticky and we haven't remembered a preceding sticky
        // element, make a checkpoint which we can restore should we end on
        // this sticky element.
        //
        // The first sticky block within consecutive sticky blocks determines
        // whether this group of sticky blocks has stickiness disabled or not.
        //
        // The criteria used here is: if migrating this group of sticky blocks
        // together with the "attached" block can't improve the lack of space,
        // then we don't do so, and stickiness is disabled (at least, for this
        // region). Otherwise, migration is allowed. When content was already
        // placed in this region, this is `regions.may_progress()`. When we're
        // still at the start of the region, migrating would leave it empty, so
        // it only helps if a following region is strictly taller.
        //
        // Note that, since the whole region is checked, this ensures sticky
        // blocks at the top of a block - but not necessarily of the page - can
        // still be migrated.
        if sticky
            && self.sticky.is_none()
            && *self.stickable.get_or_insert_with(|| {
                if self.used.y.is_zero() {
                    let current = self.regions.size.y;
                    self.regions.backlog.iter().any(|&height| height > current)
                        || self.regions.last.is_some_and(|height| height > current)
                } else {
                    self.regions.may_progress()
                }
            })
        {
            self.sticky = Some(self.snapshot());
        }

        // Handle footnotes.
        //
        // This must happen before we forget a previous sticky snapshot below.
        // If a non-sticky frame's footnote doesn't fit, the frame and any
        // preceding sticky blocks attached to it need to migrate to the next
        // region together. Resetting the sticky state first would then strand
        // those sticky blocks in this region.
        self.composer.footnotes(
            &self.regions,
            &frame,
            frame.height(),
            breakable,
            Migration::ALLOW,
        )?;

        self.target.stop_if_reached(
            self.composer.work.head(),
            &frame,
            sticky,
            self.fits(frame.height()),
        )?;

        if !sticky && !frame.is_empty() {
            // If the frame isn't sticky, we can forget a previous snapshot. We
            // interrupt a group of sticky blocks, if there was one, so we reset
            // the saved stickable check for the next group of sticky blocks.
            self.sticky = None;
            self.stickable = None;
        }

        // Push an item for the frame.
        self.use_height(frame.height());
        self.used.x.set_max(frame.width());
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
            self.use_height(-weak_spacing);
            self.composer.float(
                placed,
                &self.regions,
                self.items.iter().any(|item| matches!(item, Item::Frame(..))),
                Migration::ALLOW,
            )?;
            self.use_height(weak_spacing);
        } else {
            let frame = placed.layout(self.composer.engine, self.regions.base())?;
            self.composer.footnotes(
                &self.regions,
                &frame,
                Abs::zero(),
                true,
                Migration::ALLOW,
            )?;
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
            return Err(Stop::Finish(Finish::Soft(StopPoint::Child)));
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
            return Err(Stop::Finish(Finish::Forced));
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
        stop: StopPoint,
    ) -> SourceResult<(Frame, Abs)> {
        if forced {
            // If this is the very end of the flow, flush pending tags.
            self.flush_tags();
        } else if !self.items.is_empty() && self.items.iter().all(Item::migratable) {
            // Restore the initial state of all items are migratable.
            self.restore(init);
        } else {
            // If we ended on a sticky block, but are not yet at the end of
            // the flow, restore the saved checkpoint to move the sticky
            // suffix to the next region. Only do so if the suffix and the
            // following child that stopped distribution will fit there, or if
            // a later region may improve things.
            if let Some(snapshot) = self.sticky.take()
                && self.should_restore_sticky(&snapshot, stop)?
            {
                self.restore(snapshot);
            }
        }

        self.trim_spacing();

        let used_height_without_fr = self.used.y;

        // Determine the sum of fractionals.
        let mut frs = Fr::zero();
        let mut has_fr_child = false;
        for item in &self.items {
            if let Item::Fr(v, _, child) = item {
                frs += *v;
                has_fr_child |= child.is_some();
            }
        }

        // When we have fractional spacing, occupy the remaining space with it.
        let mut fr_space = Abs::zero();
        if frs.get() > 0.0 && region.size.y.is_finite() {
            fr_space = region.size.y - self.used.y;
            self.used.y = region.size.y;
        }

        // Lay out fractionally sized blocks.
        let mut fr_frames = vec![];
        if has_fr_child {
            for item in &self.items {
                let Item::Fr(v, _, Some(single)) = item else { continue };
                let length = v.share(frs, fr_space);
                let pod = Region::new(Size::new(region.size.x, length), region.expand);
                let frame = single.layout(self.composer.engine, pod)?;
                self.used.x.set_max(frame.width());
                fr_frames.push(frame);
            }
        }

        // Also consider the width of insertions for alignment.
        if !region.expand.x {
            self.used.x.set_max(self.composer.insertion_width());
        }

        // Determine the region's size.
        let size = region.expand.select(region.size, self.used.min(region.size));
        let free = size.y - self.used.y;

        let mut output = Frame::soft(size);
        let mut ruler = FixedAlignment::Start;
        let mut offset = Abs::zero();
        let mut fr_frames = fr_frames.into_iter();

        // Position all items.
        let mut baseline_set = false;
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
                Item::Fr(v, _, single) => {
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

                    // The baseline of the whole region will be the set to the
                    // baseline of the first in-flow frame. For example, of the
                    // first paragraph, if there is more than one. But also,
                    // inside the paragraph itself, this will be the first line
                    // (since each line is laid out as a separate frame).
                    if !baseline_set {
                        if frame.has_baseline() {
                            output.set_baseline(y + frame.baseline());
                        }
                        baseline_set = true;
                    }

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

        Ok((output, used_height_without_fr))
    }

    /// Create a snapshot of the work and items.
    fn snapshot(&self) -> DistributionSnapshot<'a, 'b> {
        DistributionSnapshot {
            work: self.composer.work.clone(),
            items: self.items.len(),
            used: self.used,
        }
    }

    /// Restore a snapshot of the work and items.
    fn restore(&mut self, snapshot: DistributionSnapshot<'a, 'b>) {
        *self.composer.work = snapshot.work;
        self.items.truncate(snapshot.items);
        self.used = snapshot.used;
    }

    /// Whether restoring a suffix of sticky blocks to migrate it to the next
    /// region can improve layout.
    fn should_restore_sticky(
        &mut self,
        snapshot: &DistributionSnapshot<'a, 'b>,
        stop: StopPoint,
    ) -> SourceResult<bool> {
        // We need the stopping point to simulate, and a known stopping point
        // means it is at the work head.
        if matches!(stop, StopPoint::Unknown) {
            return Ok(true);
        }
        let Some(target) = self.composer.work.head() else { unreachable!() };

        // If the next region isn't the final one, a later region might still
        // help, so migrate and defer to a later pass.
        let mut regions = self.regions;
        regions.next();
        if regions.may_progress() {
            return Ok(true);
        }

        // Simulate! In case of a fatal error, reject migration.
        self.composer
            .simulate_sticky_migration(snapshot.work.clone(), target, regions)
            .or(Ok(false))
    }
}
