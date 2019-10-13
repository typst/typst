use super::*;

/// Finishes a flex layout by justifying the positions of the individual boxes.
#[derive(Debug)]
pub struct FlexLayouter {
    ctx: FlexContext,
    units: Vec<FlexUnit>,

    actions: LayoutActionList,
    dimensions: Size2D,
    usable: Size2D,
    cursor: Size2D,

    line_content: Vec<(Size2D, Layout)>,
    line_metrics: Size2D,
    last_glue: Option<Layout>,
}

/// The context for flex layouting.
#[derive(Debug, Copy, Clone)]
pub struct FlexContext {
    /// The space to layout the boxes in.
    pub space: LayoutSpace,
    /// The flex spacing between two lines of boxes.
    pub flex_spacing: Size,
}

/// A unit in a flex layout.
#[derive(Debug, Clone)]
enum FlexUnit {
    /// A content unit to be arranged flexibly.
    Boxed(Layout),
    /// A unit which acts as glue between two [`FlexUnit::Boxed`] units and
    /// is only present if there was no flow break in between the two
    /// surrounding boxes.
    Glue(Layout),
}

impl FlexLayouter {
    /// Create a new flex layouter.
    pub fn new(ctx: FlexContext) -> FlexLayouter {
        FlexLayouter {
            ctx,
            units: vec![],

            actions: LayoutActionList::new(),
            dimensions: match ctx.space.alignment {
                Alignment::Left => Size2D::zero(),
                Alignment::Right => Size2D::with_x(ctx.space.usable().x),
            },
            usable: ctx.space.usable(),
            cursor: Size2D::new(ctx.space.padding.left, ctx.space.padding.top),

            line_content: vec![],
            line_metrics: Size2D::zero(),
            last_glue: None,
        }
    }

    /// Get a reference to this layouter's context.
    pub fn ctx(&self) -> &FlexContext {
        &self.ctx
    }

    /// Add a sublayout.
    pub fn add(&mut self, layout: Layout) {
        self.units.push(FlexUnit::Boxed(layout));
    }

    /// Add a glue layout which can be replaced by a line break.
    pub fn add_glue(&mut self, glue: Layout) {
        self.units.push(FlexUnit::Glue(glue));
    }

    /// Whether this layouter contains any items.
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// Compute the justified layout.
    pub fn finish(mut self) -> LayoutResult<Layout> {
        // Move the units out of the layout.
        let units = self.units;
        self.units = vec![];

        // Arrange the units.
        for unit in units {
            match unit {
                FlexUnit::Boxed(boxed) => self.boxed(boxed)?,
                FlexUnit::Glue(glue) => self.glue(glue),
            }
        }

        // Flush everything to get the correct dimensions.
        self.newline();

        Ok(Layout {
            dimensions: if self.ctx.space.shrink_to_fit {
                self.dimensions.padded(self.ctx.space.padding)
            } else {
                self.ctx.space.dimensions
            },
            actions: self.actions.into_vec(),
            debug_render: true,
        })
    }

    /// Layout the box.
    fn boxed(&mut self, boxed: Layout) -> LayoutResult<()> {
        let last_glue_x = self
            .last_glue
            .as_ref()
            .map(|g| g.dimensions.x)
            .unwrap_or(Size::zero());

        // Move to the next line if necessary.
        if self.line_metrics.x + boxed.dimensions.x + last_glue_x > self.usable.x {
            // If it still does not fit, we stand no chance.
            if boxed.dimensions.x > self.usable.x {
                return Err(LayoutError::NotEnoughSpace);
            }

            self.newline();
        } else if let Some(glue) = self.last_glue.take() {
            self.append(glue);
        }

        self.append(boxed);

        Ok(())
    }

    /// Layout the glue.
    fn glue(&mut self, glue: Layout) {
        if let Some(glue) = self.last_glue.take() {
            self.append(glue);
        }
        self.last_glue = Some(glue);
    }

    /// Append a box to the layout without checking anything.
    fn append(&mut self, layout: Layout) {
        let dim = layout.dimensions;
        self.line_content.push((self.cursor, layout));

        self.line_metrics.x += dim.x;
        self.line_metrics.y = crate::size::max(self.line_metrics.y, dim.y);
        self.cursor.x += dim.x;
    }

    /// Move to the next line.
    fn newline(&mut self) {
        // Move all actions into this layout and translate absolute positions.
        let remaining_space = Size2D::with_x(self.ctx.space.usable().x - self.line_metrics.x);
        for (cursor, layout) in self.line_content.drain(..) {
            let position = match self.ctx.space.alignment {
                Alignment::Left => cursor,
                Alignment::Right => {
                    // Right align everything by shifting it right by the
                    // amount of space left to the right of the line.
                    cursor + remaining_space
                }
            };

            self.actions.add_box(position, layout);
        }

        // Stretch the dimensions to at least the line width.
        self.dimensions.x = crate::size::max(self.dimensions.x, self.line_metrics.x);

        // If we wrote a line previously add the inter-line spacing.
        if self.dimensions.y > Size::zero() {
            self.dimensions.y += self.ctx.flex_spacing;
        }

        self.dimensions.y += self.line_metrics.y;

        // Reset the cursor the left and move down by the line and the inter-line
        // spacing.
        self.cursor.x = self.ctx.space.padding.left;
        self.cursor.y += self.line_metrics.y + self.ctx.flex_spacing;

        // Reset the current line metrics.
        self.line_metrics = Size2D::zero();
    }
}
