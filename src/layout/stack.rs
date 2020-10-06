//! Arranging boxes into a stack along the main axis.
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
    /// The layouting directions.
    pub dirs: Gen2<Dir>,
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
    pub fn add(&mut self, layout: BoxLayout, aligns: Gen2<GenAlign>) {
        // If the alignment cannot be fitted in this space, finish it.
        // TODO: Issue warning for non-fitting alignment in non-repeating
        // context.
        if !self.update_rulers(aligns) && self.ctx.repeat {
            self.finish_space(true);
        }

        // Now, we add a possibly cached soft space. If the main alignment
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
        self.update_metrics(layout.size.generalized(self.ctx.dirs));

        // Add the box to the vector and remember that spacings are allowed
        // again.
        self.space.layouts.push((self.ctx.dirs, aligns, layout));
        self.space.last_spacing = LastSpacing::None;
    }

    /// Add spacing to the stack.
    pub fn add_spacing(&mut self, mut spacing: f64, kind: SpacingKind) {
        match kind {
            // A hard space is simply an empty box.
            SpacingKind::Hard => {
                // Reduce the spacing such that it definitely fits.
                let axis = self.ctx.dirs.main.axis();
                spacing = spacing.min(self.space.usable.get(axis));

                let size = Size::new(0.0, spacing);
                self.update_metrics(size);
                self.space.layouts.push((
                    self.ctx.dirs,
                    Gen2::default(),
                    BoxLayout::new(size.specialized(self.ctx.dirs)),
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
        let mut size = self.space.size.generalized(self.ctx.dirs);
        let mut extra = self.space.extra.generalized(self.ctx.dirs);

        size.width += (added.width - extra.width).max(0.0);
        size.height += (added.height - extra.height).max(0.0);
        extra.width = extra.width.max(added.width);
        extra.height = (extra.height - added.height).max(0.0);

        self.space.size = size.specialized(self.ctx.dirs);
        self.space.extra = extra.specialized(self.ctx.dirs);
        *self.space.usable.get_mut(self.ctx.dirs.main.axis()) -= added.height;
    }

    /// Returns true if a space break is necessary.
    fn update_rulers(&mut self, aligns: Gen2<GenAlign>) -> bool {
        let allowed = self.is_fitting_alignment(aligns);
        if allowed {
            let side = self.ctx.dirs.main.side(GenAlign::Start);
            *self.space.rulers.get_mut(side) = aligns.main;
        }
        allowed
    }

    /// Whether a layout with the given alignment can still be layouted into the
    /// active space or a space break is necessary.
    pub(crate) fn is_fitting_alignment(&self, aligns: Gen2<GenAlign>) -> bool {
        self.is_fitting_axis(self.ctx.dirs.main, aligns.main)
            && self.is_fitting_axis(self.ctx.dirs.cross, aligns.cross)
    }

    fn is_fitting_axis(&self, dir: Dir, align: GenAlign) -> bool {
        align >= self.space.rulers.get(dir.side(GenAlign::Start))
            && align <= self.space.rulers.get(dir.side(GenAlign::End)).inv()
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
        let mut spaces = vec![LayoutSpace {
            size: self.usable(),
            insets: Insets::ZERO,
            expansion: Spec2::new(false, false),
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
                .specialized(self.ctx.dirs)
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

        for &(dirs, _, ref layout) in &self.space.layouts {
            // First, we store the bounds calculated so far (which were reduced
            // by the predecessors of this layout) as the initial bounding box
            // of this layout.
            bounds.push(bound);

            // Then, we reduce the bounding box for the following layouts. This
            // layout uses up space from the origin to the end. Thus, it reduces
            // the usable space for following layouts at its origin by its
            // main-axis extent.
            *bound.get_mut(dirs.main.side(GenAlign::Start)) +=
                dirs.main.factor() * layout.size.get(dirs.main.axis());
        }

        // ------------------------------------------------------------------ //
        // Step 3: Backward pass. Reduce the bounding boxes from the previous
        // layouts by what is taken by the following ones.

        // The `x` field stores the maximal cross-axis extent in one
        // axis-aligned run, while the `y` fields stores the accumulated
        // main-axis extent.
        let mut extent = Size::ZERO;
        let mut rotation = SpecAxis::Vertical;

        for (bound, entry) in bounds.iter_mut().zip(&self.space.layouts).rev() {
            let &(dirs, _, ref layout) = entry;

            // When the axes are rotated, the maximal cross-axis size
            // (`extent.x`) dictates how much main-axis extent the whole run
            // had. This value is thus stored in `extent.y`. The cross-axis
            // extent is reset for this new axis-aligned run.
            if rotation != dirs.main.axis() {
                extent.height = extent.width;
                extent.width = 0.0;
                rotation = dirs.main.axis();
            }

            // We reduce the bounding box of this layout at its end by the
            // accumulated main-axis extent of all layouts we have seen so far,
            // which are the layouts after this one since we iterate reversed.
            *bound.get_mut(dirs.main.side(GenAlign::End)) -=
                dirs.main.factor() * extent.height;

            // Then, we add this layout's main-axis extent to the accumulator.
            let size = layout.size.generalized(dirs);
            extent.width = extent.width.max(size.width);
            extent.height += size.height;
        }

        // ------------------------------------------------------------------ //
        // Step 4: Align each layout in its bounding box and collect everything
        // into a single finished layout.

        let mut layout = BoxLayout::new(size);

        let layouts = std::mem::take(&mut self.space.layouts);
        for ((dirs, aligns, child), bound) in layouts.into_iter().zip(bounds) {
            let size = child.size.specialized(dirs);

            // The space in which this layout is aligned is given by the
            // distances between the borders of its bounding box.
            let usable = bound.size().generalized(dirs);
            let local = usable.anchor(dirs, aligns) - size.anchor(dirs, aligns);
            let pos = bound.origin() + local.to_size().specialized(dirs).to_vec2();

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

/// A layout space composed of subspaces which can have different directions and
/// alignments.
struct Space {
    /// The index of this space in `ctx.spaces`.
    index: usize,
    /// Whether to include a layout for this space even if it would be empty.
    hard: bool,
    /// The so-far accumulated layouts.
    layouts: Vec<(Gen2<Dir>, Gen2<GenAlign>, BoxLayout)>,
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
