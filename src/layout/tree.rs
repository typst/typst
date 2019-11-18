use super::*;
use smallvec::smallvec;

/// Layouts syntax trees into boxes.
pub fn layout_tree(tree: &SyntaxTree, ctx: LayoutContext) -> LayoutResult<MultiLayout> {
    let mut layouter = TreeLayouter::new(ctx);
    layouter.layout(tree)?;
    layouter.finish()
}

#[derive(Debug, Clone)]
struct TreeLayouter<'a, 'p> {
    ctx: LayoutContext<'a, 'p>,
    flex: FlexLayouter,
    style: TextStyle,
}

impl<'a, 'p> TreeLayouter<'a, 'p> {
    /// Create a new layouter.
    fn new(ctx: LayoutContext<'a, 'p>) -> TreeLayouter<'a, 'p> {
        TreeLayouter {
            flex: FlexLayouter::new(FlexContext {
                flex_spacing: flex_spacing(&ctx.text_style),
                spaces: ctx.spaces.clone(),
                axes: ctx.axes,
                shrink_to_fit: ctx.shrink_to_fit,
            }),
            style: ctx.text_style.clone(),
            ctx,
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
                    if !self.flex.box_is_empty() && !self.flex.last_is_space() {
                        let space = self.style.word_spacing * self.style.font_size;
                        self.flex.add_primary_space(space);
                    }
                }
                Node::Newline => {
                    if !self.flex.box_is_empty() {
                        self.break_paragraph()?;
                    }
                }

                Node::ToggleItalics => self.style.toggle_class(FontClass::Italic),
                Node::ToggleBold => self.style.toggle_class(FontClass::Bold),
                Node::ToggleMonospace => self.style.toggle_class(FontClass::Monospace),

                Node::Func(func) => self.layout_func(func)?,
            }
        }

        Ok(())
    }

    /// Layout a function.
    fn layout_func(&mut self, func: &FuncCall) -> LayoutResult<()> {
        let (flex_spaces, stack_spaces) = self.flex.remaining()?;

        let ctx = |spaces| LayoutContext {
            top_level: false,
            text_style: &self.style,
            spaces: spaces,
            shrink_to_fit: true,
            .. self.ctx
        };

        // Try putting it in the flex space first, but if that is not enough
        // space, use the other space.
        let commands = match func.body.val.layout(ctx(flex_spaces)) {
            Ok(c) => c,
            Err(LayoutError::NotEnoughSpace(_)) => {
                func.body.val.layout(ctx(stack_spaces))?
            },
            e => e?,
        };

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

            Command::AddPrimarySpace(space) => self.flex.add_primary_space(space),
            Command::AddSecondarySpace(space) => self.flex.add_secondary_space(space)?,

            Command::FinishRun => self.flex.add_run_break(),
            Command::FinishBox => self.flex.finish_box()?,
            Command::FinishLayout => self.flex.finish_layout(true)?,

            Command::BreakParagraph => self.break_paragraph()?,

            Command::SetTextStyle(style) => self.style = style,
            Command::SetPageStyle(style) => {
                if !self.ctx.top_level {
                    Err(LayoutError::Unallowed("can only set page style from top level"))?;
                }

                self.ctx.page_style = style;
                self.flex.set_spaces(smallvec![
                    LayoutSpace {
                        dimensions: style.dimensions,
                        padding: style.margins,
                    }
                ], true);
            },

            Command::SetAxes(axes) => {
                self.flex.set_axes(axes);
                self.ctx.axes = axes;
            }
        }

        Ok(())
    }

    /// Finish the layout.
    fn finish(self) -> LayoutResult<MultiLayout> {
        self.flex.finish()
    }

    /// Finish the current flex layout and add space after it.
    fn break_paragraph(&mut self) -> LayoutResult<()> {
        self.flex.finish_box()?;
        self.flex.add_secondary_space(paragraph_spacing(&self.style))?;
        Ok(())
    }
}

fn flex_spacing(style: &TextStyle) -> Size {
    (style.line_spacing - 1.0) * style.font_size
}

fn paragraph_spacing(style: &TextStyle) -> Size {
    (style.paragraph_spacing - 1.0) * style.font_size
}
