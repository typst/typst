//! Arranging boxes into lines.
//!
//! Along the primary axis, the boxes are laid out next to each other as long as
//! they fit into a line. When necessary, a line break is inserted and the new
//! line is offset along the secondary axis by the height of the previous line
//! plus extra line spacing.
//!
//! Internally, the line layouter uses a stack layouter to stack the finished
//! lines on top of each.

use super::stack::{StackContext, StackLayouter};
use super::*;

/// Performs the line layouting.
pub struct LineLayouter {
    ctx: LineContext,
    stack: StackLayouter,
    /// The in-progress line.
    run: LineRun,
}

/// The context for line layouting.
#[derive(Debug, Clone)]
pub struct LineContext {
    /// The spaces to layout into.
    pub spaces: LayoutSpaces,
    /// The initial layouting axes, which can be updated through `set_axes`.
    pub axes: LayoutAxes,
    /// The alignment of the _resulting_ layout. This does not effect the line
    /// layouting itself, but rather how the finished layout will be positioned
    /// in a parent layout.
    pub align: LayoutAlign,
    /// Whether to spill over into copies of the last space or finish layouting
    /// when the last space is used up.
    pub repeat: bool,
    /// The spacing to be inserted between each pair of lines.
    pub line_spacing: f64,
}

/// A sequence of boxes with the same alignment. A real line can consist of
/// multiple runs with different alignments.
struct LineRun {
    /// The so-far accumulated items of the run.
    layouts: Vec<(f64, BoxLayout)>,
    /// The summed width and maximal height of the run.
    size: Size,
    /// The alignment of all layouts in the line.
    ///
    /// When a new run is created the alignment is yet to be determined and
    /// `None` as such. Once a layout is added, its alignment decides the
    /// alignment for the whole run.
    align: Option<LayoutAlign>,
    /// The amount of space left by another run on the same line or `None` if
    /// this is the only run so far.
    usable: Option<f64>,
    /// The spacing state. This influences how new spacing is handled, e.g. hard
    /// spacing may override soft spacing.
    last_spacing: LastSpacing,
}

impl LineLayouter {
    /// Create a new line layouter.
    pub fn new(ctx: LineContext) -> Self {
        Self {
            stack: StackLayouter::new(StackContext {
                spaces: ctx.spaces.clone(),
                axes: ctx.axes,
                align: ctx.align,
                repeat: ctx.repeat,
            }),
            ctx,
            run: LineRun::new(),
        }
    }

    /// Add a layout.
    pub fn add(&mut self, layout: BoxLayout) {
        let axes = self.ctx.axes;

        if let Some(align) = self.run.align {
            if layout.align.secondary != align.secondary {
                // TODO: Issue warning for non-fitting alignment in
                // non-repeating context.
                let fitting = self.stack.is_fitting_alignment(layout.align);
                if !fitting && self.ctx.repeat {
                    self.finish_space(true);
                } else {
                    self.finish_line();
                }
            } else if layout.align.primary < align.primary {
                self.finish_line();
            } else if layout.align.primary > align.primary {
                let mut rest_run = LineRun::new();

                let usable = self.stack.usable().primary(axes);
                rest_run.usable = Some(match layout.align.primary {
                    GenAlign::Start => unreachable!("start > x"),
                    GenAlign::Center => usable - 2.0 * self.run.size.x,
                    GenAlign::End => usable - self.run.size.x,
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

        let size = layout.size.generalized(axes);

        if !self.usable().fits(size) {
            if !self.line_is_empty() {
                self.finish_line();
            }

            // TODO: Issue warning about overflow if there is overflow.
            if !self.usable().fits(size) {
                self.stack.skip_to_fitting_space(layout.size);
            }
        }

        self.run.align = Some(layout.align);
        self.run.layouts.push((self.run.size.x, layout));

        self.run.size.x += size.x;
        self.run.size.y = self.run.size.y.max(size.y);
        self.run.last_spacing = LastSpacing::None;
    }

    /// Add multiple layouts.
    ///
    /// This is equivalent to calling `add` repeatedly for each layout.
    pub fn add_multiple(&mut self, layouts: MultiLayout) {
        for layout in layouts {
            self.add(layout);
        }
    }

    /// The remaining usable size of the line.
    ///
    /// This specifies how much more would fit before a line break would be
    /// needed.
    fn usable(&self) -> Size {
        // The base is the usable space of the stack layouter.
        let mut usable = self.stack.usable().generalized(self.ctx.axes);

        // If there was another run already, override the stack's size.
        if let Some(primary) = self.run.usable {
            usable.x = primary;
        }

        usable.x -= self.run.size.x;
        usable
    }

    /// Add spacing to the line.
    pub fn add_primary_spacing(&mut self, mut spacing: f64, kind: SpacingKind) {
        match kind {
            SpacingKind::Hard => {
                spacing = spacing.min(self.usable().x);
                self.run.size.x += spacing;
                self.run.last_spacing = LastSpacing::Hard;
            }

            // A soft space is cached since it might be consumed by a hard
            // spacing.
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

    /// Finish the line and add spacing to the underlying stack.
    pub fn add_secondary_spacing(&mut self, spacing: f64, kind: SpacingKind) {
        self.finish_line_if_not_empty();
        self.stack.add_spacing(spacing, kind)
    }

    /// Update the layouting axes.
    pub fn set_axes(&mut self, axes: LayoutAxes) {
        self.finish_line_if_not_empty();
        self.ctx.axes = axes;
        self.stack.set_axes(axes)
    }

    /// Update the layouting spaces.
    ///
    /// If `replace_empty` is true, the current space is replaced if there are
    /// no boxes laid out into it yet. Otherwise, the followup spaces are
    /// replaced.
    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
        self.stack.set_spaces(spaces, replace_empty && self.line_is_empty());
    }

    /// Update the line spacing.
    pub fn set_line_spacing(&mut self, line_spacing: f64) {
        self.ctx.line_spacing = line_spacing;
    }

    /// The remaining inner spaces. If something is laid out into these spaces,
    /// it will fit into this layouter's underlying stack.
    pub fn remaining(&self) -> LayoutSpaces {
        let mut spaces = self.stack.remaining();
        *spaces[0].size.secondary_mut(self.ctx.axes) -= self.run.size.y;
        spaces
    }

    /// Whether the currently set line is empty.
    pub fn line_is_empty(&self) -> bool {
        self.run.size == Size::ZERO && self.run.layouts.is_empty()
    }

    /// Finish everything up and return the final collection of boxes.
    pub fn finish(mut self) -> MultiLayout {
        self.finish_line_if_not_empty();
        self.stack.finish()
    }

    /// Finish the active space and start a new one.
    ///
    /// At the top level, this is a page break.
    pub fn finish_space(&mut self, hard: bool) {
        self.finish_line_if_not_empty();
        self.stack.finish_space(hard)
    }

    /// Finish the active line and start a new one.
    pub fn finish_line(&mut self) {
        let mut elements = LayoutElements::new();

        let layouts = std::mem::take(&mut self.run.layouts);
        for (offset, layout) in layouts {
            let x = match self.ctx.axes.primary.is_positive() {
                true => offset,
                false => self.run.size.x - offset - layout.size.primary(self.ctx.axes),
            };

            let pos = Size::with_x(x);
            elements.extend_offset(pos, layout.elements);
        }

        self.stack.add(BoxLayout {
            size: self.run.size.specialized(self.ctx.axes),
            align: self.run.align.unwrap_or(LayoutAlign::new(Start, Start)),
            elements,
        });

        self.run = LineRun::new();

        self.stack.add_spacing(self.ctx.line_spacing, SpacingKind::LINE);
    }

    fn finish_line_if_not_empty(&mut self) {
        if !self.line_is_empty() {
            self.finish_line()
        }
    }
}

impl LineRun {
    fn new() -> Self {
        Self {
            layouts: vec![],
            size: Size::ZERO,
            align: None,
            usable: None,
            last_spacing: LastSpacing::Hard,
        }
    }
}
