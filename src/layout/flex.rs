use super::*;

/// Layouts boxes flex-like.
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

    stack: StackLayouter,
    usable_width: Size,
    run: FlexRun,
    cached_glue: Option<Size2D>,
}

/// The context for flex layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Debug, Copy, Clone)]
pub struct FlexContext {
    /// The spacing between two lines of boxes.
    pub flex_spacing: Size,
    pub alignment: Alignment,
    pub space: LayoutSpace,
    pub followup_spaces: Option<LayoutSpace>,
    pub shrink_to_fit: bool,
}

macro_rules! reuse {
    ($ctx:expr, $flex_spacing:expr) => {
        FlexContext {
            flex_spacing: $flex_spacing,
            alignment: $ctx.alignment,
            space: $ctx.space,
            followup_spaces: $ctx.followup_spaces,
            shrink_to_fit: $ctx.shrink_to_fit,
        }
    };
}

impl FlexContext {
    /// Create a flex context from a generic layout context.
    pub fn from_layout_ctx(ctx: LayoutContext, flex_spacing: Size) -> FlexContext {
        reuse!(ctx, flex_spacing)
    }

    /// Create a flex context from a stack context.
    pub fn from_stack_ctx(ctx: StackContext, flex_spacing: Size) -> FlexContext {
        reuse!(ctx, flex_spacing)
    }
}

enum FlexUnit {
    /// A content unit to be arranged flexibly.
    Boxed(Layout),
    /// A unit which acts as glue between two [`FlexUnit::Boxed`] units and
    /// is only present if there was no flow break in between the two
    /// surrounding boxes.
    Glue(Size2D),
}

struct FlexRun {
    content: Vec<(Size, Layout)>,
    size: Size2D,
}

impl FlexLayouter {
    /// Create a new flex layouter.
    pub fn new(ctx: FlexContext) -> FlexLayouter {
        FlexLayouter {
            ctx,
            units: vec![],

            stack: StackLayouter::new(StackContext::from_flex_ctx(ctx)),

            usable_width: ctx.space.usable().x,
            run: FlexRun {
                content: vec![],
                size: Size2D::zero()
            },
            cached_glue: None,
        }
    }

    /// This layouter's context.
    pub fn ctx(&self) -> FlexContext {
        self.ctx
    }

    /// Add a sublayout.
    pub fn add(&mut self, layout: Layout) {
        self.units.push(FlexUnit::Boxed(layout));
    }

    /// Add a glue box which can be replaced by a line break.
    pub fn add_glue(&mut self, glue: Size2D) {
        self.units.push(FlexUnit::Glue(glue));
    }

    /// Compute the justified layout.
    ///
    /// The layouter is not consumed by this to prevent ownership problems
    /// with borrowed layouters. The state of the layouter is not reset.
    /// Therefore, it should not be further used after calling `finish`.
    pub fn finish(&mut self) -> LayoutResult<MultiLayout> {
        // Move the units out of the layout because otherwise, we run into
        // ownership problems.
        let units = std::mem::replace(&mut self.units, vec![]);
        for unit in units {
            match unit {
                FlexUnit::Boxed(boxed) => self.layout_box(boxed)?,
                FlexUnit::Glue(glue) => self.layout_glue(glue),
            }
        }

        // Finish the last flex run.
        self.finish_run()?;

        self.stack.finish()
    }

    /// Layout a content box into the current flex run or start a new run if
    /// it does not fit.
    fn layout_box(&mut self, boxed: Layout) -> LayoutResult<()> {
        let glue_width = self.cached_glue.unwrap_or(Size2D::zero()).x;
        let new_line_width = self.run.size.x + glue_width + boxed.dimensions.x;

        if self.overflows_line(new_line_width) {
            self.cached_glue = None;

            // If the box does not even fit on its own line, then we try
            // it in the next space, or we have to give up if there is none.
            if self.overflows_line(boxed.dimensions.x) {
                if self.ctx.followup_spaces.is_some() {
                    self.stack.finish_layout(true)?;
                    return self.layout_box(boxed);
                } else {
                    return Err(LayoutError::NotEnoughSpace("cannot fit box into flex run"));
                }
            }

            self.finish_run()?;
        } else {
            // Only add the glue if we did not move to a new line.
            self.flush_glue();
        }

        let dimensions = boxed.dimensions;
        self.run.content.push((self.run.size.x, boxed));

        self.grow_run(dimensions);

        Ok(())
    }

    fn layout_glue(&mut self, glue: Size2D) {
        self.flush_glue();
        self.cached_glue = Some(glue);
    }

    fn flush_glue(&mut self) {
        if let Some(glue) = self.cached_glue.take() {
            let new_line_width = self.run.size.x + glue.x;
            if !self.overflows_line(new_line_width) {
                self.grow_run(glue);
            }
        }
    }

    fn grow_run(&mut self, dimensions: Size2D) {
        self.run.size.x += dimensions.x;
        self.run.size.y = crate::size::max(self.run.size.y, dimensions.y);
    }

    fn finish_run(&mut self) -> LayoutResult<()> {
        self.run.size.y += self.ctx.flex_spacing;

        let mut actions = LayoutActionList::new();
        for (x, layout) in self.run.content.drain(..) {
            let position = Size2D::with_x(x);
            actions.add_layout(position, layout);
        }

        self.stack.add(Layout {
            dimensions: self.run.size,
            actions: actions.into_vec(),
            debug_render: false,
        })?;

        self.run.size = Size2D::zero();

        Ok(())
    }

    /// Whether this layouter contains any items.
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    fn overflows_line(&self, line: Size) -> bool {
        line > self.usable_width
    }
}
