use smallvec::smallvec;
use super::*;

/// Layouts boxes stack-like.
///
/// The boxes are arranged along an axis, each layout gettings it's own "line".
#[derive(Debug, Clone)]
pub struct StackLayouter {
    ctx: StackContext,
    layouts: MultiLayout,

    merged_actions: LayoutActionList,
    merged_dimensions: Size2D,

    // Offset on secondary axis, anchor of the layout and the layout itself.
    boxes: Vec<(Size, Size2D, Layout)>,
    usable: Size2D,
    dimensions: Size2D,
    active_space: usize,
    hard: bool,
}

/// The context for stack layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Debug, Clone)]
pub struct StackContext {
    pub spaces: LayoutSpaces,
    pub axes: LayoutAxes,
    pub shrink_to_fit: bool,
}

impl StackLayouter {
    /// Create a new stack layouter.
    pub fn new(ctx: StackContext) -> StackLayouter {
        let space = ctx.spaces[0];
        let usable = ctx.axes.generalize(space.usable());

        StackLayouter {
            ctx,
            layouts: MultiLayout::new(),

            merged_actions: LayoutActionList::new(),
            merged_dimensions: Size2D::zero(),

            boxes: vec![],
            usable,
            active_space: 0,
            dimensions: Size2D::zero(),
            hard: true,
        }
    }

    /// Add a sublayout.
    pub fn add(&mut self, layout: Layout) -> LayoutResult<()> {
        let size = self.ctx.axes.generalize(layout.dimensions);
        let mut new_dimensions = merge_sizes(self.dimensions, size);

        // Search for a suitable space to insert the box.
        while !self.usable.fits(new_dimensions) {
            if self.boxes.is_empty() && self.in_last_space() {
                Err(LayoutError::NotEnoughSpace("cannot fit box into stack"))?;
            }

            self.finish_layout(true);
            new_dimensions = merge_sizes(self.dimensions, size);
        }

        let offset = self.dimensions.y;
        let anchor = self.ctx.axes.anchor(size);
        self.boxes.push((offset, anchor, layout));

        self.dimensions = new_dimensions;

        Ok(())
    }

    /// Add multiple sublayouts from a multi-layout.
    pub fn add_multiple(&mut self, layouts: MultiLayout) -> LayoutResult<()> {
        for layout in layouts {
            self.add(layout)?;
        }
        Ok(())
    }

    /// Add space after the last layout.
    pub fn add_space(&mut self, space: Size) {
        if self.dimensions.y + space > self.usable.y {
            self.finish_layout(false);
        } else {
            self.dimensions.y += space;
        }
    }

    /// Update the axes in use by this stack layouter.
    pub fn set_axes(&mut self, axes: LayoutAxes) {
        if axes != self.ctx.axes {
            self.finish_boxes();
            self.usable = self.remains();
            self.dimensions = Size2D::zero();
            self.ctx.axes = axes;
        }
    }

    /// Finish the layouting.
    ///
    /// The layouter is not consumed by this to prevent ownership problems.
    /// Nevertheless, it should not be used further.
    pub fn finish(&mut self) -> MultiLayout {
        if self.hard || !self.boxes.is_empty() {
            self.finish_layout(false);
        }
        std::mem::replace(&mut self.layouts, MultiLayout::new())
    }

    /// Finish the current layout and start a new one in a new space.
    pub fn finish_layout(&mut self, hard: bool) {
        self.finish_boxes();

        let space = self.ctx.spaces[self.active_space];
        let actions = std::mem::replace(&mut self.merged_actions, LayoutActionList::new());

        self.layouts.add(Layout {
            dimensions: if self.ctx.shrink_to_fit {
                self.merged_dimensions.padded(space.padding)
            } else {
                space.dimensions
            },
            actions: actions.into_vec(),
            debug_render: true,
        });

        let next_space = self.next_space();
        let space = self.ctx.spaces[next_space];

        self.merged_dimensions = Size2D::zero();

        self.usable = self.ctx.axes.generalize(space.usable());
        self.dimensions = Size2D::zero();
        self.active_space = next_space;
        self.hard = hard;
    }

    /// Compose all cached boxes into a layout.
    fn finish_boxes(&mut self) {
        let space = self.ctx.spaces[self.active_space];
        let start = space.start() + Size2D::with_y(self.merged_dimensions.y);

        let anchor = self.ctx.axes.anchor(self.usable);
        let factor = if self.ctx.axes.secondary.axis.is_positive() { 1 } else { -1 };

        for (offset, layout_anchor, layout) in self.boxes.drain(..) {
            let general_position = anchor - layout_anchor + Size2D::with_y(factor * offset);
            let position = start + self.ctx.axes.specialize(general_position);

            self.merged_actions.add_layout(position, layout);
        }

        let mut dimensions = self.ctx.axes.specialize(self.dimensions);
        let usable = self.ctx.axes.specialize(self.usable);

        if needs_expansion(self.ctx.axes.primary) {
            dimensions.x = usable.x;
        }

        if needs_expansion(self.ctx.axes.secondary) {
            dimensions.y = usable.y;
        }

        self.merged_dimensions = merge_sizes(self.merged_dimensions, dimensions);
    }

    /// This layouter's context.
    pub fn ctx(&self) -> &StackContext {
        &self.ctx
    }

    /// The (generalized) usable area of the current space.
    pub fn usable(&self) -> Size2D {
        self.usable
    }

    /// The remaining usable spaces for new layouts.
    pub fn remaining(&self) -> LayoutSpaces {
        let mut spaces = smallvec![LayoutSpace {
            dimensions: self.ctx.axes.specialize(self.remains()),
            padding: SizeBox::zero(),
        }];

        for space in &self.ctx.spaces[self.next_space()..] {
            spaces.push(space.usable_space());
        }

        spaces
    }

    fn remains(&self) -> Size2D {
        Size2D::new(self.usable.x, self.usable.y - self.dimensions.y)
    }

    /// Whether this layouter is in its last space.
    pub fn in_last_space(&self) -> bool {
        self.active_space == self.ctx.spaces.len() - 1
    }

    fn next_space(&self) -> usize {
        (self.active_space + 1).min(self.ctx.spaces.len() - 1)
    }
}

fn merge_sizes(a: Size2D, b: Size2D) -> Size2D {
    Size2D {
        x: crate::size::max(a.x, b.x),
        y: a.y + b.y
    }
}

fn needs_expansion(axis: AlignedAxis) -> bool {
    !matches!(
        (axis.axis.is_positive(), axis.alignment),
        (true, Alignment::Origin) | (false, Alignment::End)
    )
}
