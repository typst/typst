use super::*;

/// Flex-layouting of boxes.
///
/// The boxes are arranged in "lines", each line having the height of its
/// biggest box. When a box does not fit on a line anymore horizontally,
/// a new line is started.
///
/// The flex layouter does not actually compute anything until the `finish`
/// method is called. The reason for this is the flex layouter will have
/// the capability to justify its layouts, later. To find a good justification
/// it needs total information about the contents.
///
/// There are two different kinds units that can be added to a flex run:
/// Normal layouts and _glue_. _Glue_ layouts are only written if a normal
/// layout follows and a glue layout is omitted if the following layout
/// flows into a new line. A _glue_ layout is typically used for a space character
/// since it prevents a space from appearing in the beginning or end of a line.
/// However, it can be any layout.
pub struct FlexLayouter {
    ctx: FlexContext,
    units: Vec<FlexUnit>,

    actions: LayoutActionList,
    usable: Size2D,
    dimensions: Size2D,
    cursor: Size2D,

    run: FlexRun,
    next_glue: Option<Layout>,
}

/// The context for flex layouting.
#[derive(Debug, Copy, Clone)]
pub struct FlexContext {
    /// The space to layout the boxes in.
    pub space: LayoutSpace,
    /// The spacing between two lines of boxes.
    pub flex_spacing: Size,
}

enum FlexUnit {
    /// A content unit to be arranged flexibly.
    Boxed(Layout),
    /// A unit which acts as glue between two [`FlexUnit::Boxed`] units and
    /// is only present if there was no flow break in between the two
    /// surrounding boxes.
    Glue(Layout),
}

struct FlexRun {
    content: Vec<(Size2D, Layout)>,
    size: Size2D,
}

impl FlexLayouter {
    /// Create a new flex layouter.
    pub fn new(ctx: FlexContext) -> FlexLayouter {
        FlexLayouter {
            ctx,
            units: vec![],

            actions: LayoutActionList::new(),
            usable: ctx.space.usable(),
            dimensions: match ctx.space.alignment {
                Alignment::Left => Size2D::zero(),
                Alignment::Right => Size2D::with_x(ctx.space.usable().x),
            },

            cursor: Size2D::new(ctx.space.padding.left, ctx.space.padding.top),

            run: FlexRun::new(),
            next_glue: None,
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

    /// Compute the justified layout.
    pub fn finish(mut self) -> LayoutResult<Layout> {
        // Move the units out of the layout because otherwise, we run into
        // ownership problems.
        let units = self.units;
        self.units = Vec::new();

        for unit in units {
            match unit {
                FlexUnit::Boxed(boxed) => self.layout_box(boxed)?,
                FlexUnit::Glue(glue) => self.layout_glue(glue),
            }
        }

        // Finish the last flex run.
        self.finish_flex_run();

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

    /// Whether this layouter contains any items.
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    fn layout_box(&mut self, boxed: Layout) -> LayoutResult<()> {
        let next_glue_width = self
            .next_glue
            .as_ref()
            .map(|g| g.dimensions.x)
            .unwrap_or(Size::zero());

        let new_line_width = self.run.size.x + next_glue_width + boxed.dimensions.x;

        if self.overflows(new_line_width) {
            // If the box does not even fit on its own line, then
            // we can't do anything.
            if self.overflows(boxed.dimensions.x) {
                return Err(LayoutError::NotEnoughSpace);
            }

            self.finish_flex_run();
        } else {
            // Only add the glue if we did not move to a new line.
            self.flush_glue();
        }

        self.add_to_flex_run(boxed);

        Ok(())
    }

    fn layout_glue(&mut self, glue: Layout) {
        self.flush_glue();
        self.next_glue = Some(glue);
    }

    fn flush_glue(&mut self) {
        if let Some(glue) = self.next_glue.take() {
            self.add_to_flex_run(glue);
        }
    }

    fn add_to_flex_run(&mut self, layout: Layout) {
        let position = self.cursor;

        self.cursor.x += layout.dimensions.x;
        self.run.size.x += layout.dimensions.x;
        self.run.size.y = crate::size::max(self.run.size.y, layout.dimensions.y);

        self.run.content.push((position, layout));
    }

    fn finish_flex_run(&mut self) {
        // Add all layouts from the current flex run at the correct positions.
        match self.ctx.space.alignment {
            Alignment::Left => {
                for (position, layout) in self.run.content.drain(..) {
                    self.actions.add_layout(position, layout);
                }
            }

            Alignment::Right => {
                let extra_space = Size2D::with_x(self.usable.x -  self.run.size.x);
                for (position, layout) in self.run.content.drain(..) {
                    self.actions.add_layout(position + extra_space, layout);
                }
            }
        }

        self.dimensions.x = crate::size::max(self.dimensions.x,  self.run.size.x);
        self.dimensions.y += self.ctx.flex_spacing;
        self.dimensions.y +=  self.run.size.y;

        self.cursor.x = self.ctx.space.padding.left;
        self.cursor.y += self.run.size.y + self.ctx.flex_spacing;
        self.run.size = Size2D::zero();
    }

    fn overflows(&self, line: Size) -> bool {
        line > self.usable.x
    }
}

impl FlexRun {
    fn new() -> FlexRun {
        FlexRun {
            content: vec![],
            size: Size2D::zero()
        }
    }
}
