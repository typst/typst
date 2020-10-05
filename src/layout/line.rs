//! Arranging boxes into lines.
//!
//! Along the primary axis, the boxes are laid out next to each other as long as
//! they fit into a line. When necessary, a line break is inserted and the new
//! line is offset along the secondary axis by the height of the previous line
//! plus extra line spacing.
//!
//! Internally, the line layouter uses a stack layouter to stack the finished
//! lines on top of each.

use super::*;

/// Performs the line layouting.
pub struct LineLayouter {
    /// The context used for line layouting.
    ctx: LineContext,
    /// The underlying layouter that stacks the finished lines.
    stack: StackLayouter,
    /// The in-progress line.
    run: LineRun,
}

/// The context for line layouting.
#[derive(Debug, Clone)]
pub struct LineContext {
    /// The initial layouting system, which can be updated through `set_sys`.
    pub sys: LayoutSystem,
    /// The spaces to layout into.
    pub spaces: Vec<LayoutSpace>,
    /// Whether to spill over into copies of the last space or finish layouting
    /// when the last space is used up.
    pub repeat: bool,
    /// The spacing to be inserted between each pair of lines.
    pub line_spacing: f64,
}

impl LineLayouter {
    /// Create a new line layouter.
    pub fn new(ctx: LineContext) -> Self {
        Self {
            stack: StackLayouter::new(StackContext {
                spaces: ctx.spaces.clone(),
                sys: ctx.sys,
                repeat: ctx.repeat,
            }),
            ctx,
            run: LineRun::new(),
        }
    }

    /// Add a layout.
    pub fn add(&mut self, layout: BoxLayout, align: LayoutAlign) {
        let sys = self.ctx.sys;

        if let Some(prev) = self.run.align {
            if align.secondary != prev.secondary {
                // TODO: Issue warning for non-fitting alignment in
                // non-repeating context.
                let fitting = self.stack.is_fitting_alignment(align);
                if !fitting && self.ctx.repeat {
                    self.finish_space(true);
                } else {
                    self.finish_line();
                }
            } else if align.primary < prev.primary {
                self.finish_line();
            } else if align.primary > prev.primary {
                let mut rest_run = LineRun::new();

                let usable = self.stack.usable().get(sys.primary.axis());
                rest_run.usable = Some(match align.primary {
                    GenAlign::Start => unreachable!("start > x"),
                    GenAlign::Center => usable - 2.0 * self.run.size.width,
                    GenAlign::End => usable - self.run.size.width,
                });

                rest_run.size.height = self.run.size.height;

                self.finish_line();
                self.stack.add_spacing(-rest_run.size.height, SpacingKind::Hard);

                self.run = rest_run;
            }
        }

        if let LastSpacing::Soft(spacing, _) = self.run.last_spacing {
            self.add_primary_spacing(spacing, SpacingKind::Hard);
        }

        let size = layout.size.generalized(sys);

        if !self.usable().fits(size) {
            if !self.line_is_empty() {
                self.finish_line();
            }

            // TODO: Issue warning about overflow if there is overflow.
            if !self.usable().fits(size) {
                self.stack.skip_to_fitting_space(layout.size);
            }
        }

        self.run.align = Some(align);
        self.run.layouts.push((self.run.size.width, layout));

        self.run.size.width += size.width;
        self.run.size.height = self.run.size.height.max(size.height);
        self.run.last_spacing = LastSpacing::None;
    }

    /// The remaining usable size of the line.
    ///
    /// This specifies how much more would fit before a line break would be
    /// needed.
    fn usable(&self) -> Size {
        // The base is the usable space of the stack layouter.
        let mut usable = self.stack.usable().generalized(self.ctx.sys);

        // If there was another run already, override the stack's size.
        if let Some(primary) = self.run.usable {
            usable.width = primary;
        }

        usable.width -= self.run.size.width;
        usable
    }

    /// Add spacing to the line.
    pub fn add_primary_spacing(&mut self, mut spacing: f64, kind: SpacingKind) {
        match kind {
            SpacingKind::Hard => {
                spacing = spacing.min(self.usable().width);
                self.run.size.width += spacing;
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

    /// Update the layouting system.
    pub fn set_sys(&mut self, sys: LayoutSystem) {
        self.finish_line_if_not_empty();
        self.ctx.sys = sys;
        self.stack.set_sys(sys)
    }

    /// Update the layouting spaces.
    ///
    /// If `replace_empty` is true, the current space is replaced if there are
    /// no boxes laid out into it yet. Otherwise, the followup spaces are
    /// replaced.
    pub fn set_spaces(&mut self, spaces: Vec<LayoutSpace>, replace_empty: bool) {
        self.stack.set_spaces(spaces, replace_empty && self.line_is_empty());
    }

    /// Update the line spacing.
    pub fn set_line_spacing(&mut self, line_spacing: f64) {
        self.ctx.line_spacing = line_spacing;
    }

    /// The remaining inner spaces. If something is laid out into these spaces,
    /// it will fit into this layouter's underlying stack.
    pub fn remaining(&self) -> Vec<LayoutSpace> {
        let mut spaces = self.stack.remaining();
        *spaces[0].size.get_mut(self.ctx.sys.secondary.axis()) -= self.run.size.height;
        spaces
    }

    /// Whether the currently set line is empty.
    pub fn line_is_empty(&self) -> bool {
        self.run.size == Size::ZERO && self.run.layouts.is_empty()
    }

    /// Finish everything up and return the final collection of boxes.
    pub fn finish(mut self) -> Vec<BoxLayout> {
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
        let mut layout = BoxLayout::new(self.run.size.specialized(self.ctx.sys));
        let align = self.run.align.unwrap_or_default();

        let layouts = std::mem::take(&mut self.run.layouts);
        for (offset, child) in layouts {
            let x = match self.ctx.sys.primary.is_positive() {
                true => offset,
                false => {
                    self.run.size.width
                        - offset
                        - child.size.get(self.ctx.sys.primary.axis())
                }
            };

            let pos = Point::new(x, 0.0);
            layout.push_layout(pos, child);
        }

        self.stack.add(layout, align);

        self.run = LineRun::new();
        self.stack.add_spacing(self.ctx.line_spacing, SpacingKind::LINE);
    }

    fn finish_line_if_not_empty(&mut self) {
        if !self.line_is_empty() {
            self.finish_line()
        }
    }
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
