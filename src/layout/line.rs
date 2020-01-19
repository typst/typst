use super::*;


/// The line layouter arranges boxes next to each other along a primary axis
/// and arranges the resulting lines using an underlying stack layouter.
#[derive(Debug, Clone)]
pub struct LineLayouter {
    /// The context for layouting.
    ctx: LineContext,
    /// The underlying stack layouter.
    stack: StackLayouter,
    /// The currently written line.
    run: LineRun,
}

/// The context for line layouting.
#[derive(Debug, Clone)]
pub struct LineContext {
    /// The spaces to layout in.
    pub spaces: LayoutSpaces,
    /// The initial layouting axes, which can be updated by the
    /// [`LineLayouter::set_axes`] method.
    pub axes: LayoutAxes,
    /// Which alignment to set on the resulting layout. This affects how it will
    /// be positioned in a parent box.
    pub alignment: LayoutAlignment,
    /// Whether to have repeated spaces or to use only the first and only once.
    pub repeat: bool,
    /// Whether to output a command which renders a debugging box showing the
    /// extent of the layout.
    pub debug: bool,
    /// The line spacing.
    pub line_spacing: Size,
}

/// A simple line of boxes.
#[derive(Debug, Clone)]
struct LineRun {
    /// The so-far accumulated layouts in the line.
    layouts: Vec<(Size, Layout)>,
    /// The width (primary size) and maximal height (secondary size) of the
    /// line.
    size: Size2D,
    /// The alignment of all layouts in the line.
    alignment: Option<LayoutAlignment>,
    /// The remaining usable space if another differently aligned line run
    /// already took up some space.
    usable: Option<Size>,
    /// A possibly cached soft spacing or spacing state.
    last_spacing: LastSpacing,
}

impl LineLayouter {
    /// Create a new line layouter.
    pub fn new(ctx: LineContext) -> LineLayouter {
        LineLayouter {
            stack: StackLayouter::new(StackContext {
                spaces: ctx.spaces.clone(),
                axes: ctx.axes,
                alignment: ctx.alignment,
                repeat: ctx.repeat,
                debug: ctx.debug,
            }),
            ctx,
            run: LineRun::new(),
        }
    }

    /// Add a layout to the run.
    pub fn add(&mut self, layout: Layout) {
        let axes = self.ctx.axes;

        if let Some(alignment) = self.run.alignment {
            if layout.alignment.secondary != alignment.secondary {
                // TODO: Issue warning for non-fitting alignment in
                //       non-repeating context.
                let fitting = self.stack.is_fitting_alignment(layout.alignment);
                if !fitting && self.ctx.repeat {
                    self.finish_space(true);
                } else {
                    self.finish_line();
                }
            } else if layout.alignment.primary < alignment.primary {
                self.finish_line();

            } else if layout.alignment.primary > alignment.primary {
                let mut rest_run = LineRun::new();

                let usable = self.stack.usable().get_primary(axes);
                rest_run.usable = Some(match layout.alignment.primary {
                    Alignment::Origin => unreachable!("origin > x"),
                    Alignment::Center => usable - 2 * self.run.size.x,
                    Alignment::End => usable - self.run.size.x,
                });

                rest_run.size.y = self.run.size.y;

                self.finish_line();
                self.stack.add_spacing(-rest_run.size.y, SpacingKind::Hard);

                self.run = rest_run;
            }
        }

        if let LastSpacing::Soft(spacing, _) = self.run.last_spacing {
            self.add_primary_spacing(spacing, SpacingKind::Hard);
        }

        let size = layout.dimensions.generalized(axes);

        if !self.usable().fits(size) {
            if !self.line_is_empty() {
                self.finish_line();
            }

            // TODO: Issue warning about overflow if there is overflow.
            if !self.usable().fits(size) {
                self.stack.skip_to_fitting_space(layout.dimensions);
            }
        }

        self.run.alignment = Some(layout.alignment);
        self.run.layouts.push((self.run.size.x, layout));

        self.run.size.x += size.x;
        self.run.size.y.max_eq(size.y);
        self.run.last_spacing = LastSpacing::None;
    }

    /// Add multiple layouts to the run.
    ///
    /// This function simply calls `add` repeatedly for each layout.
    pub fn add_multiple(&mut self, layouts: MultiLayout) {
        for layout in layouts {
            self.add(layout);
        }
    }

    /// The remaining usable size in the run.
    fn usable(&self) -> Size2D {
        // The base is the usable space per stack layouter.
        let mut usable = self.stack.usable().generalized(self.ctx.axes);

        // If this is a alignment-continuing line, we override the primary
        // usable size.
        if let Some(primary) = self.run.usable {
            usable.x = primary;
        }

        usable.x -= self.run.size.x;
        usable
    }

    /// Add primary spacing to the line.
    pub fn add_primary_spacing(&mut self, mut spacing: Size, kind: SpacingKind) {
        match kind {
            // A hard space is simply an empty box.
            SpacingKind::Hard => {
                spacing.min_eq(self.usable().x);
                self.run.size.x += spacing;
                self.run.last_spacing = LastSpacing::Hard;
            }

            // A soft space is cached if it is not consumed by a hard space or
            // previous soft space with higher level.
            SpacingKind::Soft(level) => {
                let consumes = match self.run.last_spacing {
                    LastSpacing::None => true,
                    LastSpacing::Soft(_, prev) if level < prev => true,
                    _ => false,
                };

                if consumes {
                    self.run.last_spacing = LastSpacing::Soft(spacing, level);
                }
            }
        }
    }

    /// Finish the run and add secondary spacing to the underlying stack.
    pub fn add_secondary_spacing(&mut self, spacing: Size, kind: SpacingKind) {
        self.finish_line_if_not_empty();
        self.stack.add_spacing(spacing, kind)
    }

    /// Change the layouting axes used by this layouter.
    pub fn set_axes(&mut self, axes: LayoutAxes) {
        self.finish_line_if_not_empty();
        self.ctx.axes = axes;
        self.stack.set_axes(axes)
    }

    /// Change the layouting spaces to use.
    ///
    /// If `replace_empty` is true, the current space is replaced if there are
    /// no boxes laid into it yet. Otherwise, only the followup spaces are
    /// replaced.
    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
        self.stack.set_spaces(spaces, replace_empty && self.line_is_empty());
    }

    /// Change the line spacing.
    pub fn set_line_spacing(&mut self, line_spacing: Size) {
        self.ctx.line_spacing = line_spacing;
    }

    /// The remaining unpadded, unexpanding spaces.
    pub fn remaining(&self) -> LayoutSpaces {
        let mut spaces = self.stack.remaining();
        *spaces[0].dimensions.get_secondary_mut(self.ctx.axes)
            -= self.run.size.y;
        spaces
    }

    /// Whether the currently set line is empty.
    pub fn line_is_empty(&self) -> bool {
        self.run.size == Size2D::ZERO && self.run.layouts.is_empty()
    }

    /// Finish the last line and compute the final multi-layout.
    pub fn finish(mut self) -> MultiLayout {
        self.finish_line_if_not_empty();
        self.stack.finish()
    }

    /// Finish the currently active space and start a new one.
    pub fn finish_space(&mut self, hard: bool) {
        self.finish_line_if_not_empty();
        self.stack.finish_space(hard)
    }

    /// Add the current line to the stack and start a new line.
    pub fn finish_line(&mut self) {
        let mut actions = LayoutActions::new();

        let layouts = std::mem::replace(&mut self.run.layouts, vec![]);
        for (offset, layout) in layouts {
            let x = match self.ctx.axes.primary.is_positive() {
                true => offset,
                false => self.run.size.x
                    - offset
                    - layout.dimensions.get_primary(self.ctx.axes),
            };

            let pos = Size2D::with_x(x);
            actions.add_layout(pos, layout);
        }

        self.stack.add(Layout {
            dimensions: self.run.size.specialized(self.ctx.axes),
            alignment: self.run.alignment
                .unwrap_or(LayoutAlignment::new(Origin, Origin)),
            actions: actions.to_vec(),
        });

        self.run = LineRun::new();

        self.stack.add_spacing(self.ctx.line_spacing, SpacingKind::LINE);
    }

    /// Finish the current line if it is not empty.
    fn finish_line_if_not_empty(&mut self) {
        if !self.line_is_empty() {
            self.finish_line()
        }
    }
}

impl LineRun {
    fn new() -> LineRun {
        LineRun {
            layouts: vec![],
            size: Size2D::ZERO,
            alignment: None,
            usable: None,
            last_spacing: LastSpacing::Hard,
        }
    }
}
