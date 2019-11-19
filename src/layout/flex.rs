use super::*;

#[derive(Debug, Clone)]
pub struct FlexLayouter {
    stack: StackLayouter,

    axes: LayoutAxes,
    flex_spacing: Size,

    units: Vec<FlexUnit>,
    line: FlexLine,
}

#[derive(Debug, Clone)]
enum FlexUnit {
    Boxed(Layout),
    Space(Size, bool),
    SetAxes(LayoutAxes),
    Break,
}

#[derive(Debug, Clone)]
struct FlexLine {
    usable: Size,
    actions: LayoutActionList,
    combined_dimensions: Size2D,
    part: PartialLine,
}

impl FlexLine {
    fn new(usable: Size) -> FlexLine {
        FlexLine {
            usable,
            actions: LayoutActionList::new(),
            combined_dimensions: Size2D::zero(),
            part: PartialLine::new(usable),
        }
    }
}

#[derive(Debug, Clone)]
struct PartialLine {
    usable: Size,
    content: Vec<(Size, Layout)>,
    dimensions: Size2D,
    space: Option<(Size, bool)>,
}

impl PartialLine {
    fn new(usable: Size) -> PartialLine {
        PartialLine {
            usable,
            content: vec![],
            dimensions: Size2D::zero(),
            space: None,
        }
    }
}

/// The context for flex layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Debug, Clone)]
pub struct FlexContext {
    pub spaces: LayoutSpaces,
    pub axes: LayoutAxes,
    pub shrink_to_fit: bool,
    pub flex_spacing: Size,
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
            stack,

            axes: ctx.axes,
            flex_spacing: ctx.flex_spacing,

            units: vec![],
            line: FlexLine::new(usable)
        }
    }

    pub fn add(&mut self, layout: Layout) {
        self.units.push(FlexUnit::Boxed(layout));
    }

    pub fn add_multiple(&mut self, layouts: MultiLayout) {
        for layout in layouts {
            self.add(layout);
        }
    }

    pub fn add_break(&mut self) {
        self.units.push(FlexUnit::Break);
    }

    pub fn add_primary_space(&mut self, space: Size, soft: bool) {
        self.units.push(FlexUnit::Space(space, soft));
    }

    pub fn add_secondary_space(&mut self, space: Size) -> LayoutResult<()> {
        self.finish_run()?;
        Ok(self.stack.add_space(space))
    }

    pub fn set_axes(&mut self, axes: LayoutAxes) {
        self.units.push(FlexUnit::SetAxes(axes));
    }

    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
        if replace_empty && self.run_is_empty() && self.stack.space_is_empty() {
            self.stack.set_spaces(spaces, true);
            self.start_run();

            // let usable = self.stack.primary_usable();
            // self.line = FlexLine::new(usable);

            // // self.total_usable = self.stack.primary_usable();
            // // self.usable = self.total_usable;
            // // self.space = None;
        } else {
            self.stack.set_spaces(spaces, false);
        }
    }

    pub fn remaining(&self) -> LayoutResult<(LayoutSpaces, LayoutSpaces)> {
        let mut future = self.clone();
        future.finish_run()?;

        let stack_spaces = future.stack.remaining();
        let mut flex_spaces = stack_spaces.clone();
        flex_spaces[0].dimensions.x = future.last_run_remaining.x;
        flex_spaces[0].dimensions.y += future.last_run_remaining.y;

        Ok((flex_spaces, stack_spaces))
    }

    pub fn run_is_empty(&self) -> bool {
        !self.units.iter().any(|unit| matches!(unit, FlexUnit::Boxed(_)))
    }

    pub fn run_last_is_space(&self) -> bool {
        matches!(self.units.last(), Some(FlexUnit::Space(_)))
    }

    pub fn finish(mut self) -> LayoutResult<MultiLayout> {
        self.finish_space(false)?;
        Ok(self.stack.finish())
    }

    pub fn finish_space(&mut self, hard: bool) -> LayoutResult<()> {
        self.finish_run()?;
        Ok(self.stack.finish_space(hard))
    }

    pub fn finish_run(&mut self) -> LayoutResult<()> {
        let units = std::mem::replace(&mut self.units, vec![]);
        for unit in units {
            match unit {
                FlexUnit::Boxed(boxed) => self.layout_box(boxed)?,
                FlexUnit::Space(space, soft) => {
                    self.layout_space();
                    self.space = Some(space);
                }

                FlexUnit::Break => {
                    self.space = None;
                    self.finish_line()?;
                },

                FlexUnit::SetAxes(axes) => self.layout_set_axes(axes),
            }
        }

        self.finish_line()?;

        Ok(())
    }

    fn layout_box(&mut self, boxed: Layout) -> LayoutResult<()> {
        let size = self.axes.generalize(boxed.dimensions);

        if size.x > self.size_left() {
            self.space = None;
            self.finish_line()?;

            while size.x > self.usable {
                if self.stack.space_is_last() {
                    Err(LayoutError::NotEnoughSpace("cannot fix box into flex run"))?;
                }

                self.finish_space(true);
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
            self.finish_partial_line();

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

    fn finish_line(&mut self) -> LayoutResult<()> {
        self.finish_partial_line();

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

    fn finish_partial_line(&mut self) {
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

    fn size_left(&self) -> Size {
        let space = self.space.unwrap_or(Size::zero());
        self.usable - (self.run.size.x + space)
    }
}
