use super::*;

/// A node that arranges its children into a paragraph.
///
/// Boxes are laid out along the cross axis as long as they fit into a line.
/// When necessary, a line break is inserted and the new line is offset along
/// the main axis by the height of the previous line plus extra line spacing.
#[derive(Debug, Clone, PartialEq)]
pub struct Par {
    pub dirs: Gen2<Dir>,
    pub line_spacing: f64,
    pub children: Vec<LayoutNode>,
    pub aligns: Gen2<GenAlign>,
    pub expand: Spec2<bool>,
}

#[async_trait(?Send)]
impl Layout for Par {
    async fn layout(
        &self,
        ctx: &mut LayoutContext,
        constraints: LayoutConstraints,
    ) -> Vec<LayoutItem> {
        let mut layouter = LineLayouter::new(LineContext {
            dirs: self.dirs,
            spaces: constraints.spaces,
            repeat: constraints.repeat,
            line_spacing: self.line_spacing,
            expand: self.expand,
        });

        for child in &self.children {
            let items = child
                .layout(ctx, LayoutConstraints {
                    spaces: layouter.remaining(),
                    repeat: constraints.repeat,
                })
                .await;

            for item in items {
                match item {
                    LayoutItem::Spacing(amount) => layouter.push_spacing(amount),
                    LayoutItem::Box(boxed, aligns) => layouter.push_box(boxed, aligns),
                }
            }
        }

        layouter
            .finish()
            .into_iter()
            .map(|boxed| LayoutItem::Box(boxed, self.aligns))
            .collect()
    }
}

impl From<Par> for LayoutNode {
    fn from(par: Par) -> Self {
        Self::dynamic(par)
    }
}

/// Performs the line layouting.
struct LineLayouter {
    /// The context used for line layouting.
    ctx: LineContext,
    /// The underlying layouter that stacks the finished lines.
    stack: StackLayouter,
    /// The in-progress line.
    run: LineRun,
}

/// The context for line layouting.
#[derive(Debug, Clone)]
struct LineContext {
    /// The layout directions.
    dirs: Gen2<Dir>,
    /// The spaces to layout into.
    spaces: Vec<LayoutSpace>,
    /// Whether to spill over into copies of the last space or finish layouting
    /// when the last space is used up.
    repeat: bool,
    /// The spacing to be inserted between each pair of lines.
    line_spacing: f64,
    /// Whether to expand the size of the resulting layout to the full size of
    /// this space or to shrink it to fit the content.
    expand: Spec2<bool>,
}

impl LineLayouter {
    /// Create a new line layouter.
    fn new(ctx: LineContext) -> Self {
        Self {
            stack: StackLayouter::new(StackContext {
                spaces: ctx.spaces.clone(),
                dirs: ctx.dirs,
                repeat: ctx.repeat,
                expand: ctx.expand,
            }),
            ctx,
            run: LineRun::new(),
        }
    }

    /// Add a layout.
    fn push_box(&mut self, layout: BoxLayout, aligns: Gen2<GenAlign>) {
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

                // FIXME: Alignment in non-expanding parent.
                rest_run.usable = Some(match aligns.cross {
                    GenAlign::Start => unreachable!("start > x"),
                    GenAlign::Center => usable - 2.0 * self.run.size.cross,
                    GenAlign::End => usable - self.run.size.cross,
                });

                self.finish_line();

                // Move back up in the stack layouter.
                self.stack.push_spacing(-rest_run.size.main - self.ctx.line_spacing);
                self.run = rest_run;
            }
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
    }

    /// Add spacing to the line.
    fn push_spacing(&mut self, mut spacing: f64) {
        spacing = spacing.min(self.usable().cross);
        self.run.size.cross += spacing;
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

    /// The remaining inner spaces. If something is laid out into these spaces,
    /// it will fit into this layouter's underlying stack.
    fn remaining(&self) -> Vec<LayoutSpace> {
        let mut spaces = self.stack.remaining();
        *spaces[0].size.get_mut(self.ctx.dirs.main.axis()) -= self.run.size.main;
        spaces
    }

    /// Whether the currently set line is empty.
    fn line_is_empty(&self) -> bool {
        self.run.size == Gen2::ZERO && self.run.layouts.is_empty()
    }

    /// Finish everything up and return the final collection of boxes.
    fn finish(mut self) -> Vec<BoxLayout> {
        self.finish_line_if_not_empty();
        self.stack.finish()
    }

    /// Finish the active space and start a new one.
    ///
    /// At the top level, this is a page break.
    fn finish_space(&mut self, hard: bool) {
        self.finish_line_if_not_empty();
        self.stack.finish_space(hard)
    }

    /// Finish the active line and start a new one.
    fn finish_line(&mut self) {
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

        self.stack.push_box(layout, aligns);
        self.stack.push_spacing(self.ctx.line_spacing);
        self.run = LineRun::new();
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
}

impl LineRun {
    fn new() -> Self {
        Self {
            layouts: vec![],
            size: Gen2::ZERO,
            aligns: None,
            usable: None,
        }
    }
}
