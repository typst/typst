use smallvec::smallvec;
use super::*;

/// Layout a syntax tree into a multibox.
pub fn layout_tree(tree: &SyntaxTree, ctx: LayoutContext) -> LayoutResult<MultiLayout> {
    let mut layouter = TreeLayouter::new(ctx);
    layouter.layout(tree)?;
    layouter.finish()
}

#[derive(Debug, Clone)]
struct TreeLayouter<'a, 'p> {
    ctx: LayoutContext<'a, 'p>,
    stack: StackLayouter,
    style: LayoutStyle,
}

impl<'a, 'p> TreeLayouter<'a, 'p> {
    /// Create a new syntax tree layouter.
    fn new(ctx: LayoutContext<'a, 'p>) -> TreeLayouter<'a, 'p> {
        TreeLayouter {
            stack: StackLayouter::new(StackContext {
                spaces: ctx.spaces.clone(),
                axes: ctx.axes,
                alignment: ctx.alignment,
            }),
            style: ctx.style.clone(),
            ctx,
        }
    }

    fn layout(&mut self, tree: &SyntaxTree) -> LayoutResult<()> {
        for node in &tree.nodes {
            match &node.v {
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
            alignment: self.ctx.alignment,
        })?;

        self.stack.add(layout)
    }

    fn layout_space(&mut self) {

    }

    fn layout_paragraph(&mut self) -> LayoutResult<()> {
        Ok(self.stack.add_spacing(
            paragraph_spacing(&self.style.text),
            PARAGRAPH_KIND,
        ))
    }

    fn layout_func(&mut self, func: &FuncCall) -> LayoutResult<()> {
        let spaces = self.stack.remaining();

        let commands = func.0.layout(LayoutContext {
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
        use Command::*;

        match command {
            LayoutTree(tree) => self.layout(tree)?,

            Add(layout) => self.stack.add(layout)?,
            AddMultiple(layouts) => self.stack.add_multiple(layouts)?,
            AddSpacing(space, kind, axis) => match axis {
                GenericAxisKind::Primary => {},
                GenericAxisKind::Secondary => self.stack.add_spacing(space, kind),
            }

            FinishLine => {},
            FinishRun => {},
            FinishSpace => self.stack.finish_space(true),
            BreakParagraph => self.layout_paragraph()?,

            SetTextStyle(style) => self.style.text = style,
            SetPageStyle(style) => {
                if !self.ctx.top_level {
                    error!("the page style cannot only be altered from a top-level context");
                }

                self.style.page = style;
                self.stack.set_spaces(smallvec![
                    LayoutSpace {
                        dimensions: style.dimensions,
                        padding: style.margins,
                        expand: (true, true),
                    }
                ], true);
            }
            SetAlignment(alignment) => self.ctx.alignment = alignment,
            SetAxes(axes) => {
                self.stack.set_axes(axes);
                self.ctx.axes = axes;
            }
        }

        Ok(())
    }

    fn finish(self) -> LayoutResult<MultiLayout> {
        Ok(self.stack.finish())
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
