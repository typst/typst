use super::*;

/// Layouts syntax trees into boxes.
pub fn layout_tree(tree: &SyntaxTree, ctx: LayoutContext) -> LayoutResult<MultiLayout> {
    let mut layouter = TreeLayouter::new(ctx);
    layouter.layout(tree)?;
    layouter.finish()
}

#[derive(Debug, Clone)]
struct TreeLayouter<'a, 'p> {
    ctx: LayoutContext<'a, 'p>,
    stack: StackLayouter,
    flex: FlexLayouter,
    style: Cow<'a, TextStyle>,
}

impl<'a, 'p> TreeLayouter<'a, 'p> {
    /// Create a new layouter.
    fn new(ctx: LayoutContext<'a, 'p>) -> TreeLayouter<'a, 'p> {
        TreeLayouter {
            ctx,
            stack: StackLayouter::new(StackContext {
                spaces: ctx.spaces,
                axes: ctx.axes,
            }),
            flex: FlexLayouter::new(FlexContext {
                flex_spacing: flex_spacing(&ctx.style),
                spaces: ctx.spaces.iter().map(|space| space.usable_space(true)).collect(),
                axes: ctx.axes,
            }),
            style: Cow::Borrowed(ctx.style),
        }
    }

    /// Layout a syntax tree.
    fn layout(&mut self, tree: &SyntaxTree) -> LayoutResult<()> {
        for node in &tree.nodes {
            match &node.val {
                Node::Text(text) => {
                    self.flex.add(layout_text(text, TextContext {
                        loader: &self.ctx.loader,
                        style: &self.style,
                    })?);
                }

                Node::Space => {
                    if !self.flex.is_empty() {
                        self.flex.add_space(self.style.word_spacing * self.style.font_size);
                    }
                }
                Node::Newline => {
                    if !self.flex.is_empty() {
                        self.finish_paragraph()?;
                    }
                }

                Node::ToggleItalics => self.style.to_mut().toggle_class(FontClass::Italic),
                Node::ToggleBold => self.style.to_mut().toggle_class(FontClass::Bold),
                Node::ToggleMonospace => self.style.to_mut().toggle_class(FontClass::Monospace),

                Node::Func(func) => self.layout_func(func)?,
            }
        }

        Ok(())
    }

    /// Layout a function.
    fn layout_func(&mut self, func: &FuncCall) -> LayoutResult<()> {
        // Finish the current flex layout on a copy to find out how
        // much space would be remaining if we finished.
        let mut lookahead = self.stack.clone();
        lookahead.add_multiple(self.flex.clone().finish()?)?;
        let spaces = lookahead.remaining(true);

        let commands = func.body.val.layout(LayoutContext {
            style: &self.style,
            spaces,
            .. self.ctx
        })?;

        for command in commands {
            self.execute(command)?;
        }

        Ok(())
    }

    fn execute(&mut self, command: Command) -> LayoutResult<()> {
        match command {
            Command::LayoutTree(tree) => self.layout(tree)?,

            Command::Add(layout) => self.flex.add(layout),
            Command::AddMultiple(layouts) => self.flex.add_multiple(layouts),

            Command::FinishFlexRun => self.flex.add_break(),
            Command::FinishFlexLayout => self.finish_paragraph()?,
            Command::FinishLayout => self.finish_layout(true)?,

            Command::SetStyle(style) => *self.style.to_mut() = style,
            Command::SetAxes(axes) => {
                self.stack.set_axes(axes);
                self.flex.set_axes(axes);
                self.ctx.axes = axes;
            }
        }

        Ok(())
    }

    /// Finish the layout.
    fn finish(mut self) -> LayoutResult<MultiLayout> {
        self.finish_flex()?;
        Ok(self.stack.finish())
    }

    /// Finish the current stack layout.
    fn finish_layout(&mut self, include_empty: bool) -> LayoutResult<()> {
        self.finish_flex()?;
        self.stack.finish_layout(include_empty);
        self.start_new_flex();
        Ok(())
    }

    /// Finish the current flex layout and add space after it.
    fn finish_paragraph(&mut self) -> LayoutResult<()> {
        self.finish_flex()?;
        self.stack.add_space(paragraph_spacing(&self.style));
        self.start_new_flex();
        Ok(())
    }

    /// Finish the current flex layout and add it the stack.
    fn finish_flex(&mut self) -> LayoutResult<()> {
        if !self.flex.is_empty() {
            let layouts = self.flex.finish()?;
            self.stack.add_multiple(layouts)?;
        }
        Ok(())
    }

    /// Start a new flex layout.
    fn start_new_flex(&mut self) {
        self.flex = FlexLayouter::new(FlexContext {
            flex_spacing: flex_spacing(&self.style),
            spaces: self.stack.remaining(true),
            axes: self.ctx.axes,
        });
    }
}

fn flex_spacing(style: &TextStyle) -> Size {
    (style.line_spacing - 1.0) * style.font_size
}

fn paragraph_spacing(style: &TextStyle) -> Size {
    (style.paragraph_spacing - 1.0) * style.font_size
}
