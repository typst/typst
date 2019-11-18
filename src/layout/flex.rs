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
#[derive(Debug, Clone)]
pub struct FlexLayouter {
    axes: LayoutAxes,
    flex_spacing: Size,

    stack: StackLayouter,
    units: Vec<FlexUnit>,

    total_usable: Size,
    merged_actions: LayoutActionList,
    merged_dimensions: Size2D,
    max_extent: Size,

    usable: Size,
    run: FlexRun,
    space: Option<Size>,

    last_run_remaining: Size2D,
}

/// The context for flex layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Debug, Clone)]
pub struct FlexContext {
    pub spaces: LayoutSpaces,
    pub axes: LayoutAxes,
    pub shrink_to_fit: bool,
    /// The spacing between two lines of boxes.
    pub flex_spacing: Size,
}

#[derive(Debug, Clone)]
enum FlexUnit {
    /// A content unit to be arranged flexibly.
    Boxed(Layout),
    /// Space between two box units which is only present if there
    /// was no flow break in between the two surrounding units.
    Space(Size),
    /// A forced break of the current flex run.
    Break,
    SetAxes(LayoutAxes),
}

#[derive(Debug, Clone)]
struct FlexRun {
    content: Vec<(Size, Layout)>,
    size: Size2D,
}

impl FlexLayouter {
    /// Create a new flex layouter.
    pub fn new(ctx: FlexContext) -> FlexLayouter {
        let stack = StackLayouter::new(StackContext {
            spaces: ctx.spaces,
            axes: ctx.axes,
            shrink_to_fit: ctx.shrink_to_fit,
        });

        let usable = stack.primary_usable();
        FlexLayouter {
            axes: ctx.axes,
            flex_spacing: ctx.flex_spacing,

            units: vec![],
            stack,

            total_usable: usable,
            merged_actions: LayoutActionList::new(),
            merged_dimensions: Size2D::zero(),
            max_extent: Size::zero(),

            usable,
            run: FlexRun { content: vec![], size: Size2D::zero() },
            space: None,

            last_run_remaining: Size2D::zero(),
        }
    }

    /// Add a sublayout.
    pub fn add(&mut self, layout: Layout) {
        self.units.push(FlexUnit::Boxed(layout));
    }

    /// Add multiple sublayouts from a multi-layout.
    pub fn add_multiple(&mut self, layouts: MultiLayout) {
        for layout in layouts {
            self.add(layout);
        }
    }

    /// Add a forced run break.
    pub fn add_run_break(&mut self) {
        self.units.push(FlexUnit::Break);
    }

    /// Add a space box which can be replaced by a run break.
    pub fn add_primary_space(&mut self, space: Size) {
        self.units.push(FlexUnit::Space(space));
    }

    pub fn add_secondary_space(&mut self, space: Size) -> LayoutResult<()> {
        self.finish_box()?;
        self.stack.add_space(space);
        Ok(())
    }

    /// Update the axes in use by this flex layouter.
    pub fn set_axes(&mut self, axes: LayoutAxes) {
        self.units.push(FlexUnit::SetAxes(axes));
    }

    /// Update the followup space to be used by this flex layouter.
    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
        if replace_empty && self.box_is_empty() && self.stack.space_is_empty() {
            self.stack.set_spaces(spaces, true);
            self.total_usable = self.stack.primary_usable();
            self.usable = self.total_usable;
            self.space = None;
        } else {
            self.stack.set_spaces(spaces, false);
        }
    }

    /// Compute the justified layout.
    ///
    /// The layouter is not consumed by this to prevent ownership problems
    /// with borrowed layouters. The state of the layouter is not reset.
    /// Therefore, it should not be further used after calling `finish`.
    pub fn finish(mut self) -> LayoutResult<MultiLayout> {
        self.finish_box()?;
        Ok(self.stack.finish())
    }

    pub fn finish_layout(&mut self, hard: bool) -> LayoutResult<()> {
        self.finish_box()?;
        self.stack.finish_layout(hard);
        Ok(())
    }

    pub fn finish_box(&mut self) -> LayoutResult<()> {
        if self.box_is_empty() {
            return Ok(());
        }

        // Move the units out of the layout because otherwise, we run into
        // ownership problems.
        let units = std::mem::replace(&mut self.units, vec![]);
        for unit in units {
            match unit {
                FlexUnit::Boxed(boxed) => self.layout_box(boxed)?,
                FlexUnit::Space(space) => {
                    self.layout_space();
                    self.space = Some(space);
                }

                FlexUnit::Break => {
                    self.space = None;
                    self.finish_run()?;
                },

                FlexUnit::SetAxes(axes) => self.layout_set_axes(axes),
            }
        }

        // Finish the last flex run.
        self.finish_run()?;

        Ok(())
    }

    /// Layout a content box into the current flex run or start a new run if
    /// it does not fit.
    fn layout_box(&mut self, boxed: Layout) -> LayoutResult<()> {
        let size = self.axes.generalize(boxed.dimensions);

        if size.x > self.size_left() {
            self.space = None;
            self.finish_run()?;

            while size.x > self.usable {
                if self.stack.in_last_space() {
                    Err(LayoutError::NotEnoughSpace("cannot fix box into flex run"))?;
                }

                self.stack.finish_layout(true);
                self.total_usable = self.stack.primary_usable();
                self.usable = self.total_usable;
            }
        }

        self.layout_space();

        let offset = self.run.size.x;
        self.run.content.push((offset, boxed));

        self.run.size.x += size.x;
        self.run.size.y = crate::size::max(self.run.size.y, size.y);

        Ok(())
    }

    fn layout_space(&mut self) {
        if let Some(space) = self.space.take() {
            if self.run.size.x > Size::zero() && self.run.size.x + space <= self.usable {
                self.run.size.x += space;
            }
        }
    }

    fn layout_set_axes(&mut self, axes: LayoutAxes) {
        if axes.primary != self.axes.primary {
            self.finish_aligned_run();

            self.usable = match axes.primary.alignment {
                Alignment::Origin =>
                    if self.max_extent == Size::zero() {
                        self.total_usable
                    } else {
                        Size::zero()
                    },
                Alignment::Center => crate::size::max(
                    self.total_usable - 2 * self.max_extent,
                    Size::zero()
                ),
                Alignment::End => self.total_usable - self.max_extent,
            };
        }

        if axes.secondary != self.axes.secondary {
            self.stack.set_axes(axes);
        }

        self.axes = axes;
    }

    /// Finish the current flex run.
    fn finish_run(&mut self) -> LayoutResult<()> {
        self.finish_aligned_run();

        if self.merged_dimensions.y == Size::zero() {
            return Ok(());
        }

        let actions = std::mem::replace(&mut self.merged_actions, LayoutActionList::new());
        self.stack.add(Layout {
            dimensions: self.axes.specialize(self.merged_dimensions),
            actions: actions.into_vec(),
            debug_render: false,
        })?;

        self.merged_dimensions = Size2D::zero();
        self.max_extent = Size::zero();
        self.usable = self.total_usable;

        Ok(())
    }

    fn finish_aligned_run(&mut self) {
        if self.run.content.is_empty() {
            return;
        }

        let factor = if self.axes.primary.axis.is_positive() { 1 } else { -1 };
        let anchor = self.axes.primary.anchor(self.total_usable)
                     - self.axes.primary.anchor(self.run.size.x);

        self.max_extent = crate::size::max(self.max_extent, anchor + factor * self.run.size.x);

        for (offset, layout) in self.run.content.drain(..) {
            let general_position = Size2D::with_x(anchor + factor * offset);
            let position = self.axes.specialize(general_position);

            self.merged_actions.add_layout(position, layout);
        }

        self.merged_dimensions.x = match self.axes.primary.alignment {
            Alignment::Origin => self.run.size.x,
            Alignment::Center | Alignment::End => self.total_usable,
        };

        self.merged_dimensions.y = crate::size::max(
            self.merged_dimensions.y,
            self.run.size.y + self.flex_spacing,
        );

        self.last_run_remaining = Size2D::new(self.size_left(), self.merged_dimensions.y);
        self.run.size = Size2D::zero();
    }

    pub fn remaining(&self) -> LayoutResult<(LayoutSpaces, LayoutSpaces)> {
        let mut future = self.clone();
        future.finish_box()?;

        let stack_spaces = future.stack.remaining();

        let mut flex_spaces = stack_spaces.clone();
        flex_spaces[0].dimensions.x = future.last_run_remaining.x;
        flex_spaces[0].dimensions.y += future.last_run_remaining.y;

        Ok((flex_spaces, stack_spaces))
    }

    pub fn box_is_empty(&self) -> bool {
        !self.units.iter().any(|unit| matches!(unit, FlexUnit::Boxed(_)))
    }

    pub fn last_is_space(&self) -> bool {
        matches!(self.units.last(), Some(FlexUnit::Space(_)))
    }

    fn size_left(&self) -> Size {
        let space = self.space.unwrap_or(Size::zero());
        self.usable - (self.run.size.x + space)
    }
}
