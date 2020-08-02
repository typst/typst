//! The line layouter arranges boxes into lines.
//!
//! Along the primary axis, the boxes are laid out next to each other while they
//! fit into a line. When a line break is necessary, the line is finished and a
//! new line is started offset on the secondary axis by the height of previous
//! line and the extra line spacing.
//!
//! Internally, the line layouter uses a stack layouter to arrange the finished
//! lines.

use super::stack::{StackLayouter, StackContext};
use super::*;

/// Performs the line layouting.
#[derive(Debug)]
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
    pub align: LayoutAlign,
    /// Whether to have repeated spaces or to use only the first and only once.
    pub repeat: bool,
    /// The line spacing.
    pub line_spacing: f64,
}

/// A line run is a sequence of boxes with the same alignment that are arranged
/// in a line. A real line can consist of multiple runs with different
/// alignments.
#[derive(Debug)]
struct LineRun {
    /// The so-far accumulated layouts in the line.
    layouts: Vec<(f64, BoxLayout)>,
    /// The width and maximal height of the line.
    size: Size,
    /// The alignment of all layouts in the line.
    ///
    /// When a new run is created the alignment is yet to be determined. Once a
    /// layout is added, it is decided which alignment the run has and all
    /// further elements of the run must have this alignment.
    align: Option<LayoutAlign>,
    /// If another line run with different alignment already took up some space
    /// of the line, this run has less space and how much is stored here.
    usable: Option<f64>,
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
                align: ctx.align,
                repeat: ctx.repeat,
            }),
            ctx,
            run: LineRun::new(),
        }
    }

    /// Add a layout to the run.
    pub fn add(&mut self, layout: BoxLayout) {
        let axes = self.ctx.axes;

        if let Some(align) = self.run.align {
            if layout.align.secondary != align.secondary {
                // TODO: Issue warning for non-fitting alignment in
                //       non-repeating context.
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

    /// Add multiple layouts to the run.
    ///
    /// This function simply calls `add` repeatedly for each layout.
    pub fn add_multiple(&mut self, layouts: MultiLayout) {
        for layout in layouts {
            self.add(layout);
        }
    }

    /// The remaining usable size of the run.
    ///
    /// This specifies how much more fits before a line break needs to be
    /// issued.
    fn usable(&self) -> Size {
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

    /// Add spacing along the primary axis to the line.
    pub fn add_primary_spacing(&mut self, mut spacing: f64, kind: SpacingKind) {
        match kind {
            // A hard space is simply an empty box.
            SpacingKind::Hard => {
                spacing = spacing.min(self.usable().x);
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

    /// Finish the line and add secondary spacing to the underlying stack.
    pub fn add_secondary_spacing(&mut self, spacing: f64, kind: SpacingKind) {
        self.finish_line_if_not_empty();
        self.stack.add_spacing(spacing, kind)
    }

    /// Update the layouting axes used by this layouter.
    pub fn set_axes(&mut self, axes: LayoutAxes) {
        self.finish_line_if_not_empty();
        self.ctx.axes = axes;
        self.stack.set_axes(axes)
    }

    /// Update the layouting spaces to use.
    ///
    /// If `replace_empty` is true, the current space is replaced if there are
    /// no boxes laid into it yet. Otherwise, only the followup spaces are
    /// replaced.
    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
        self.stack.set_spaces(spaces, replace_empty && self.line_is_empty());
    }

    /// Update the line spacing.
    pub fn set_line_spacing(&mut self, line_spacing: f64) {
        self.ctx.line_spacing = line_spacing;
    }

    /// The remaining inner layout spaces. Inner means, that padding is already
    /// subtracted and the spaces are unexpanding. This can be used to signal
    /// a function how much space it has to layout itself.
    pub fn remaining(&self) -> LayoutSpaces {
        let mut spaces = self.stack.remaining();
        *spaces[0].size.secondary_mut(self.ctx.axes)
            -= self.run.size.y;
        spaces
    }

    /// Whether the currently set line is empty.
    pub fn line_is_empty(&self) -> bool {
        self.run.size == Size::ZERO && self.run.layouts.is_empty()
    }

    /// Finish the last line and compute the final list of boxes.
    pub fn finish(mut self) -> MultiLayout {
        self.finish_line_if_not_empty();
        self.stack.finish()
    }

    /// Finish the currently active space and start a new one.
    ///
    /// At the top level, this is a page break.
    pub fn finish_space(&mut self, hard: bool) {
        self.finish_line_if_not_empty();
        self.stack.finish_space(hard)
    }

    /// Finish the line and start a new one.
    pub fn finish_line(&mut self) {
        let mut elements = LayoutElements::new();

        let layouts = std::mem::take(&mut self.run.layouts);
        for (offset, layout) in layouts {
            let x = match self.ctx.axes.primary.is_positive() {
                true => offset,
                false => self.run.size.x
                    - offset
                    - layout.size.primary(self.ctx.axes),
            };

            let pos = Size::with_x(x);
            elements.extend_offset(pos, layout.elements);
        }

        self.stack.add(BoxLayout {
            size: self.run.size.specialized(self.ctx.axes),
            align: self.run.align
                .unwrap_or(LayoutAlign::new(Start, Start)),
                elements
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
            size: Size::ZERO,
            align: None,
            usable: None,
            last_spacing: LastSpacing::Hard,
        }
    }
}
