use super::*;

/// Layouts boxes stack-like.
///
/// The boxes are arranged vertically, each layout gettings it's own "line".
pub struct StackLayouter {
    ctx: StackContext,
    layouts: MultiLayout,
    actions: LayoutActionList,

    space: LayoutSpace,
    usable: Size2D,
    dimensions: Size2D,
    cursor: Size2D,
    in_extra_space: bool,
    started: bool,
}

/// The context for stack layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Debug, Copy, Clone)]
pub struct StackContext {
    pub alignment: Alignment,
    pub space: LayoutSpace,
    pub followup_spaces: Option<LayoutSpace>,
    pub shrink_to_fit: bool,
}

macro_rules! reuse {
    ($ctx:expr) => {
        StackContext {
            alignment: $ctx.alignment,
            space: $ctx.space,
            followup_spaces: $ctx.followup_spaces,
            shrink_to_fit: $ctx.shrink_to_fit
        }
    };
}

impl StackContext {
    /// Create a stack context from a generic layout context.
    pub fn from_layout_ctx(ctx: LayoutContext) -> StackContext {
        reuse!(ctx)
    }

    /// Create a stack context from a flex context.
    pub fn from_flex_ctx(ctx: FlexContext) -> StackContext {
        reuse!(ctx)
    }
}

impl StackLayouter {
    /// Create a new stack layouter.
    pub fn new(ctx: StackContext) -> StackLayouter {
        StackLayouter {
            ctx,
            layouts: MultiLayout::new(),
            actions: LayoutActionList::new(),

            space: ctx.space,
            usable: ctx.space.usable(),
            dimensions: start_dimensions(ctx.alignment, ctx.space),
            cursor: start_cursor(ctx.alignment, ctx.space),
            in_extra_space: false,
            started: true,
        }
    }

    /// This layouter's context.
    pub fn ctx(&self) -> StackContext {
        self.ctx
    }

    /// Add a sublayout to the bottom.
    pub fn add(&mut self, layout: Layout) -> LayoutResult<()> {
        if !self.started {
            self.start_new_space()?;
        }

        let new_dimensions = Size2D {
            x: crate::size::max(self.dimensions.x, layout.dimensions.x),
            y: self.dimensions.y + layout.dimensions.y,
        };

        if self.overflows(new_dimensions) {
            if self.ctx.followup_spaces.is_some() &&
                !(self.in_extra_space && self.overflows(layout.dimensions))
            {
                self.finish_layout(true)?;
                return self.add(layout);
            } else {
                return Err(LayoutError::NotEnoughSpace("cannot fit box into stack"));
            }
        }

        // Determine where to put the box. When we right-align it, we want the
        // cursor to point to the top-right corner of the box. Therefore, the
        // position has to be moved to the left by the width of the box.
        let position = match self.ctx.alignment {
            Alignment::Left => self.cursor,
            Alignment::Right => self.cursor - Size2D::with_x(layout.dimensions.x),
            Alignment::Center => self.cursor - Size2D::with_x(layout.dimensions.x / 2),
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
        if !self.started {
            self.start_new_space()?;
        }

        let new_dimensions = self.dimensions + Size2D::with_y(space);

        if self.overflows(new_dimensions) {
            if self.ctx.followup_spaces.is_some() {
                self.finish_layout(false)?;
            } else {
                return Err(LayoutError::NotEnoughSpace("cannot fit space into stack"));
            }
        } else {
            self.cursor.y += space;
            self.dimensions.y += space;
        }

        Ok(())
    }

    /// Finish the layouting.
    ///
    /// The layouter is not consumed by this to prevent ownership problems.
    /// It should not be used further.
    pub fn finish(&mut self) -> LayoutResult<MultiLayout> {
        if self.started {
            self.finish_layout(false)?;
        }
        Ok(std::mem::replace(&mut self.layouts, MultiLayout::new()))
    }

    /// Finish the current layout and start a new one in an extra space
    /// (if there is an extra space).
    ///
    /// If `start_new_empty` is true, a new empty layout will be started. Otherwise,
    /// the new layout only emerges when new content is added.
    pub fn finish_layout(&mut self, start_new_empty: bool) -> LayoutResult<()> {
        let actions = std::mem::replace(&mut self.actions, LayoutActionList::new());
        self.layouts.add(Layout {
            dimensions: if self.ctx.shrink_to_fit {
                self.dimensions.padded(self.space.padding)
            } else {
                self.space.dimensions
            },
            actions: actions.into_vec(),
            debug_render: true,
        });

        self.started = false;

        if start_new_empty {
            self.start_new_space()?;
        }

        Ok(())
    }

    pub fn start_new_space(&mut self) -> LayoutResult<()> {
        if let Some(space) = self.ctx.followup_spaces {
            self.started = true;
            self.space = space;
            self.usable = space.usable();
            self.dimensions = start_dimensions(self.ctx.alignment, space);
            self.cursor = start_cursor(self.ctx.alignment, space);
            self.in_extra_space = true;
            Ok(())
        } else {
            Err(LayoutError::NotEnoughSpace("no extra space to start"))
        }
    }

    /// The remaining space for new layouts.
    pub fn remaining(&self) -> Size2D {
        Size2D {
            x: self.usable.x,
            y: self.usable.y - self.dimensions.y,
        }
    }

    /// Whether the active space of this layouter contains no content.
    pub fn current_space_is_empty(&self) -> bool {
        !self.started || self.actions.is_empty()
    }

    fn overflows(&self, dimensions: Size2D) -> bool {
        !self.usable.fits(dimensions)
    }
}

fn start_dimensions(alignment: Alignment, space: LayoutSpace) -> Size2D {
    match alignment {
        Alignment::Left => Size2D::zero(),
        Alignment::Right | Alignment::Center => Size2D::with_x(space.usable().x),
    }
}

fn start_cursor(alignment: Alignment, space: LayoutSpace) -> Size2D {
    Size2D {
        // If left-align, the cursor points to the top-left corner of
        // each box. If we right-align, it points to the top-right
        // corner.
        x: match alignment {
            Alignment::Left => space.padding.left,
            Alignment::Right => space.dimensions.x - space.padding.right,
            Alignment::Center => space.padding.left + (space.usable().x / 2),
        },
        y: space.padding.top,
    }
}
