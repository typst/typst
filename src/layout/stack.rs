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
use crate::geom::Value4;

/// Performs the stack layouting.
pub struct StackLayouter {
    ctx: StackContext,
    layouts: MultiLayout,
    /// The in-progress space.
    space: Space,
}

/// The context for stack layouting.
#[derive(Debug, Clone)]
pub struct StackContext {
    /// The spaces to layout into.
    pub spaces: LayoutSpaces,
    /// The initial layouting axes, which can be updated through `set_axes`.
    pub axes: LayoutAxes,
    /// The alignment of the _resulting_ layout. This does not effect the line
    /// layouting itself, but rather how the finished layout will be positioned
    /// in a parent layout.
    pub align: LayoutAlign,
    /// Whether to spill over into copies of the last space or finish layouting
    /// when the last space is used up.
    pub repeat: bool,
}

/// A layout space composed of subspaces which can have different axes and
/// alignments.
struct Space {
    /// The index of this space in `ctx.spaces`.
    index: usize,
    /// Whether to include a layout for this space even if it would be empty.
    hard: bool,
    /// The so-far accumulated layouts.
    layouts: Vec<(LayoutAxes, BoxLayout)>,
    /// The specialized size of this space.
    size: Size,
    /// The specialized remaining space.
    usable: Size,
    /// The specialized extra-needed size to affect the size at all.
    extra: Size,
    /// Dictate which alignments for new boxes are still allowed and which
    /// require a new space to be started. For example, after an `End`-aligned
    /// item, no `Start`-aligned one can follow.
    rulers: Value4<GenAlign>,
    /// The spacing state. This influences how new spacing is handled, e.g. hard
    /// spacing may override soft spacing.
    last_spacing: LastSpacing,
}

impl StackLayouter {
    /// Create a new stack layouter.
    pub fn new(ctx: StackContext) -> Self {
        let space = ctx.spaces[0];
        Self {
            ctx,
            layouts: MultiLayout::new(),
            space: Space::new(0, true, space.usable()),
        }
    }

    /// Add a layout to the stack.
    pub fn add(&mut self, layout: BoxLayout) {
        // If the alignment cannot be fitted in this space, finish it.
        // TODO: Issue warning for non-fitting alignment in non-repeating
        // context.
        if !self.update_rulers(layout.align) && self.ctx.repeat {
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
        self.update_metrics(layout.size.generalized(self.ctx.axes));

        // Add the box to the vector and remember that spacings are allowed
        // again.
        self.space.layouts.push((self.ctx.axes, layout));
        self.space.last_spacing = LastSpacing::None;
    }

    /// Add multiple layouts to the stack.
    ///
    /// This is equivalent to calling `add` repeatedly for each layout.
    pub fn add_multiple(&mut self, layouts: MultiLayout) {
        for layout in layouts {
            self.add(layout);
        }
    }

    /// Add spacing to the stack.
    pub fn add_spacing(&mut self, mut spacing: f64, kind: SpacingKind) {
        match kind {
            // A hard space is simply an empty box.
            SpacingKind::Hard => {
                // Reduce the spacing such that it definitely fits.
                spacing = spacing.min(self.space.usable.secondary(self.ctx.axes));
                let size = Size::with_y(spacing);

                self.update_metrics(size);
                self.space.layouts.push((self.ctx.axes, BoxLayout {
                    size: size.specialized(self.ctx.axes),
                    align: LayoutAlign::new(Start, Start),
                    elements: LayoutElements::new(),
                }));

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
        let axes = self.ctx.axes;

        let mut size = self.space.size.generalized(axes);
        let mut extra = self.space.extra.generalized(axes);

        size.x += (added.x - extra.x).max(0.0);
        size.y += (added.y - extra.y).max(0.0);

        extra.x = extra.x.max(added.x);
        extra.y = (extra.y - added.y).max(0.0);

        self.space.size = size.specialized(axes);
        self.space.extra = extra.specialized(axes);
        *self.space.usable.secondary_mut(axes) -= added.y;
    }

    /// Returns true if a space break is necessary.
    fn update_rulers(&mut self, align: LayoutAlign) -> bool {
        let allowed = self.is_fitting_alignment(align);
        if allowed {
            *self.space.rulers.get_mut(self.ctx.axes.secondary, Start) = align.secondary;
        }
        allowed
    }

    /// Whether a layout with the given alignment can still be layouted into the
    /// active space or a space break is necessary.
    pub(crate) fn is_fitting_alignment(&mut self, align: LayoutAlign) -> bool {
        self.is_fitting_axis(self.ctx.axes.primary, align.primary)
            && self.is_fitting_axis(self.ctx.axes.secondary, align.secondary)
    }

    fn is_fitting_axis(&mut self, dir: Dir, align: GenAlign) -> bool {
        align >= *self.space.rulers.get_mut(dir, Start)
            && align <= self.space.rulers.get_mut(dir, End).inv()
    }

    /// Update the layouting axes.
    pub fn set_axes(&mut self, axes: LayoutAxes) {
        // Forget the spacing because it is not relevant anymore.
        if axes.secondary != self.ctx.axes.secondary {
            self.space.last_spacing = LastSpacing::Hard;
        }

        self.ctx.axes = axes;
    }

    /// Update the layouting spaces.
    ///
    /// If `replace_empty` is true, the current space is replaced if there are
    /// no boxes laid out into it yet. Otherwise, the followup spaces are
    /// replaced.
    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
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
    pub fn remaining(&self) -> LayoutSpaces {
        let size = self.usable();

        let mut spaces = vec![LayoutSpace {
            size,
            padding: Margins::ZERO,
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
            - Size::with_y(self.space.last_spacing.soft_or_zero())
                .specialized(self.ctx.axes)
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
    pub fn finish(mut self) -> MultiLayout {
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
            self.space.size.x = usable.x;
        }
        if space.expansion.vertical {
            self.space.size.y = usable.y;
        }

        let size = self.space.size.padded(space.padding);

        // ------------------------------------------------------------------ //
        // Step 2: Forward pass. Create a bounding box for each layout in which
        // it will be aligned. Then, go forwards through the boxes and remove
        // what is taken by previous layouts from the following layouts.

        let start = space.start();

        let mut bounds = vec![];
        let mut bound = Margins {
            left: start.x,
            top: start.y,
            right: start.x + self.space.size.x,
            bottom: start.y + self.space.size.y,
        };

        for (axes, layout) in &self.space.layouts {
            // First, we store the bounds calculated so far (which were reduced
            // by the predecessors of this layout) as the initial bounding box
            // of this layout.
            bounds.push(bound);

            // Then, we reduce the bounding box for the following layouts. This
            // layout uses up space from the origin to the end. Thus, it reduces
            // the usable space for following layouts at it's origin by its
            // extent along the secondary axis.
            *bound.get_mut(axes.secondary, Start) +=
                axes.secondary.factor() * layout.size.secondary(*axes);
        }

        // ------------------------------------------------------------------ //
        // Step 3: Backward pass. Reduce the bounding boxes from the previous
        // layouts by what is taken by the following ones.

        // The `x` field stores the maximal primary extent in one axis-aligned
        // run, while the `y` fields stores the accumulated secondary extent.
        let mut extent = Size::ZERO;
        let mut rotation = Vertical;

        for (bound, entry) in bounds.iter_mut().zip(&self.space.layouts).rev() {
            let (axes, layout) = entry;

            // When the axes are rotated, the the maximal primary size
            // (`extent.x`) dictates how much secondary extent the whole run
            // had. This value is thus stored in `extent.y`. The primary extent
            // is reset for this new axis-aligned run.
            if rotation != axes.secondary.axis() {
                extent.y = extent.x;
                extent.x = 0.0;
                rotation = axes.secondary.axis();
            }

            // We reduce the bounding box of this layout at it's end by the
            // accumulated secondary extent of all layouts we have seen so far,
            // which are the layouts after this one since we iterate reversed.
            *bound.get_mut(axes.secondary, End) -= axes.secondary.factor() * extent.y;

            // Then, we add this layout's secondary extent to the accumulator.
            let size = layout.size.generalized(*axes);
            extent.x = extent.x.max(size.x);
            extent.y += size.y;
        }

        // ------------------------------------------------------------------ //
        // Step 4: Align each layout in its bounding box and collect everything
        // into a single finished layout.

        let mut elements = LayoutElements::new();

        let layouts = std::mem::take(&mut self.space.layouts);
        for ((axes, layout), bound) in layouts.into_iter().zip(bounds) {
            let size = layout.size.specialized(axes);
            let align = layout.align;

            // The space in which this layout is aligned is given by the
            // distances between the borders of it's bounding box.
            let usable = Size::new(bound.right - bound.left, bound.bottom - bound.top)
                .generalized(axes);

            let local = usable.anchor(align, axes) - size.anchor(align, axes);
            let pos = Size::new(bound.left, bound.top) + local.specialized(axes);

            elements.extend_offset(pos, layout.elements);
        }

        self.layouts.push(BoxLayout { size, align: self.ctx.align, elements });

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

impl Space {
    fn new(index: usize, hard: bool, usable: Size) -> Self {
        Self {
            index,
            hard,
            layouts: vec![],
            size: Size::ZERO,
            usable,
            extra: Size::ZERO,
            rulers: Value4::with_all(Start),
            last_spacing: LastSpacing::Hard,
        }
    }
}
