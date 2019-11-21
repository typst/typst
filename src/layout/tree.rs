use super::*;
use smallvec::smallvec;

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
    /// Create a new syntax tree layouter.
    fn new(ctx: LayoutContext<'a, 'p>) -> TreeLayouter<'a, 'p> {
        TreeLayouter {
            flex: FlexLayouter::new(FlexContext {
                flex_spacing: flex_spacing(&ctx.text_style),
                spaces: ctx.spaces.clone(),
                axes: ctx.axes,
                expand: ctx.expand,
            }),
            style: ctx.text_style.clone(),
            ctx,
        }
    }

    fn layout(&mut self, tree: &SyntaxTree) -> LayoutResult<()> {
        for node in &tree.nodes {
            match &node.val {
                Node::Text(text) => self.layout_text(text)?,

                Node::Space => self.layout_space(),
                Node::Newline => self.layout_paragraph()?,

                Node::ToggleItalics => self.style.toggle_class(FontClass::Italic),
                Node::ToggleBold => self.style.toggle_class(FontClass::Bold),
                Node::ToggleMonospace => self.style.toggle_class(FontClass::Monospace),

                Node::Func(func) => self.layout_func(func)?,
            }
        }

        Ok(())
    }

    fn layout_text(&mut self, text: &str) -> LayoutResult<()> {
        let layout = layout_text(text, TextContext {
            loader: &self.ctx.loader,
            style: &self.style,
        })?;

        Ok(self.flex.add(layout))
    }

    fn layout_space(&mut self) {
        if !self.flex.run_is_empty() {
            self.flex.add_primary_space(word_spacing(&self.style), true);
        }
    }

    fn layout_paragraph(&mut self) -> LayoutResult<()> {
        if !self.flex.run_is_empty() {
            self.flex.add_secondary_space(paragraph_spacing(&self.style), true)?;
        }
        Ok(())
    }

    fn layout_func(&mut self, func: &FuncCall) -> LayoutResult<()> {
        let (first, second) = self.flex.remaining()?;

        let ctx = |spaces| {
            LayoutContext {
                loader: self.ctx.loader,
                top_level: false,
                text_style: &self.style,
                page_style: self.ctx.page_style,
                spaces,
                axes: self.ctx.axes.expanding(false),
                expand: false,
            }
        };

        let commands = match func.body.val.layout(ctx(first)) {
            Ok(c) => c,
            Err(e) => {
                match (e, second) {
                    (LayoutError::NotEnoughSpace(_), Some(space)) => {
                        func.body.val.layout(ctx(space))?
                    }
                    (e, _) => Err(e)?,
                }
            }
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

            Command::AddPrimarySpace(space) => self.flex.add_primary_space(space, false),
            Command::AddSecondarySpace(space) => self.flex.add_secondary_space(space, false)?,

            Command::FinishLine => self.flex.add_break(),
            Command::FinishRun => { self.flex.finish_run()?; },
            Command::FinishSpace => self.flex.finish_space(true)?,

            Command::BreakParagraph => self.layout_paragraph()?,

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

    fn finish(self) -> LayoutResult<MultiLayout> {
        self.flex.finish()
    }
}

fn word_spacing(style: &TextStyle) -> Size {
    style.word_spacing * style.font_size
}

fn flex_spacing(style: &TextStyle) -> Size {
    (style.line_spacing - 1.0) * style.font_size
}

fn paragraph_spacing(style: &TextStyle) -> Size {
    (style.paragraph_spacing - 1.0) * style.font_size
}
