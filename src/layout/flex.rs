use super::*;

/// The flex layouter first arranges boxes along a primary and if necessary also
/// along a secondary axis.
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
    Space(Size, SpacingKind),
    SetAxes(LayoutAxes),
    Break,
}

#[derive(Debug, Clone)]
struct FlexLine {
    usable: Size,
    actions: LayoutActions,
    combined_dimensions: Size2D,
}

impl FlexLine {
    fn new(usable: Size) -> FlexLine {
        FlexLine {
            usable,
            actions: LayoutActions::new(),
            combined_dimensions: Size2D::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
struct PartialLine {
    usable: Size,
    content: Vec<(Size, Layout)>,
    dimensions: Size2D,
    space: LastSpacing,
}

impl PartialLine {
    fn new(usable: Size) -> PartialLine {
        PartialLine {
            usable,
            content: vec![],
            dimensions: Size2D::ZERO,
            space: LastSpacing::Hard,
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
    pub alignment: LayoutAlignment,
    pub flex_spacing: Size,
}

impl FlexLayouter {
    /// Create a new flex layouter.
    pub fn new(ctx: FlexContext) -> FlexLayouter {
        let stack = StackLayouter::new(StackContext {
            spaces: ctx.spaces,
            axes: ctx.axes,
            alignment: ctx.alignment,
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

    pub fn add_primary_space(&mut self, space: Size, kind: SpacingKind) {
        self.units.push(FlexUnit::Space(space, kind))
    }

    pub fn add_secondary_space(&mut self, space: Size, kind: SpacingKind) -> LayoutResult<()> {
        if !self.run_is_empty() {
            self.finish_run()?;
        }
        Ok(self.stack.add_spacing(space, kind))
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
        unimplemented!()
    }

    fn start_line(&mut self) {
        unimplemented!()
    }

    #[allow(dead_code)]
    fn finish_partial_line(&mut self) {
        unimplemented!()
    }

    fn layout_box(&mut self, _boxed: Layout) -> LayoutResult<()> {
        unimplemented!()
    }

    fn layout_space(&mut self, _space: Size, _kind: SpacingKind) {
        unimplemented!()
    }

    fn layout_set_axes(&mut self, _axes: LayoutAxes) {
        unimplemented!()
    }
}
