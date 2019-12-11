use smallvec::smallvec;
use crate::size::{min, max};
use super::*;

/// The stack layouter arranges boxes stacked onto each other.
///
/// The boxes are laid out in the direction of the secondary layouting axis and
/// are aligned along both axes.
#[derive(Debug, Clone)]
pub struct StackLayouter {
    /// The context for layouter.
    ctx: StackContext,
    /// The output layouts.
    layouts: MultiLayout,
    /// The currently active layout space.
    space: Space,
}

/// The context for stack layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Debug, Clone)]
pub struct StackContext {
    pub spaces: LayoutSpaces,
    pub axes: LayoutAxes,
    pub alignment: LayoutAlignment,
}

/// A layout space composed of subspaces which can have different axes and
/// alignments.
#[derive(Debug, Clone)]
struct Space {
    /// The index of this space in the list of spaces.
    index: usize,
    /// Whether to add the layout for this space even if it would be empty.
    hard: bool,
    /// The so-far accumulated subspaces.
    layouts: Vec<(LayoutAxes, Layout)>,
    /// The specialized size of this subspace.
    size: Size2D,
    /// The specialized remaining space.
    usable: Size2D,
    /// The specialized extra-needed dimensions to affect the size at all.
    extra: Size2D,
    /// The maximal secondary alignment for both specialized axes (horizontal,
    /// vertical).
    alignment: (Alignment, Alignment),
    /// The last added spacing if the last added thing was spacing.
    last_spacing: LastSpacing,
}

impl StackLayouter {
    /// Create a new stack layouter.
    pub fn new(ctx: StackContext) -> StackLayouter {
        let axes = ctx.axes;
        let space = ctx.spaces[0];

        StackLayouter {
            ctx,
            layouts: MultiLayout::new(),
            space: Space::new(0, true, space.usable()),
        }
    }

    /// Add a layout to the stack.
    pub fn add(&mut self, layout: Layout) -> LayoutResult<()> {
        // If the layout's secondary alignment is less than what we have already
        // seen, it needs to go into the next space.
        if layout.alignment.secondary < *self.secondary_alignment() {
            self.finish_space(true);
        }

        if layout.alignment.secondary == *self.secondary_alignment() {
            // Add a cached soft space if there is one and the alignment stayed
            // the same. Soft spaces are discarded if the alignment changes.
            if let LastSpacing::Soft(spacing, _) = self.space.last_spacing {
                self.add_spacing(spacing, SpacingKind::Hard);
            }
        } else {
            // We want the new maximal alignment and since the layout's
            // secondary alignment is at least the previous maximum, we just
            // take it.
            *self.secondary_alignment() = layout.alignment.secondary;
        }

        // Find the first space that fits the layout.
        while !self.space.usable.fits(layout.dimensions) {
            if self.space_is_last() && self.space_is_empty() {
                error!("box of size {} does not fit into remaining usable size {}",
                    layout.dimensions, self.space.usable);
            }

            self.finish_space(true);
        }

        self.update_metrics(layout.dimensions.generalized(self.ctx.axes));

        self.space.layouts.push((self.ctx.axes, layout));
        self.space.last_spacing = LastSpacing::None;

        Ok(())
    }

    /// Add multiple layouts to the stack.
    ///
    /// This function simply calls `add` for each layout.
    pub fn add_multiple(&mut self, layouts: MultiLayout) -> LayoutResult<()> {
        for layout in layouts {
            self.add(layout)?;
        }
        Ok(())
    }

    /// Add secondary spacing to the stack.
    pub fn add_spacing(&mut self, mut spacing: Size, kind: SpacingKind) {
        match kind {
            // A hard space is simply an empty box.
            SpacingKind::Hard => {
                // Reduce the spacing such that it definitely fits.
                spacing.min_eq(self.space.usable.secondary(self.ctx.axes));
                let dimensions = Size2D::with_y(spacing);

                self.update_metrics(dimensions);

                self.space.layouts.push((self.ctx.axes, Layout {
                    dimensions: dimensions.specialized(self.ctx.axes),
                    alignment: LayoutAlignment::default(),
                    actions: vec![]
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

    /// Update the size metrics to reflect that a layout or spacing with the
    /// given generalized dimensions has been added.
    fn update_metrics(&mut self, dimensions: Size2D) {
        let axes = self.ctx.axes;

        let mut size = self.space.size.generalized(axes);
        let mut extra = self.space.extra.generalized(axes);

        size.x += max(dimensions.x - extra.x, Size::ZERO);
        size.y += max(dimensions.y - extra.y, Size::ZERO);
        extra.x = max(extra.x, dimensions.x);
        extra.y = max(extra.y - dimensions.y, Size::ZERO);

        self.space.size = size.specialized(axes);
        self.space.extra = extra.specialized(axes);
        *self.space.usable.secondary_mut(axes) -= dimensions.y;
    }

    /// Change the layouting axes used by this layouter.
    ///
    /// This starts a new subspace (if the axes are actually different from the
    /// current ones).
    pub fn set_axes(&mut self, axes: LayoutAxes) {
        // Forget the spacing because it is not relevant anymore.
        if axes.secondary != self.ctx.axes.secondary {
            self.space.last_spacing = LastSpacing::Hard;
        }

        self.ctx.axes = axes;
    }

    /// Change the layouting spaces to use.
    ///
    /// If `replace_empty` is true, the current space is replaced if there are
    /// no boxes laid into it yet. Otherwise, only the followup spaces are
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

    /// The remaining unpadded, unexpanding spaces. If a multi-layout is laid
    /// out into these spaces, it will fit into this stack.
    pub fn remaining(&self) -> LayoutSpaces {
        let mut spaces = smallvec![LayoutSpace {
            dimensions: self.space.usable,
            padding: SizeBox::ZERO,
            expand: LayoutExpansion::new(false, false),
        }];

        for space in &self.ctx.spaces[self.next_space()..] {
            spaces.push(space.usable_space());
        }

        spaces
    }

    /// The usable size along the primary axis.
    pub fn primary_usable(&self) -> Size {
        self.space.usable.primary(self.ctx.axes)
    }

    /// Whether the current layout space (not subspace) is empty.
    pub fn space_is_empty(&self) -> bool {
        self.space.size == Size2D::ZERO && self.space.layouts.is_empty()
    }

    /// Whether the current layout space is the last is the followup list.
    pub fn space_is_last(&self) -> bool {
        self.space.index == self.ctx.spaces.len() - 1
    }

    /// Compute the finished multi-layout.
    pub fn finish(mut self) -> MultiLayout {
        if self.space.hard || !self.space_is_empty() {
            self.finish_space(false);
        }
        self.layouts
    }

    /// Finish the current space and start a new one.
    pub fn finish_space(&mut self, hard: bool) {
        let space = self.ctx.spaces[self.space.index];

        // ------------------------------------------------------------------ //
        // Step 1: Determine the full dimensions of the space.
        // (Mostly done already while collecting the boxes, but here we
        //  expand if necessary.)

        let usable = space.usable();
        if space.expand.horizontal { self.space.size.x = usable.x; }
        if space.expand.vertical   { self.space.size.y = usable.y; }

        let dimensions = self.space.size.padded(space.padding);

        // ------------------------------------------------------------------ //
        // Step 2: Forward pass. Create a bounding box for each layout in which
        // it will be aligned. Then, go forwards through the boxes and remove
        // what is taken by previous layouts from the following layouts.

        let start = space.start();

        let mut bounds = vec![];
        let mut bound = SizeBox {
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
            *bound.secondary_origin_mut(*axes)
                += axes.secondary.factor() * layout.dimensions.secondary(*axes);
        }

        // ------------------------------------------------------------------ //
        // Step 3: Backward pass. Reduce the bounding boxes from the previous
        // layouts by what is taken by the following ones.

        let mut extent = Size::ZERO;

        for (bound, entry) in bounds.iter_mut().zip(&self.space.layouts).rev() {
            let (axes, layout) = entry;

            // We reduce the bounding box of this layout at it's end by the
            // accumulated secondary extent of all layouts we have seen so far,
            // which are the layouts after this one since we iterate reversed.
            *bound.secondary_end_mut(*axes) -= axes.secondary.factor() * extent;

            // Then, we add this layout's secondary extent to the accumulator.
            extent += layout.dimensions.secondary(*axes);
        }

        // ------------------------------------------------------------------ //
        // Step 4: Align each layout in its bounding box and collect everything
        // into a single finished layout.

        let mut actions = LayoutActions::new();
        actions.add(LayoutAction::DebugBox(dimensions));

        let layouts = std::mem::replace(&mut self.space.layouts, vec![]);

        for ((axes, layout), bound) in layouts.into_iter().zip(bounds) {
            let LayoutAxes { primary, secondary } = axes;

            let size = layout.dimensions.specialized(axes);
            let alignment = layout.alignment;

            // The space in which this layout is aligned is given by it's
            // corresponding bound box.
            let usable = Size2D::new(
                bound.right - bound.left,
                bound.bottom - bound.top
            ).generalized(axes);

            let offsets = Size2D {
                x: usable.x.anchor(alignment.primary, primary.is_positive())
                    - size.x.anchor(alignment.primary, primary.is_positive()),
                y: usable.y.anchor(alignment.secondary, secondary.is_positive())
                    - size.y.anchor(alignment.secondary, secondary.is_positive()),
            };

            let position = Size2D::new(bound.left, bound.top)
                + offsets.specialized(axes);

            println!("pos: {}", position);
            println!("usable: {}", usable);
            println!("size: {}", size);

            actions.add_layout(position, layout);
        }

        self.layouts.push(Layout {
            dimensions,
            alignment: self.ctx.alignment,
            actions: actions.to_vec(),
        });

        self.start_space(self.next_space(), hard);
    }

    /// Start a new space with the given index.
    fn start_space(&mut self, index: usize, hard: bool) {
        let space = self.ctx.spaces[index];
        self.space = Space::new(index, hard, space.usable());
    }

    /// The index of the next space.
    fn next_space(&self) -> usize {
        (self.space.index + 1).min(self.ctx.spaces.len() - 1)
    }

    // Access the secondary alignment in the current system of axes.
    fn secondary_alignment(&mut self) -> &mut Alignment {
        match self.ctx.axes.primary.is_horizontal() {
            true => &mut self.space.alignment.1,
            false => &mut self.space.alignment.0,
        }
    }
}

impl Space {
    fn new(index: usize, hard: bool, usable: Size2D) -> Space {
        Space {
            index,
            hard,
            layouts: vec![],
            size: Size2D::ZERO,
            usable,
            extra: Size2D::ZERO,
            alignment: (Alignment::Origin, Alignment::Origin),
            last_spacing: LastSpacing::Hard,
        }
    }
}
