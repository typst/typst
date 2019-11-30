use super::*;

#[derive(Debug, Clone)]
pub struct FlexLayouter {
    axes: LayoutAxes,
    flex_spacing: Size,
    stack: StackLayouter,

    units: Vec<FlexUnit>,
    line: FlexLine,
    part: PartialLine,
}

#[derive(Debug, Clone)]
enum FlexUnit {
    Boxed(Layout),
    Space(Size, SpaceKind),
    SetAxes(LayoutAxes),
    Break,
}

#[derive(Debug, Clone)]
struct FlexLine {
    usable: Size,
    actions: LayoutActionList,
    combined_dimensions: Size2D,
}

impl FlexLine {
    fn new(usable: Size) -> FlexLine {
        FlexLine {
            usable,
            actions: LayoutActionList::new(),
            combined_dimensions: Size2D::zero(),
        }
    }
}

#[derive(Debug, Clone)]
struct PartialLine {
    usable: Size,
    content: Vec<(Size, Layout)>,
    dimensions: Size2D,
    space: SpaceState,
}

impl PartialLine {
    fn new(usable: Size) -> PartialLine {
        PartialLine {
            usable,
            content: vec![],
            dimensions: Size2D::zero(),
            space: SpaceState::Forbidden,
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
            axes: ctx.axes,
            flex_spacing: ctx.flex_spacing,
            stack,

            units: vec![],
            line: FlexLine::new(usable),
            part: PartialLine::new(usable),
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

    pub fn add_primary_space(&mut self, space: Size, kind: SpaceKind) {
        self.units.push(FlexUnit::Space(space, kind))
    }

    pub fn add_secondary_space(&mut self, space: Size, kind: SpaceKind) -> LayoutResult<()> {
        if !self.run_is_empty() {
            self.finish_run()?;
        }
        Ok(self.stack.add_space(space, kind))
    }

    pub fn set_axes(&mut self, axes: LayoutAxes) {
        self.units.push(FlexUnit::SetAxes(axes));
    }

    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
        if replace_empty && self.run_is_empty() && self.stack.space_is_empty() {
            self.stack.set_spaces(spaces, true);
            self.start_line();
        } else {
            self.stack.set_spaces(spaces, false);
        }
    }

    pub fn remaining(&self) -> LayoutSpaces {
        self.stack.remaining()
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

        self.stack.finish_space(hard);
        Ok(self.start_line())
    }

    pub fn finish_run(&mut self) -> LayoutResult<Size2D> {
        let units = std::mem::replace(&mut self.units, vec![]);
        for unit in units {
            match unit {
                FlexUnit::Boxed(boxed) => self.layout_box(boxed)?,
                FlexUnit::Space(space, kind) => self.layout_space(space, kind),
                FlexUnit::SetAxes(axes) => self.layout_set_axes(axes),
                FlexUnit::Break => { self.finish_line()?; },
            }
        }

        self.finish_line()
    }

    fn finish_line(&mut self) -> LayoutResult<Size2D> {
        self.finish_partial_line();

        if self.axes.primary.needs_expansion() {
            self.line.combined_dimensions.x = self.line.usable;
        }

        self.stack.add(Layout {
            dimensions: self.axes.specialize(self.line.combined_dimensions),
            actions: self.line.actions.to_vec(),
            debug_render: false,
        })?;

        self.stack.add_space(self.flex_spacing, SpaceKind::Independent);

        let remaining = self.axes.specialize(Size2D {
            x: self.part.usable
                - self.part.dimensions.x
                - self.part.space.soft_or_zero(),
            y: self.line.combined_dimensions.y,
        });

        self.start_line();

        Ok(remaining)
    }

    fn start_line(&mut self) {
        let usable = self.stack.primary_usable();
        self.line = FlexLine::new(usable);
        self.part = PartialLine::new(usable);
    }

    fn finish_partial_line(&mut self) {
        let factor = self.axes.primary.axis.factor();
        let anchor =
            self.axes.primary.anchor(self.line.usable)
            - self.axes.primary.anchor(self.part.dimensions.x);

        for (offset, layout) in self.part.content.drain(..) {
            let pos = self.axes.specialize(Size2D::with_x(anchor + factor * offset));
            self.line.actions.add_layout(pos, layout);
        }

        self.line.combined_dimensions.x = match self.axes.primary.alignment {
            Alignment::Origin => self.part.dimensions.x,
            Alignment::Center => self.part.usable / 2 + self.part.dimensions.x / 2,
            Alignment::End => self.part.usable,
        };

        self.line.combined_dimensions.y.max_eq(self.part.dimensions.y);
    }

    fn layout_box(&mut self, boxed: Layout) -> LayoutResult<()> {
        let size = self.axes.generalize(boxed.dimensions);
        let new_dimension = self.part.dimensions.x
            + size.x
            + self.part.space.soft_or_zero();

        if new_dimension > self.part.usable {
            self.finish_line()?;

            while size.x > self.line.usable {
                if self.stack.space_is_last() {
                    lerr!("box does not fit into line");
                }

                self.stack.finish_space(true);
            }
        }

        if let SpaceState::Soft(space) = self.part.space {
            self.layout_space(space, SpaceKind::Hard);
        }

        let offset = self.part.dimensions.x;
        self.part.content.push((offset, boxed));

        self.part.dimensions.x += size.x;
        self.part.dimensions.y.max_eq(size.y);
        self.part.space = SpaceState::Allowed;

        Ok(())
    }

    fn layout_space(&mut self, space: Size, kind: SpaceKind) {
        if kind == SpaceKind::Soft {
            if self.part.space != SpaceState::Forbidden {
                self.part.space = SpaceState::Soft(space);
            }
        } else {
            if self.part.dimensions.x + space > self.part.usable {
                self.part.dimensions.x = self.part.usable;
            } else {
                self.part.dimensions.x += space;
            }

            if kind == SpaceKind::Hard {
                self.part.space = SpaceState::Forbidden;
            }
        }
    }

    fn layout_set_axes(&mut self, axes: LayoutAxes) {
        if axes.primary != self.axes.primary {
            self.finish_partial_line();

            let extent = self.line.combined_dimensions.x;
            let usable = self.line.usable;

            let new_usable = match axes.primary.alignment {
                Alignment::Origin if extent == Size::zero() => usable,
                Alignment::Center if extent < usable / 2 => usable - 2 * extent,
                Alignment::End => usable - extent,
                _ => Size::zero(),
            };

            self.part = PartialLine::new(new_usable);
        }

        if axes.secondary != self.axes.secondary {
            self.stack.set_axes(axes);
        }

        self.axes = axes;
    }
}
