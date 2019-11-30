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
    style: LayoutStyle,
}

impl<'a, 'p> TreeLayouter<'a, 'p> {
    /// Create a new syntax tree layouter.
    fn new(ctx: LayoutContext<'a, 'p>) -> TreeLayouter<'a, 'p> {
        TreeLayouter {
            flex: FlexLayouter::new(FlexContext {
                flex_spacing: flex_spacing(&ctx.style.text),
                spaces: ctx.spaces.clone(),
                axes: ctx.axes,
                expand: ctx.expand,
            }),
            style: ctx.style.clone(),
            ctx,
        }
    }

    fn layout(&mut self, tree: &SyntaxTree) -> LayoutResult<()> {
        for node in &tree.nodes {
            match &node.val {
                Node::Text(text) => self.layout_text(text)?,

                Node::Space => self.layout_space(),
                Node::Newline => self.layout_paragraph()?,

                Node::ToggleItalics => self.style.text.toggle_class(FontClass::Italic),
                Node::ToggleBold => self.style.text.toggle_class(FontClass::Bold),
                Node::ToggleMonospace => self.style.text.toggle_class(FontClass::Monospace),

                Node::Func(func) => self.layout_func(func)?,
            }
        }

        Ok(())
    }

    fn layout_text(&mut self, text: &str) -> LayoutResult<()> {
        let layout = layout_text(text, TextContext {
            loader: &self.ctx.loader,
            style: &self.style.text,
        })?;

        Ok(self.flex.add(layout))
    }

    fn layout_space(&mut self) {
        self.flex.add_primary_space(
            word_spacing(&self.style.text),
            SPACE_KIND,
        );
    }

    fn layout_paragraph(&mut self) -> LayoutResult<()> {
        self.flex.add_secondary_space(
            paragraph_spacing(&self.style.text),
            PARAGRAPH_KIND,
        )
    }

    fn layout_func(&mut self, func: &FuncCall) -> LayoutResult<()> {
        let spaces = self.flex.remaining();

        let commands = func.body.val.layout(LayoutContext {
            loader: self.ctx.loader,
            style: &self.style,
            top_level: false,
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

            Command::AddPrimarySpace(space) => self.flex.add_primary_space(space, SpacingKind::Hard),
            Command::AddSecondarySpace(space) => self.flex.add_secondary_space(space, SpacingKind::Hard)?,

            Command::FinishLine => self.flex.add_break(),
            Command::FinishRun => { self.flex.finish_run()?; },
            Command::FinishSpace => self.flex.finish_space(true)?,

            Command::BreakParagraph => self.layout_paragraph()?,

            Command::SetTextStyle(style) => self.style.text = style,
            Command::SetPageStyle(style) => {
                if !self.ctx.top_level {
                    lerr!("page style cannot only be altered in the top-level context");
                }

                self.style.page = style;
                self.flex.set_spaces(smallvec![
                    LayoutSpace {
                        dimensions: style.dimensions,
                        expand: (true, true),
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
