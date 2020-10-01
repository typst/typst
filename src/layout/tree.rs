//! Layouting of syntax trees.

use super::line::{LineContext, LineLayouter};
use super::text::{layout_text, TextContext};
use super::*;
use crate::style::LayoutStyle;
use crate::syntax::{
    Decoration, Expr, NodeHeading, NodeRaw, Span, SpanWith, Spanned, SynNode, SynTree,
};
use crate::{DynFuture, Feedback, Pass};

/// Layout a syntax tree into a collection of boxes.
pub async fn layout_tree(tree: &SynTree, ctx: LayoutContext<'_>) -> Pass<MultiLayout> {
    let mut layouter = TreeLayouter::new(ctx);
    layouter.layout_tree(tree).await;
    layouter.finish()
}

/// Performs the tree layouting.
struct TreeLayouter<'a> {
    ctx: LayoutContext<'a>,
    layouter: LineLayouter,
    style: LayoutStyle,
    feedback: Feedback,
}

impl<'a> TreeLayouter<'a> {
    fn new(ctx: LayoutContext<'a>) -> Self {
        Self {
            layouter: LineLayouter::new(LineContext {
                spaces: ctx.spaces.clone(),
                axes: ctx.axes,
                align: ctx.align,
                repeat: ctx.repeat,
                line_spacing: ctx.style.text.line_spacing(),
            }),
            style: ctx.style.clone(),
            ctx,
            feedback: Feedback::new(),
        }
    }

    fn finish(self) -> Pass<MultiLayout> {
        Pass::new(self.layouter.finish(), self.feedback)
    }

    fn layout_tree<'t>(&'t mut self, tree: &'t SynTree) -> DynFuture<'t, ()> {
        Box::pin(async move {
            for node in tree {
                self.layout_node(node).await;
            }
        })
    }

    async fn layout_node(&mut self, node: &Spanned<SynNode>) {
        let decorate = |this: &mut Self, deco: Decoration| {
            this.feedback.decorations.push(deco.span_with(node.span));
        };

        match &node.v {
            SynNode::Spacing => self.layout_space(),
            SynNode::Linebreak => self.layouter.finish_line(),
            SynNode::Parbreak => self.layout_parbreak(),

            SynNode::ToggleItalic => {
                self.style.text.italic = !self.style.text.italic;
                decorate(self, Decoration::Italic);
            }
            SynNode::ToggleBolder => {
                self.style.text.bolder = !self.style.text.bolder;
                decorate(self, Decoration::Bold);
            }

            SynNode::Text(text) => {
                if self.style.text.italic {
                    decorate(self, Decoration::Italic);
                }
                if self.style.text.bolder {
                    decorate(self, Decoration::Bold);
                }
                self.layout_text(text).await;
            }

            SynNode::Raw(raw) => self.layout_raw(raw).await,
            SynNode::Heading(heading) => self.layout_heading(heading).await,

            SynNode::Expr(expr) => {
                self.layout_expr(expr.span_with(node.span)).await;
            }
        }
    }

    fn layout_space(&mut self) {
        self.layouter
            .add_primary_spacing(self.style.text.word_spacing(), SpacingKind::WORD);
    }

    fn layout_parbreak(&mut self) {
        self.layouter.add_secondary_spacing(
            self.style.text.paragraph_spacing(),
            SpacingKind::PARAGRAPH,
        );
    }

    async fn layout_text(&mut self, text: &str) {
        self.layouter.add(
            layout_text(text, TextContext {
                loader: &mut self.ctx.loader.borrow_mut(),
                style: &self.style.text,
                dir: self.ctx.axes.primary,
                align: self.ctx.align,
            })
            .await,
        );
    }

    async fn layout_heading(&mut self, heading: &NodeHeading) {
        let style = self.style.text.clone();
        self.style.text.font_scale *= 1.5 - 0.1 * heading.level.v.min(5) as f64;
        self.style.text.bolder = true;

        self.layout_parbreak();
        self.layout_tree(&heading.contents).await;
        self.layout_parbreak();

        self.style.text = style;
    }

    async fn layout_raw(&mut self, raw: &NodeRaw) {
        if !raw.inline {
            self.layout_parbreak();
        }

        // TODO: Make this more efficient.
        let fallback = self.style.text.fallback.clone();
        self.style.text.fallback.list.insert(0, "monospace".to_string());
        self.style.text.fallback.flatten();

        let mut first = true;
        for line in &raw.lines {
            if !first {
                self.layouter.finish_line();
            }
            first = false;
            self.layout_text(line).await;
        }

        self.style.text.fallback = fallback;

        if !raw.inline {
            self.layout_parbreak();
        }
    }

    async fn layout_expr(&mut self, expr: Spanned<&Expr>) {
        let ctx = LayoutContext {
            style: &self.style,
            spaces: self.layouter.remaining(),
            root: false,
            ..self.ctx
        };

        let val = expr.v.eval(&ctx, &mut self.feedback).await;
        let commands = val.span_with(expr.span).into_commands();

        for command in commands {
            self.execute_command(command, expr.span).await;
        }
    }

    async fn execute_command(&mut self, command: Command, span: Span) {
        use Command::*;

        match command {
            LayoutSyntaxTree(tree) => self.layout_tree(&tree).await,

            Add(layout) => self.layouter.add(layout),
            AddMultiple(layouts) => self.layouter.add_multiple(layouts),
            AddSpacing(space, kind, axis) => match axis {
                Primary => self.layouter.add_primary_spacing(space, kind),
                Secondary => self.layouter.add_secondary_spacing(space, kind),
            },

            BreakLine => self.layouter.finish_line(),
            BreakPage => {
                if self.ctx.root {
                    self.layouter.finish_space(true)
                } else {
                    error!(
                        @self.feedback, span,
                        "page break cannot only be issued from root context",
                    );
                }
            }

            SetTextStyle(style) => {
                self.layouter.set_line_spacing(style.line_spacing());
                self.style.text = style;
            }
            SetPageStyle(style) => {
                if self.ctx.root {
                    self.style.page = style;

                    // The line layouter has no idea of page styles and thus we
                    // need to recompute the layouting space resulting of the
                    // new page style and update it within the layouter.
                    let margins = style.margins();
                    self.ctx.base = style.size.unpadded(margins);
                    self.layouter.set_spaces(
                        vec![LayoutSpace {
                            size: style.size,
                            padding: margins,
                            expansion: LayoutExpansion::new(true, true),
                        }],
                        true,
                    );
                } else {
                    error!(
                        @self.feedback, span,
                        "page style cannot only be changed from root context",
                    );
                }
            }

            SetAlignment(align) => self.ctx.align = align,
            SetAxes(axes) => {
                self.layouter.set_axes(axes);
                self.ctx.axes = axes;
            }
        }
    }
}
