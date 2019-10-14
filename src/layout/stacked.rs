use super::*;

/// Stack-like layouting of boxes.
///
/// The boxes are arranged vertically, each layout gettings it's own "line".
pub struct StackLayouter {
    ctx: StackContext,
    actions: LayoutActionList,
    usable: Size2D,
    dimensions: Size2D,
    cursor: Size2D,
}

/// The context for the [`StackLayouter`].
#[derive(Debug, Copy, Clone)]
pub struct StackContext {
    pub space: LayoutSpace,
}

impl StackLayouter {
    /// Create a new stack layouter.
    pub fn new(ctx: StackContext) -> StackLayouter {
        let space = ctx.space;

        StackLayouter {
            ctx,
            actions: LayoutActionList::new(),

            usable: ctx.space.usable(),
            dimensions: match ctx.space.alignment {
                Alignment::Left => Size2D::zero(),
                Alignment::Right => Size2D::with_x(space.usable().x),
            },

            cursor: Size2D::new(
                // If left-align, the cursor points to the top-left corner of
                // each box. If we right-align, it points to the top-right
                // corner.
                match ctx.space.alignment {
                    Alignment::Left => space.padding.left,
                    Alignment::Right => space.dimensions.x - space.padding.right,
                },
                space.padding.top,
            ),
        }
    }

    /// Get a reference to this layouter's context.
    pub fn ctx(&self) -> &StackContext {
        &self.ctx
    }

    /// Add a sublayout to the bottom.
    pub fn add(&mut self, layout: Layout) -> LayoutResult<()> {
        let new_dimensions = Size2D {
            x: crate::size::max(self.dimensions.x, layout.dimensions.x),
            y: self.dimensions.y + layout.dimensions.y,
        };

        if self.overflows(new_dimensions) {
            return Err(LayoutError::NotEnoughSpace);
        }

        // Determine where to put the box. When we right-align it, we want the
        // cursor to point to the top-right corner of the box. Therefore, the
        // position has to be moved to the left by the width of the box.
        let position = match self.ctx.space.alignment {
            Alignment::Left => self.cursor,
            Alignment::Right => self.cursor - Size2D::with_x(layout.dimensions.x),
        };

        self.cursor.y += layout.dimensions.y;
        self.dimensions = new_dimensions;

        self.actions.add_layout(position, layout);

        Ok(())
    }

    /// Add multiple sublayouts from a multi-layout.
    pub fn add_many(&mut self, layouts: MultiLayout) -> LayoutResult<()> {
        for layout in layouts {
            self.add(layout)?;
        }
        Ok(())
    }

    /// Add vertical space after the last layout.
    pub fn add_space(&mut self, space: Size) -> LayoutResult<()> {
        if self.overflows(self.dimensions + Size2D::with_y(space)) {
            return Err(LayoutError::NotEnoughSpace);
        }

        self.cursor.y += space;
        self.dimensions.y += space;

        Ok(())
    }

    /// Finish the layouting.
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

    /// The remaining space for new layouts.
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

    fn overflows(&self, dimensions: Size2D) -> bool {
        !self.usable.fits(dimensions)
    }
}
