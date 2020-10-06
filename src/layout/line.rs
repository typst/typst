//! Arranging boxes into lines.
//!
//! The boxes are laid out along the cross axis as long as they fit into a line.
//! When necessary, a line break is inserted and the new line is offset along
//! the main axis by the height of the previous line plus extra line spacing.
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
    /// The layout directions.
    pub dirs: Gen2<Dir>,
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
                dirs: ctx.dirs,
                repeat: ctx.repeat,
            }),
            ctx,
            run: LineRun::new(),
        }
    }

    /// Add a layout.
    pub fn add(&mut self, layout: BoxLayout, aligns: Gen2<GenAlign>) {
        let dirs = self.ctx.dirs;
        if let Some(prev) = self.run.aligns {
            if aligns.main != prev.main {
                // TODO: Issue warning for non-fitting alignment in
                // non-repeating context.
                let fitting = aligns.main >= self.stack.space.allowed_align;
                if !fitting && self.ctx.repeat {
                    self.finish_space(true);
                } else {
                    self.finish_line();
                }
            } else if aligns.cross < prev.cross {
                self.finish_line();
            } else if aligns.cross > prev.cross {
                let usable = self.stack.usable().get(dirs.cross.axis());

                let mut rest_run = LineRun::new();
                rest_run.size.main = self.run.size.main;
                rest_run.usable = Some(match aligns.cross {
                    GenAlign::Start => unreachable!("start > x"),
                    GenAlign::Center => usable - 2.0 * self.run.size.cross,
                    GenAlign::End => usable - self.run.size.cross,
                });

                self.finish_line();

                // Move back up in the stack layouter.
                self.stack.add_spacing(-rest_run.size.main, SpacingKind::Hard);
                self.run = rest_run;
            }
        }

        if let LastSpacing::Soft(spacing, _) = self.run.last_spacing {
            self.add_cross_spacing(spacing, SpacingKind::Hard);
        }

        let size = layout.size.switch(dirs);
        let usable = self.usable();

        if usable.main < size.main || usable.cross < size.cross {
            if !self.line_is_empty() {
                self.finish_line();
            }

            // TODO: Issue warning about overflow if there is overflow.
            let usable = self.usable();
            if usable.main < size.main || usable.cross < size.cross {
                self.stack.skip_to_fitting_space(layout.size);
            }
        }

        self.run.aligns = Some(aligns);
        self.run.layouts.push((self.run.size.cross, layout));

        self.run.size.cross += size.cross;
        self.run.size.main = self.run.size.main.max(size.main);
        self.run.last_spacing = LastSpacing::None;
    }

    /// The remaining usable size of the line.
    ///
    /// This specifies how much more would fit before a line break would be
    /// needed.
    fn usable(&self) -> Gen2<f64> {
        // The base is the usable space of the stack layouter.
        let mut usable = self.stack.usable().switch(self.ctx.dirs);

        // If there was another run already, override the stack's size.
        if let Some(cross) = self.run.usable {
            usable.cross = cross;
        }

        usable.cross -= self.run.size.cross;
        usable
    }

    /// Finish the line and add spacing to the underlying stack.
    pub fn add_main_spacing(&mut self, spacing: f64, kind: SpacingKind) {
        self.finish_line_if_not_empty();
        self.stack.add_spacing(spacing, kind)
    }

    /// Add spacing to the line.
    pub fn add_cross_spacing(&mut self, mut spacing: f64, kind: SpacingKind) {
        match kind {
            SpacingKind::Hard => {
                spacing = spacing.min(self.usable().cross);
                self.run.size.cross += spacing;
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
        *spaces[0].size.get_mut(self.ctx.dirs.main.axis()) -= self.run.size.main;
        spaces
    }

    /// Whether the currently set line is empty.
    pub fn line_is_empty(&self) -> bool {
        self.run.size == Gen2::ZERO && self.run.layouts.is_empty()
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
        let dirs = self.ctx.dirs;

        let mut layout = BoxLayout::new(self.run.size.switch(dirs).to_size());
        let aligns = self.run.aligns.unwrap_or_default();

        let children = std::mem::take(&mut self.run.layouts);
        for (offset, child) in children {
            let cross = if dirs.cross.is_positive() {
                offset
            } else {
                self.run.size.cross - offset - child.size.get(dirs.cross.axis())
            };

            let pos = Gen2::new(0.0, cross).switch(dirs).to_point();
            layout.push_layout(pos, child);
        }

        self.stack.add(layout, aligns);

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
    size: Gen2<f64>,
    /// The alignment of all layouts in the line.
    ///
    /// When a new run is created the alignment is yet to be determined and
    /// `None` as such. Once a layout is added, its alignment decides the
    /// alignment for the whole run.
    aligns: Option<Gen2<GenAlign>>,
    /// The amount of cross-space left by another run on the same line or `None`
    /// if this is the only run so far.
    usable: Option<f64>,
    /// The spacing state. This influences how new spacing is handled, e.g. hard
    /// spacing may override soft spacing.
    last_spacing: LastSpacing,
}

impl LineRun {
    fn new() -> Self {
        Self {
            layouts: vec![],
            size: Gen2::ZERO,
            aligns: None,
            usable: None,
            last_spacing: LastSpacing::Hard,
        }
    }
}
