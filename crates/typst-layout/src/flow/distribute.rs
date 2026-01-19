// Allow map_or pattern due to MSRV (is_none_or requires Rust 1.91.0)
#![allow(clippy::unnecessary_map_or)]

use typst_library::diag::warning;
use typst_library::foundations::Smart;
use typst_library::introspection::Tag;
use typst_library::layout::{
    Abs, Axes, FixedAlignment, Fr, Frame, FrameItem, ParExclusions, Point, Ratio, Region,
    Regions, Rel, Size, WrapFloat,
};
use typst_library::text::TextElem;
use typst_utils::Numeric;

/// Maximum number of iterative refinement passes for wrap-float exclusions.
/// Usually converges in 1-2 iterations; cap at 3 for pathological cases.
const MAX_WRAP_ITER: usize = 3;

use super::{
    Child, Composer, FlowResult, LineChild, MultiChild, MultiSpill, ParChild, ParSpill,
    PlacedChild, SingleChild, Stop, Work,
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
        wrap_state: WrapState::default(),
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
    /// State for tracking wrap-float exclusions that affect paragraph layout.
    wrap_state: WrapState,
}

/// A snapshot of the distribution state.
struct DistributionSnapshot<'a, 'b> {
    work: Work<'a, 'b>,
    items: usize,
}

/// State for tracking wrap-float exclusions during distribution.
///
/// Wrap-floats are positioned during distribution and create exclusion zones
/// that affect paragraph layout. This struct tracks all active wrap-floats
/// in the current region so that paragraphs can query for exclusions.
#[derive(Debug, Default)]
struct WrapState {
    /// Active wrap-floats in region coordinates.
    floats: Vec<WrapFloat>,
}

impl WrapState {
    /// Add a wrap-float to the exclusion map.
    fn add(&mut self, wf: WrapFloat) {
        self.floats.push(wf);
    }

    /// Build exclusions for a paragraph at the given y-position.
    ///
    /// Returns `None` if there are no exclusions overlapping the paragraph,
    /// which is an optimization to avoid unnecessary work in the common case.
    fn exclusions_for(
        &self,
        par_y: Abs,
        par_height_estimate: Abs,
    ) -> Option<ParExclusions> {
        if self.floats.is_empty() {
            return None;
        }
        let excl =
            ParExclusions::from_wrap_floats(par_y, par_height_estimate, &self.floats);
        if excl.is_empty() { None } else { Some(excl) }
    }

    /// Find the bottom of existing floats on the given side.
    ///
    /// Returns the y-coordinate where a new float can be placed without
    /// overlapping existing floats on the same side (left or right).
    /// Note: WrapFloat.height already includes clearance, so the returned
    /// position accounts for proper spacing.
    /// Returns zero if no floats exist on that side.
    fn bottom_of_floats_on_side(&self, align_x: FixedAlignment) -> Abs {
        let mut bottom = Abs::zero();
        for wf in &self.floats {
            // Check if this float is on the same side
            let same_side = match align_x {
                FixedAlignment::Start => wf.left_margin > Abs::zero(),
                FixedAlignment::End => wf.right_margin > Abs::zero(),
                FixedAlignment::Center => true, // Center floats conflict with both sides
            };
            if same_side {
                // wf.height includes clearance, so this gives proper spacing
                bottom = bottom.max(wf.y + wf.height);
            }
        }
        bottom
    }
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
    /// A frame for a wrap-float: an in-flow float that text will wrap around.
    /// Stores: frame, y-position (region-relative), x-alignment, delta offsets.
    WrapFloat(Frame, Abs, FixedAlignment, Axes<Rel<Abs>>),
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

        // Handle spill of a paragraph that broke across regions.
        if let Some(spill) = self.composer.work.par_spill.take() {
            self.par_spill(spill)?;
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
            Child::Fr(fr, weakness) => self.fr(*fr, *weakness),
            Child::Line(line) => self.line(line)?,
            Child::Par(par) => self.par(par)?,
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
        if weakness > 0 && !self.keep_weak_rel_spacing(amount, weakness) {
            return;
        }

        self.regions.size.y -= amount;
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
                        self.regions.size.y -= amount - prev_amount;
                        *item = Item::Abs(amount, weakness);
                    }
                    return false;
                }
                // These are "peeked beyond" for spacing collapsing purposes.
                Item::Tag(_)
                | Item::Abs(_, 0)
                | Item::Placed(..)
                | Item::WrapFloat(..) => {}
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
                Item::Tag(_) | Item::Abs(..) | Item::Placed(..) | Item::WrapFloat(..) => {
                }
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
                    self.regions.size.y += amount;
                    self.items.remove(i);
                    break;
                }
                Item::Fr(_, 1.., None) => {
                    self.items.remove(i);
                    break;
                }
                Item::Tag(_) | Item::Abs(..) | Item::Placed(..) | Item::WrapFloat(..) => {
                }
                Item::Frame(..) | Item::Fr(..) => break,
            }
        }
    }

    /// The amount of trailing weak spacing.
    fn weak_spacing(&mut self) -> Abs {
        for item in self.items.iter().rev() {
            match *item {
                Item::Abs(amount, 1..) => return amount,
                Item::Tag(_) | Item::Abs(..) | Item::Placed(..) | Item::WrapFloat(..) => {
                }
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

    /// Processes a paragraph with deferred layout.
    ///
    /// This measures and commits the paragraph, then processes each resulting
    /// frame like a line. If a frame doesn't fit, remaining frames are saved
    /// to `par_spill` for processing in the next region.
    ///
    /// When wrap-floats are present, uses iterative refinement to handle the
    /// circular dependency between paragraph height and exclusion zones:
    /// 1. Measure without exclusions to get height estimate
    /// 2. Compute exclusions based on that height
    /// 3. Re-measure with exclusions, check if line breaks changed
    /// 4. If changed, recompute exclusions and repeat (max 3 iterations)
    fn par(&mut self, par: &'b ParChild<'a>) -> FlowResult<()> {
        let current_y = self.current_y();

        // First measure without exclusions to estimate paragraph height.
        let initial_measured = par.measure(self.composer.engine, None)?;

        // Compute exclusions from wrap-floats based on estimated position and height.
        let initial_exclusions = self
            .wrap_state
            .exclusions_for(current_y, initial_measured.total_height);

        // If we have exclusions, use iterative refinement to converge on stable
        // line breaks. Otherwise, use the initial measurement directly.
        let (measured, final_exclusions) = if let Some(excl) = initial_exclusions {
            self.refine_paragraph_measure(par, current_y, excl, &initial_measured)?
        } else {
            (initial_measured, None)
        };

        // Commit with the same exclusions used in the final measure.
        let result =
            par.commit(self.composer.engine, &measured, final_exclusions.as_ref())?;

        // Warn if text overflowed the wrap-float gap (e.g., a word too wide to fit).
        if measured.has_overfull && final_exclusions.is_some() {
            self.composer.engine.sink.warn(warning!(
                par.elem.span(),
                "text overflows wrap-float gap; consider reducing float size or clearance"
            ));
        }

        // Compute widow/orphan prevention needs, replicating collector's lines() logic.
        let costs = par.styles.get(TextElem::costs);
        let len = result.frames.len();
        let prevent_orphans = costs.orphan() > Ratio::zero()
            && len >= 2
            && !result.frames.get(1).map_or(true, |f| f.is_empty());
        let prevent_widows = costs.widow() > Ratio::zero()
            && len >= 2
            && !result
                .frames
                .get(len.saturating_sub(2))
                .map_or(true, |f| f.is_empty());
        let prevent_all = len == 3 && prevent_orphans && prevent_widows;

        // Compute heights for widow/orphan logic.
        let height_at =
            |i: usize| result.frames.get(i).map(Frame::height).unwrap_or_default();
        let front_1 = height_at(0);
        let front_2 = height_at(1);
        let back_2 = height_at(len.saturating_sub(2));
        let back_1 = height_at(len.saturating_sub(1));
        let leading = par.leading;

        // Pre-compute (frame, need) pairs for all lines.
        let frames_with_needs: Vec<(Frame, Abs)> = result
            .frames
            .into_iter()
            .enumerate()
            .map(|(i, frame)| {
                let need = if prevent_all && i == 0 {
                    front_1 + leading + front_2 + leading + back_1
                } else if prevent_orphans && i == 0 {
                    front_1 + leading + front_2
                } else if prevent_widows && i >= 2 && i + 2 == len {
                    back_2 + leading + back_1
                } else {
                    frame.height()
                };
                (frame, need)
            })
            .collect();

        // Process frames through par_spill mechanism.
        let spill = ParSpill {
            par,
            frames: frames_with_needs.into_iter(),
            placed_count: 0,
            align: par.align,
            leading,
            had_exclusions: final_exclusions.is_some(),
            span: par.elem.span(),
        };

        // If par_spill saves remaining frames and returns Err, we must advance
        // past this Child::Par so it's not reprocessed in the next region.
        // This matches how multi() handles MultiSpill.
        match self.par_spill(spill) {
            Ok(()) => Ok(()),
            Err(Stop::Finish(forced)) => {
                self.composer.work.advance();
                Err(Stop::Finish(forced))
            }
            Err(other) => Err(other),
        }
    }

    /// Iterative refinement for paragraphs affected by wrap-float exclusions.
    ///
    /// Handles the circular dependency where paragraph height depends on line
    /// breaks, but line breaks depend on exclusion zones, which depend on where
    /// the paragraph ends up (its y-position and height).
    ///
    /// The algorithm:
    /// 1. Measure with initial exclusions
    /// 2. If line breaks changed from previous iteration, recompute exclusions
    ///    with the new height estimate and re-measure
    /// 3. Repeat until convergence (same break_info) or max iterations reached
    ///
    /// Usually converges in 1-2 iterations. Caps at MAX_WRAP_ITER to handle
    /// pathological cases where layout oscillates.
    fn refine_paragraph_measure(
        &mut self,
        par: &'b ParChild<'a>,
        par_y: Abs,
        initial_exclusions: ParExclusions,
        initial_measured: &crate::inline::ParMeasureResult,
    ) -> FlowResult<(crate::inline::ParMeasureResult, Option<ParExclusions>)> {
        let mut exclusions = initial_exclusions;
        let mut prev_break_info = initial_measured.break_info.clone();

        // Track seen break patterns to detect oscillation
        let mut seen_patterns: Vec<Vec<crate::inline::BreakInfo>> =
            vec![prev_break_info.clone()];

        for _iteration in 0..MAX_WRAP_ITER {
            // Measure with current exclusions
            let measured = par.measure(self.composer.engine, Some(&exclusions))?;

            // Check for convergence: same line breaks as previous iteration
            if measured.break_info == prev_break_info {
                return Ok((measured, Some(exclusions)));
            }

            // Check for oscillation: have we seen this break pattern before?
            if seen_patterns.contains(&measured.break_info) {
                // Oscillation detected - use current measurement and warn
                self.composer.engine.sink.warn(warning!(
                    par.elem.span(),
                    "wrap layout oscillating; using current approximation"
                ));
                return Ok((measured, Some(exclusions)));
            }

            seen_patterns.push(measured.break_info.clone());
            prev_break_info = measured.break_info.clone();

            // Recompute exclusions with the new height estimate
            let new_exclusions =
                self.wrap_state.exclusions_for(par_y, measured.total_height);

            match new_exclusions {
                Some(excl) => {
                    exclusions = excl;
                }
                None => {
                    // No longer overlapping any wrap-floats - measure without exclusions
                    let final_measured = par.measure(self.composer.engine, None)?;
                    return Ok((final_measured, None));
                }
            }
        }

        // Max iterations reached without convergence - warn and use last measurement
        self.composer.engine.sink.warn(warning!(
            par.elem.span(),
            "wrap layout did not converge after {} iterations",
            MAX_WRAP_ITER
        ));

        let final_measured = par.measure(self.composer.engine, Some(&exclusions))?;
        Ok((final_measured, Some(exclusions)))
    }

    /// Processes spillover from a paragraph that broke across regions.
    fn par_spill(&mut self, spill: ParSpill<'a, 'b>) -> FlowResult<()> {
        // Check if exclusion context changed between regions (in either direction).
        let current_region_has_exclusions = !self.wrap_state.floats.is_empty();
        let exclusions_changed = spill.had_exclusions != current_region_has_exclusions;

        // Compute current exclusions if needed for re-measurement.
        let current_exclusions = if current_region_has_exclusions {
            // Estimate paragraph height for exclusion computation.
            // Use a generous estimate since we're continuing a paragraph.
            let height_estimate = self.regions.size.y;
            self.wrap_state.exclusions_for(self.current_y(), height_estimate)
        } else {
            None
        };

        // If exclusions changed, re-measure the paragraph with current exclusions
        // and use the new frames instead of the cached ones.
        let frames_with_needs: Vec<(Frame, Abs)> = if exclusions_changed {
            // Re-measure and re-commit the paragraph with current region's exclusions.
            let measured =
                spill.par.measure(self.composer.engine, current_exclusions.as_ref())?;
            let result = spill.par.commit(
                self.composer.engine,
                &measured,
                current_exclusions.as_ref(),
            )?;

            // Compute widow/orphan prevention needs (same logic as in par()).
            let costs = spill.par.styles.get(TextElem::costs);
            let len = result.frames.len();
            let prevent_orphans = costs.orphan() > Ratio::zero()
                && len >= 2
                && !result.frames.get(1).map_or(true, |f| f.is_empty());
            let prevent_widows = costs.widow() > Ratio::zero()
                && len >= 2
                && !result
                    .frames
                    .get(len.saturating_sub(2))
                    .map_or(true, |f| f.is_empty());
            let prevent_all = len == 3 && prevent_orphans && prevent_widows;

            let height_at =
                |i: usize| result.frames.get(i).map(Frame::height).unwrap_or_default();
            let front_1 = height_at(0);
            let front_2 = height_at(1);
            let back_2 = height_at(len.saturating_sub(2));
            let back_1 = height_at(len.saturating_sub(1));
            let leading = spill.leading;

            // Skip frames that were already placed and compute needs for remaining.
            result
                .frames
                .into_iter()
                .enumerate()
                .skip(spill.placed_count)
                .map(|(i, frame)| {
                    let need = if prevent_all && i == 0 {
                        front_1 + leading + front_2 + leading + back_1
                    } else if prevent_orphans && i == 0 {
                        front_1 + leading + front_2
                    } else if prevent_widows && i >= 2 && i + 2 == len {
                        back_2 + leading + back_1
                    } else {
                        frame.height()
                    };
                    (frame, need)
                })
                .collect()
        } else {
            // Use cached frames.
            spill.frames.collect()
        };

        let mut frames_iter = frames_with_needs.into_iter();
        let mut placed_in_this_region = 0;
        let mut first = true;

        while let Some((frame, need)) = frames_iter.next() {
            // Add leading between lines (but not before first in this region).
            if !first {
                self.rel(spill.leading.into(), 5);
            }
            first = false;

            // If the line doesn't fit and a followup region may improve things,
            // save remaining frames and finish the region.
            if !self.regions.size.y.fits(frame.height()) && self.regions.may_progress() {
                // Put this frame back by creating new spill with it prepended.
                let remaining: Vec<(Frame, Abs)> =
                    std::iter::once((frame, need)).chain(frames_iter).collect();
                self.composer.work.par_spill = Some(ParSpill {
                    par: spill.par,
                    frames: remaining.into_iter(),
                    placed_count: spill.placed_count + placed_in_this_region,
                    align: spill.align,
                    leading: spill.leading,
                    // Track whether current frames were computed with exclusions.
                    had_exclusions: if exclusions_changed {
                        current_region_has_exclusions
                    } else {
                        spill.had_exclusions
                    },
                    span: spill.span,
                });
                return Err(Stop::Finish(false));
            }

            // If the line's need doesn't fit here but does fit in next region,
            // save remaining frames and finish the region.
            if !self.regions.size.y.fits(need)
                && self.regions.iter().nth(1).is_some_and(|region| region.y.fits(need))
            {
                let remaining: Vec<(Frame, Abs)> =
                    std::iter::once((frame, need)).chain(frames_iter).collect();
                self.composer.work.par_spill = Some(ParSpill {
                    par: spill.par,
                    frames: remaining.into_iter(),
                    placed_count: spill.placed_count + placed_in_this_region,
                    align: spill.align,
                    leading: spill.leading,
                    had_exclusions: !exclusions_changed && spill.had_exclusions,
                    span: spill.span,
                });
                return Err(Stop::Finish(false));
            }

            // Place the frame. If this fails (e.g., due to footnote migration),
            // save the current frame and remaining frames so they can be
            // processed in the next region.
            if let Err(err) = self.frame(frame.clone(), spill.align, false, false) {
                let remaining: Vec<(Frame, Abs)> =
                    std::iter::once((frame, need)).chain(frames_iter).collect();
                self.composer.work.par_spill = Some(ParSpill {
                    par: spill.par,
                    frames: remaining.into_iter(),
                    placed_count: spill.placed_count + placed_in_this_region,
                    align: spill.align,
                    leading: spill.leading,
                    had_exclusions: !exclusions_changed && spill.had_exclusions,
                    span: spill.span,
                });
                return Err(err);
            }

            placed_in_this_region += 1;
        }

        Ok(())
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
            self.items.push(Item::Fr(fr, 0, Some(single)));
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
        if placed.float && placed.wrap {
            // Wrap-floats: in-flow floats that text will wrap around.
            // Unlike regular floats, wrap-floats are positioned during
            // distribution and don't go through the composer's float handling.
            self.wrap_float(placed)?;
        } else if placed.float {
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

    /// Maximum ratio of column width a wrap-float can occupy (2/3).
    /// Floats wider than this leave too little room for text.
    const MAX_WRAP_WIDTH_RATIO: f64 = 2.0 / 3.0;

    /// Minimum ratio of column width that must remain for text (1/6).
    /// If the gap beside a wrap-float is smaller than this, warn the user.
    const MIN_WRAP_GAP_RATIO: f64 = 1.0 / 6.0;

    /// Processes a wrap-float: an in-flow float that text will wrap around.
    ///
    /// Unlike regular floats which go through the composer's insertion system,
    /// wrap-floats are positioned during distribution based on their alignment.
    /// They don't consume vertical space - text will eventually wrap around them.
    fn wrap_float(&mut self, placed: &'b PlacedChild<'a>) -> FlowResult<()> {
        // Layout the float content.
        let frame = placed.layout(self.composer.engine, self.regions.base())?;

        // Validate width: if the float is too wide (>2/3 of column), there's not
        // enough room for text to wrap. Fall back to regular float behavior.
        let base_width = self.regions.base().x;
        let max_wrap_width = base_width * Self::MAX_WRAP_WIDTH_RATIO;
        if frame.width() > max_wrap_width {
            self.composer.engine.sink.warn(warning!(
                placed.span(),
                "wrap-float too wide ({:.1}pt > {:.1}pt limit); treating as regular float",
                frame.width().to_pt(),
                max_wrap_width.to_pt()
            ));
            // Fall back to regular float handling through the composer.
            let weak_spacing = self.weak_spacing();
            self.regions.size.y += weak_spacing;
            self.composer.float(
                placed,
                &self.regions,
                self.items.iter().any(|item| matches!(item, Item::Frame(..))),
                true,
            )?;
            self.regions.size.y -= weak_spacing;
            return Ok(());
        }

        // Warn if the gap for text is too narrow. The exclusion includes clearance,
        // so the actual gap = base_width - frame_width - clearance.
        let exclusion_width = frame.width() + placed.clearance;
        let gap = base_width - exclusion_width;
        let min_gap = base_width * Self::MIN_WRAP_GAP_RATIO;
        if gap < min_gap {
            self.composer.engine.sink.warn(warning!(
                placed.span(),
                "wrap-float leaves too little room for text ({:.1}pt gap < {:.1}pt minimum)",
                gap.to_pt(),
                min_gap.to_pt()
            ));
        }

        // Compute y-position based on vertical alignment.
        // For top/bottom aligned floats, stack below/above existing floats on the same side.
        let region_height = self.regions.full;
        let float_height = frame.height();
        let current_y = self.current_y();

        // Find the bottom of existing floats on the same side to avoid overlap.
        let existing_bottom = self.wrap_state.bottom_of_floats_on_side(placed.align_x);

        let y = match placed.align_y {
            // Auto: float appears near its source position in the flow.
            Smart::Auto => current_y.max(existing_bottom),
            // Custom alignment: position at top/center/bottom of region.
            Smart::Custom(align) => match align {
                // Top-aligned: place at top, but don't overlap existing content or floats.
                // If content already exists on the page, float appears at current position.
                Some(FixedAlignment::Start) => existing_bottom.max(current_y),
                Some(FixedAlignment::End) => region_height - float_height,
                Some(FixedAlignment::Center) => (region_height - float_height) / 2.0,
                None => current_y.max(existing_bottom), // Fallback to current position.
            },
        };

        // Check if the float would overflow the page. If so, finish the current
        // region so the wrap-float is re-processed at the top of the next page.
        if y + float_height > region_height && self.regions.may_progress() {
            return Err(Stop::Finish(false));
        }

        // Handle any footnotes in the float content.
        self.composer
            .footnotes(&self.regions, &frame, Abs::zero(), true, true)?;

        // Register the wrap-float in WrapState for exclusion computation.
        // This allows subsequent paragraphs to query for exclusions.
        let wf = WrapFloat::from_placed(&frame, y, placed.align_x, placed.clearance);
        self.wrap_state.add(wf);

        // Push the wrap-float item. Note: wrap-floats don't consume vertical
        // space - they're rendered but text flows past them.
        self.flush_tags();
        self.items
            .push(Item::WrapFloat(frame, y, placed.align_x, placed.delta));

        Ok(())
    }

    /// Get the current y-position in the flow (how far down we've placed content).
    fn current_y(&self) -> Abs {
        self.regions.full - self.regions.size.y
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
                Item::Fr(v, _, child) => {
                    frs += *v;
                    has_fr_child |= child.is_some();
                }
                Item::Frame(frame, _) => {
                    used.y += frame.height();
                    used.x.set_max(frame.width());
                }
                Item::Tag(_) | Item::Placed(..) | Item::WrapFloat(..) => {}
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
                let Item::Fr(v, _, Some(single)) = item else { continue };
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
                Item::WrapFloat(frame, y, align_x, delta) => {
                    // Wrap-float: x from alignment, y was pre-computed during distribution.
                    let x = align_x.position(size.x - frame.width());
                    let pos = Point::new(x, y)
                        + delta.zip_map(size, Rel::relative_to).to_point();

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
