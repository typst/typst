use smallvec::smallvec;

use crate::func::Command;
use crate::syntax::{SyntaxTree, Node, FuncCall};
use crate::style::TextStyle;
use super::*;

/// Layout a syntax tree into a multibox.
pub fn layout(tree: &SyntaxTree, ctx: LayoutContext) -> LayoutResult<MultiLayout> {
    let mut layouter = TreeLayouter::new(ctx);
    layouter.layout(tree)?;
    layouter.finish()
}

#[derive(Debug, Clone)]
struct TreeLayouter<'a, 'p> {
    ctx: LayoutContext<'a, 'p>,
    layouter: LineLayouter,
    style: LayoutStyle,
}

impl<'a, 'p> TreeLayouter<'a, 'p> {
    /// Create a new syntax tree layouter.
    fn new(ctx: LayoutContext<'a, 'p>) -> TreeLayouter<'a, 'p> {
        TreeLayouter {
            layouter: LineLayouter::new(LineContext {
                spaces: ctx.spaces.clone(),
                axes: ctx.axes,
                alignment: ctx.alignment,
                repeat: ctx.repeat,
                debug: ctx.debug,
                line_spacing: ctx.style.text.line_spacing(),
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

                Node::ToggleItalics => {},
                Node::ToggleBold => {},
                Node::ToggleMonospace => {},

                Node::Func(func) => self.layout_func(func)?,
            }
        }

        Ok(())
    }

    fn layout_text(&mut self, text: &str) -> LayoutResult<()> {
        let layout = layout_text(text, TextContext {
            loader: &self.ctx.loader,
            style: &self.style.text,
            axes: self.ctx.axes,
            alignment: self.ctx.alignment,
        })?;

        self.layouter.add(layout)
    }

    fn layout_space(&mut self) {
        self.layouter.add_primary_spacing(self.style.text.word_spacing(), WORD_KIND);
    }

    fn layout_paragraph(&mut self) -> LayoutResult<()> {
        self.layouter.add_secondary_spacing(self.style.text.paragraph_spacing(), PARAGRAPH_KIND)
    }

    fn layout_func(&mut self, func: &FuncCall) -> LayoutResult<()> {
        let commands = func.0.layout(LayoutContext {
            style: &self.style,
            spaces: self.layouter.remaining(),
            nested: true,
            debug: false,
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

            Add(layout) => self.layouter.add(layout)?,
            AddMultiple(layouts) => self.layouter.add_multiple(layouts)?,
            AddSpacing(space, kind, axis) => match axis {
                Primary => self.layouter.add_primary_spacing(space, kind),
                Secondary => self.layouter.add_secondary_spacing(space, kind)?,
            }

            FinishLine => self.layouter.finish_line()?,
            FinishSpace => self.layouter.finish_space(true)?,
            BreakParagraph => self.layout_paragraph()?,
            BreakPage => {
                if self.ctx.nested {
                    error!("page break cannot be issued from nested context");
                }

                self.layouter.finish_space(true)?
            }

            SetTextStyle(style) => {
                self.layouter.set_line_spacing(style.line_spacing());
                self.style.text = style;
            }
            SetPageStyle(style) => {
                if self.ctx.nested {
                    error!("page style cannot be altered in nested context");
                }

                self.style.page = style;

                let margins = style.margins();
                self.ctx.base = style.dimensions.unpadded(margins);
                self.layouter.set_spaces(smallvec![
                    LayoutSpace {
                        dimensions: style.dimensions,
                        padding: margins,
                        expansion: LayoutExpansion::new(true, true),
                    }
                ], true);
            }
            SetAlignment(alignment) => self.ctx.alignment = alignment,
            SetAxes(axes) => {
                self.layouter.set_axes(axes);
                self.ctx.axes = axes;
            }
        }

        Ok(())
    }

    fn finish(self) -> LayoutResult<MultiLayout> {
        self.layouter.finish()
    }
}
