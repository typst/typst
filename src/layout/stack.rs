//! Arranging boxes into a stack along the secondary axis.
//!
//! Individual layouts can be aligned at `Start`, `Center` or  `End` along both
//! axes. These alignments are with respect to the size of the finished layout
//! and not the total usable size. This means that a later layout can have
//! influence on the position of an earlier one. Consider the following example.
//! ```typst
//! [align: right][A word.]
//! [align: left][A sentence with a couple more words.]
//! ```
//! The resulting layout looks like this:
//! ```text
//! |--------------------------------------|
//! |                              A word. |
//! |                                      |
//! | A sentence with a couple more words. |
//! |--------------------------------------|
//! ```
//! The position of the first aligned box thus depends on the length of the
//! sentence in the second box.

use super::*;

/// Performs the stack layouting.
pub struct StackLayouter {
    /// The context used for stack layouting.
    ctx: StackContext,
    /// The finished layouts.
    layouts: Vec<BoxLayout>,
    /// The in-progress space.
    space: Space,
}

/// The context for stack layouting.
#[derive(Debug, Clone)]
pub struct StackContext {
    /// The initial layouting system, which can be updated through `set_sys`.
    pub sys: LayoutSystem,
    /// The spaces to layout into.
    pub spaces: Vec<LayoutSpace>,
    /// Whether to spill over into copies of the last space or finish layouting
    /// when the last space is used up.
    pub repeat: bool,
}

impl StackLayouter {
    /// Create a new stack layouter.
    pub fn new(ctx: StackContext) -> Self {
        let space = ctx.spaces[0];
        Self {
            ctx,
            layouts: vec![],
            space: Space::new(0, true, space.usable()),
        }
    }

    /// Add a layout to the stack.
    pub fn add(&mut self, layout: BoxLayout, align: LayoutAlign) {
        // If the alignment cannot be fitted in this space, finish it.
        // TODO: Issue warning for non-fitting alignment in non-repeating
        // context.
        if !self.update_rulers(align) && self.ctx.repeat {
            self.finish_space(true);
        }

        // Now, we add a possibly cached soft space. If the secondary alignment
        // changed before, a possibly cached space would have already been
        // discarded.
        if let LastSpacing::Soft(spacing, _) = self.space.last_spacing {
            self.add_spacing(spacing, SpacingKind::Hard);
        }

        // TODO: Issue warning about overflow if there is overflow.
        if !self.space.usable.fits(layout.size) && self.ctx.repeat {
            self.skip_to_fitting_space(layout.size);
        }

        // Change the usable space and size of the space.
        self.update_metrics(layout.size.generalized(self.ctx.sys));

        // Add the box to the vector and remember that spacings are allowed
        // again.
        self.space.layouts.push((self.ctx.sys, align, layout));
        self.space.last_spacing = LastSpacing::None;
    }

    /// Add spacing to the stack.
    pub fn add_spacing(&mut self, mut spacing: f64, kind: SpacingKind) {
        match kind {
            // A hard space is simply an empty box.
            SpacingKind::Hard => {
                // Reduce the spacing such that it definitely fits.
                let axis = self.ctx.sys.secondary.axis();
                spacing = spacing.min(self.space.usable.get(axis));

                let size = Size::new(0.0, spacing);
                self.update_metrics(size);
                self.space.layouts.push((
                    self.ctx.sys,
                    LayoutAlign::default(),
                    BoxLayout::new(size.specialized(self.ctx.sys)),
                ));

                self.space.last_spacing = LastSpacing::Hard;
            }

            // A soft space is cached if it is not consumed by a hard space or
            // previous soft space with higher level.
            SpacingKind::Soft(level) => {
                let consumes = match self.space.last_spacing {
                    LastSpacing::None => true,
                    LastSpacing::Soft(_, prev) if level < prev => true,
                    _ => false,
                };

                if consumes {
                    self.space.last_spacing = LastSpacing::Soft(spacing, level);
                }
            }
        }
    }

    fn update_metrics(&mut self, added: Size) {
        let sys = self.ctx.sys;

        let mut size = self.space.size.generalized(sys);
        let mut extra = self.space.extra.generalized(sys);

        size.width += (added.width - extra.width).max(0.0);
        size.height += (added.height - extra.height).max(0.0);

        extra.width = extra.width.max(added.width);
        extra.height = (extra.height - added.height).max(0.0);

        self.space.size = size.specialized(sys);
        self.space.extra = extra.specialized(sys);
        *self.space.usable.get_mut(sys.secondary.axis()) -= added.height;
    }

    /// Returns true if a space break is necessary.
    fn update_rulers(&mut self, align: LayoutAlign) -> bool {
        let allowed = self.is_fitting_alignment(align);
        if allowed {
            let side = self.ctx.sys.secondary.side(GenAlign::Start);
            *self.space.rulers.get_mut(side) = align.secondary;
        }
        allowed
    }

    /// Whether a layout with the given alignment can still be layouted into the
    /// active space or a space break is necessary.
    pub(crate) fn is_fitting_alignment(&self, align: LayoutAlign) -> bool {
        self.is_fitting_axis(self.ctx.sys.primary, align.primary)
            && self.is_fitting_axis(self.ctx.sys.secondary, align.secondary)
    }

    fn is_fitting_axis(&self, dir: Dir, align: GenAlign) -> bool {
        align >= self.space.rulers.get(dir.side(GenAlign::Start))
            && align <= self.space.rulers.get(dir.side(GenAlign::End)).inv()
    }

    /// Update the layouting system.
    pub fn set_sys(&mut self, sys: LayoutSystem) {
        // Forget the spacing because it is not relevant anymore.
        if sys.secondary != self.ctx.sys.secondary {
            self.space.last_spacing = LastSpacing::Hard;
        }

        self.ctx.sys = sys;
    }

    /// Update the layouting spaces.
    ///
    /// If `replace_empty` is true, the current space is replaced if there are
    /// no boxes laid out into it yet. Otherwise, the followup spaces are
    /// replaced.
    pub fn set_spaces(&mut self, spaces: Vec<LayoutSpace>, replace_empty: bool) {
        if replace_empty && self.space_is_empty() {
            self.ctx.spaces = spaces;
            self.start_space(0, self.space.hard);
        } else {
            self.ctx.spaces.truncate(self.space.index + 1);
            self.ctx.spaces.extend(spaces);
        }
    }

    /// Move to the first space that can fit the given size or do nothing
    /// if no space is capable of that.
    pub fn skip_to_fitting_space(&mut self, size: Size) {
        let start = self.next_space();
        for (index, space) in self.ctx.spaces[start ..].iter().enumerate() {
            if space.usable().fits(size) {
                self.finish_space(true);
                self.start_space(start + index, true);
                break;
            }
        }
    }

    /// The remaining inner spaces. If something is laid out into these spaces,
    /// it will fit into this stack.
    pub fn remaining(&self) -> Vec<LayoutSpace> {
        let size = self.usable();

        let mut spaces = vec![LayoutSpace {
            size,
            insets: Insets::ZERO,
            expansion: LayoutExpansion::new(false, false),
        }];

        for space in &self.ctx.spaces[self.next_space() ..] {
            spaces.push(space.inner());
        }

        spaces
    }

    /// The remaining usable size.
    pub fn usable(&self) -> Size {
        self.space.usable
            - Size::new(0.0, self.space.last_spacing.soft_or_zero())
                .specialized(self.ctx.sys)
    }

    /// Whether the current layout space is empty.
    pub fn space_is_empty(&self) -> bool {
        self.space.size == Size::ZERO && self.space.layouts.is_empty()
    }

    /// Whether the current layout space is the last in the followup list.
    pub fn space_is_last(&self) -> bool {
        self.space.index == self.ctx.spaces.len() - 1
    }

    /// Finish everything up and return the final collection of boxes.
    pub fn finish(mut self) -> Vec<BoxLayout> {
        if self.space.hard || !self.space_is_empty() {
            self.finish_space(false);
        }
        self.layouts
    }

    /// Finish active current space and start a new one.
    pub fn finish_space(&mut self, hard: bool) {
        let space = self.ctx.spaces[self.space.index];

        // ------------------------------------------------------------------ //
        // Step 1: Determine the full size of the space.
        // (Mostly done already while collecting the boxes, but here we
        //  expand if necessary.)

        let usable = space.usable();
        if space.expansion.horizontal {
            self.space.size.width = usable.width;
        }
        if space.expansion.vertical {
            self.space.size.height = usable.height;
        }

        let size = self.space.size - space.insets.size();

        // ------------------------------------------------------------------ //
        // Step 2: Forward pass. Create a bounding box for each layout in which
        // it will be aligned. Then, go forwards through the boxes and remove
        // what is taken by previous layouts from the following layouts.

        let start = space.start();

        let mut bounds = vec![];
        let mut bound = Rect {
            x0: start.x,
            y0: start.y,
            x1: start.x + self.space.size.width,
            y1: start.y + self.space.size.height,
        };

        for (sys, _, layout) in &self.space.layouts {
            // First, we store the bounds calculated so far (which were reduced
            // by the predecessors of this layout) as the initial bounding box
            // of this layout.
            bounds.push(bound);

            // Then, we reduce the bounding box for the following layouts. This
            // layout uses up space from the origin to the end. Thus, it reduces
            // the usable space for following layouts at its origin by its
            // extent along the secondary axis.
            *bound.get_mut(sys.secondary, GenAlign::Start) +=
                sys.secondary.factor() * layout.size.get(sys.secondary.axis());
        }

        // ------------------------------------------------------------------ //
        // Step 3: Backward pass. Reduce the bounding boxes from the previous
        // layouts by what is taken by the following ones.

        // The `x` field stores the maximal primary extent in one axis-aligned
        // run, while the `y` fields stores the accumulated secondary extent.
        let mut extent = Size::ZERO;
        let mut rotation = SpecAxis::Vertical;

        for (bound, entry) in bounds.iter_mut().zip(&self.space.layouts).rev() {
            let (sys, _, layout) = entry;

            // When the axes are rotated, the maximal primary size (`extent.x`)
            // dictates how much secondary extent the whole run had. This value
            // is thus stored in `extent.y`. The primary extent is reset for
            // this new axis-aligned run.
            if rotation != sys.secondary.axis() {
                extent.height = extent.width;
                extent.width = 0.0;
                rotation = sys.secondary.axis();
            }

            // We reduce the bounding box of this layout at its end by the
            // accumulated secondary extent of all layouts we have seen so far,
            // which are the layouts after this one since we iterate reversed.
            *bound.get_mut(sys.secondary, GenAlign::End) -=
                sys.secondary.factor() * extent.height;

            // Then, we add this layout's secondary extent to the accumulator.
            let size = layout.size.generalized(*sys);
            extent.width = extent.width.max(size.width);
            extent.height += size.height;
        }

        // ------------------------------------------------------------------ //
        // Step 4: Align each layout in its bounding box and collect everything
        // into a single finished layout.

        let mut layout = BoxLayout::new(size);

        let layouts = std::mem::take(&mut self.space.layouts);
        for ((sys, align, child), bound) in layouts.into_iter().zip(bounds) {
            let size = child.size.specialized(sys);

            // The space in which this layout is aligned is given by the
            // distances between the borders of its bounding box.
            let usable = bound.size().generalized(sys);
            let local = usable.anchor(align, sys) - size.anchor(align, sys);
            let pos = bound.origin() + local.to_size().specialized(sys).to_vec2();

            layout.push_layout(pos, child);
        }

        self.layouts.push(layout);

        // ------------------------------------------------------------------ //
        // Step 5: Start the next space.

        self.start_space(self.next_space(), hard)
    }

    fn start_space(&mut self, index: usize, hard: bool) {
        let space = self.ctx.spaces[index];
        self.space = Space::new(index, hard, space.usable());
    }

    fn next_space(&self) -> usize {
        (self.space.index + 1).min(self.ctx.spaces.len() - 1)
    }
}

/// A layout space composed of subspaces which can have different systems and
/// alignments.
struct Space {
    /// The index of this space in `ctx.spaces`.
    index: usize,
    /// Whether to include a layout for this space even if it would be empty.
    hard: bool,
    /// The so-far accumulated layouts.
    layouts: Vec<(LayoutSystem, LayoutAlign, BoxLayout)>,
    /// The specialized size of this space.
    size: Size,
    /// The specialized remaining space.
    usable: Size,
    /// The specialized extra-needed size to affect the size at all.
    extra: Size,
    /// Dictate which alignments for new boxes are still allowed and which
    /// require a new space to be started. For example, after an `End`-aligned
    /// item, no `Start`-aligned one can follow.
    rulers: Sides<GenAlign>,
    /// The spacing state. This influences how new spacing is handled, e.g. hard
    /// spacing may override soft spacing.
    last_spacing: LastSpacing,
}

impl Space {
    fn new(index: usize, hard: bool, usable: Size) -> Self {
        Self {
            index,
            hard,
            layouts: vec![],
            size: Size::ZERO,
            usable,
            extra: Size::ZERO,
            rulers: Sides::uniform(GenAlign::Start),
            last_spacing: LastSpacing::Hard,
        }
    }
}

/// The spacing kind of the most recently inserted item in a layouting process.
///
/// Since the last inserted item may not be spacing at all, this can be `None`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum LastSpacing {
    /// The last item was hard spacing.
    Hard,
    /// The last item was soft spacing with the given width and level.
    Soft(f64, u32),
    /// The last item wasn't spacing.
    None,
}

impl LastSpacing {
    /// The width of the soft space if this is a soft space or zero otherwise.
    fn soft_or_zero(self) -> f64 {
        match self {
            LastSpacing::Soft(space, _) => space,
            _ => 0.0,
        }
    }
}
