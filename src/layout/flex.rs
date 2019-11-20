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
    space: Option<Size>,
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
    pub expand: bool,
    pub flex_spacing: Size,
}

impl FlexLayouter {
    /// Create a new flex layouter.
    pub fn new(ctx: FlexContext) -> FlexLayouter {
        let stack = StackLayouter::new(StackContext {
            spaces: ctx.spaces,
            axes: ctx.axes,
            expand: ctx.expand,
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

    pub fn add_secondary_space(&mut self, space: Size, soft: bool) -> LayoutResult<()> {
        if !self.run_is_empty() {
            self.finish_run()?;
        }
        Ok(self.stack.add_space(space, soft))
    }

    pub fn set_axes(&mut self, axes: LayoutAxes) {
        self.units.push(FlexUnit::SetAxes(axes));
    }

    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
        if replace_empty && self.run_is_empty() && self.stack.space_is_empty() {
            self.stack.set_spaces(spaces, true);
            self.start_run();
        } else {
            self.stack.set_spaces(spaces, false);
        }
    }

    pub fn remaining(&self) -> LayoutResult<(LayoutSpaces, Option<LayoutSpaces>)> {
        if self.run_is_empty() {
            Ok((self.stack.remaining(), None))
        } else {
            let mut future = self.clone();
            let remaining_run = future.finish_run()?;

            let stack_spaces = future.stack.remaining();
            let mut flex_spaces = stack_spaces.clone();
            flex_spaces[0].dimensions.x = remaining_run.x;
            flex_spaces[0].dimensions.y += remaining_run.y;

            Ok((flex_spaces, Some(stack_spaces)))
        }

    }

    pub fn run_is_empty(&self) -> bool {
        !self.units.iter().any(|unit| matches!(unit, FlexUnit::Boxed(_)))
    }

    pub fn run_last_is_space(&self) -> bool {
        matches!(self.units.last(), Some(FlexUnit::Space(_, _)))
    }

    pub fn finish(mut self) -> LayoutResult<MultiLayout> {
        self.finish_space(false)?;
        Ok(self.stack.finish())
    }

    pub fn finish_space(&mut self, hard: bool) -> LayoutResult<()> {
        if !self.run_is_empty() {
            self.finish_run()?;
        }
        Ok(self.stack.finish_space(hard))
    }

    pub fn finish_run(&mut self) -> LayoutResult<Size2D> {
        let units = std::mem::replace(&mut self.units, vec![]);
        for unit in units {
            match unit {
                FlexUnit::Boxed(boxed) => self.layout_box(boxed)?,
                FlexUnit::Space(space, soft) => self.layout_space(space, soft),
                FlexUnit::SetAxes(axes) => self.layout_set_axes(axes),
                FlexUnit::Break => self.layout_break(),
            }
        }

        self.finish_line()
    }

    fn finish_line(&mut self) -> LayoutResult<Size2D> {
        self.finish_partial_line();

        self.stack.add(Layout {
            dimensions: self.axes.specialize(self.line.combined_dimensions),
            actions: self.line.actions.into_vec(),
            debug_render: false,
        })?;

        let remaining = self.axes.specialize(Size2D {
            x: self.line.usable - self.line.combined_dimensions.x,
            y: self.line.combined_dimensions.y,
        });

        self.line = FlexLine::new(self.stack.primary_usable());

        Ok(remaining)
    }

    fn finish_partial_line(&mut self) {
        let part = self.line.part;

        let factor = self.axes.primary.axis.factor();
        let anchor =
            self.axes.primary.anchor(self.line.usable)
            - self.axes.primary.anchor(part.dimensions.x);

        for (offset, layout) in part.content {
            let pos = self.axes.specialize(Size2D::with_x(anchor + factor * offset));
            self.line.actions.add_layout(pos, layout);
        }

        self.line.combined_dimensions.x.max_eq(part.dimensions.x);
        self.line.part = PartialLine::new(self.line.usable - part.dimensions.x);
    }

    fn start_run(&mut self) {
        let usable = self.stack.primary_usable();
        self.line = FlexLine::new(usable);
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

    fn layout_space(&mut self, space: Size, soft: bool) {
        if let Some(space) = self.space.take() {
            if self.run.size.x > Size::zero() && self.run.size.x + space <= self.usable {
                self.run.size.x += space;
            }
        }
    }

    fn layout_set_axes(&mut self, axes: LayoutAxes) {
        if axes.primary != self.axes.primary {
            self.finish_partial_line();

            // self.usable = match axes.primary.alignment {
            //     Alignment::Origin =>
            //         if self.max_extent == Size::zero() {
            //             self.total_usable
            //         } else {
            //             Size::zero()
            //         },
            //     Alignment::Center => crate::size::max(
            //         self.total_usable - 2 * self.max_extent,
            //         Size::zero()
            //     ),
            //     Alignment::End => self.total_usable - self.max_extent,
            // };
        }

        if axes.secondary != self.axes.secondary {
            self.stack.set_axes(axes);
        }

        self.axes = axes;
    }

    fn layout_break(&mut self) {

    }

    fn size_left(&self) -> Size {
        let space = self.space.unwrap_or(Size::zero());
        self.usable - (self.run.size.x + space)
    }
}
