use super::*;


/// Layouts boxes block-style.
#[derive(Debug)]
pub struct StackLayouter {
    ctx: StackContext,
    actions: LayoutActionList,
    dimensions: Size2D,
    usable: Size2D,
    cursor: Size2D,
}

#[derive(Debug, Copy, Clone)]
pub struct StackContext {
    pub space: LayoutSpace,
}

impl StackLayouter {
    /// Create a new box layouter.
    pub fn new(ctx: StackContext) -> StackLayouter {
        let space = ctx.space;

        StackLayouter {
            ctx,
            actions: LayoutActionList::new(),
            dimensions: match ctx.space.alignment {
                Alignment::Left => Size2D::zero(),
                Alignment::Right => Size2D::with_x(space.usable().x),
            },
            usable: space.usable(),
            cursor: Size2D::new(match ctx.space.alignment {
                Alignment::Left => space.padding.left,
                Alignment::Right => space.dimensions.x - space.padding.right,
            }, space.padding.top),
        }
    }

    /// Get a reference to this layouter's context.
    pub fn ctx(&self) -> &StackContext {
        &self.ctx
    }

    /// Add a sublayout.
    pub fn add_box(&mut self, layout: Layout) -> LayoutResult<()> {
        // In the flow direction (vertical) add the layout and in the second
        // direction just consider the maximal size of any child layout.
        let new_size = Size2D {
            x: crate::size::max(self.dimensions.x, layout.dimensions.x),
            y: self.dimensions.y + layout.dimensions.y,
        };

        // Check whether this box fits.
        if self.overflows(new_size) {
            return Err(LayoutError::NotEnoughSpace);
        }

        self.dimensions = new_size;

        // Determine where to put the box. When we right-align it, we want the
        // cursor to point to the top-right corner of the box. Therefore, the
        // position has to be moved to the left by the width of the box.
        let position = match self.ctx.space.alignment {
            Alignment::Left => self.cursor,
            Alignment::Right => self.cursor - Size2D::with_x(layout.dimensions.x),
        };

        self.cursor.y += layout.dimensions.y;

        self.add_box_absolute(position, layout);

        Ok(())
    }

    /// Add a sublayout at an absolute position.
    pub fn add_box_absolute(&mut self, position: Size2D, layout: Layout) -> LayoutResult<()> {
        Ok(self.actions.add_box(position, layout))
    }

    /// Add space in between two boxes.
    pub fn add_space(&mut self, space: Size) -> LayoutResult<()> {
        // Check whether this space fits.
        if self.overflows(self.dimensions + Size2D::with_y(space)) {
            return Err(LayoutError::NotEnoughSpace);
        }

        self.cursor.y += space;
        self.dimensions.y += space;

        Ok(())
    }

    /// The remaining space for new boxes.
    pub fn remaining(&self) -> Size2D {
        Size2D {
            x: self.usable.x,
            y: self.usable.y - self.dimensions.y,
        }
    }

    /// Whether this layouter contains any items.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Finish the layouting and create a box layout from this.
    pub fn finish(self) -> Layout {
        Layout {
            dimensions: if self.ctx.space.shrink_to_fit {
                self.dimensions.padded(self.ctx.space.padding)
            } else {
                self.ctx.space.dimensions
            },
            actions: self.actions.into_vec(),
            debug_render: true,
        }
    }

    /// Whether the given box is bigger than what we can hold.
    fn overflows(&self, dimensions: Size2D) -> bool {
        dimensions.x > self.usable.x || dimensions.y > self.usable.y
    }
}
