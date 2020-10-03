//! Layouting of syntax trees.

use super::line::{LineContext, LineLayouter};
use super::shaping::{shape, ShapeOptions};
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
                sys: ctx.sys,
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
            SynNode::Space => self.layout_space(),
            SynNode::Text(text) => {
                if self.style.text.emph {
                    decorate(self, Decoration::Emph);
                }
                if self.style.text.strong {
                    decorate(self, Decoration::Strong);
                }
                self.layout_text(text).await;
            }

            SynNode::Linebreak => self.layouter.finish_line(),
            SynNode::Parbreak => self.layout_parbreak(),
            SynNode::Emph => {
                self.style.text.emph = !self.style.text.emph;
                decorate(self, Decoration::Emph);
            }
            SynNode::Strong => {
                self.style.text.strong = !self.style.text.strong;
                decorate(self, Decoration::Strong);
            }

            SynNode::Heading(heading) => self.layout_heading(heading).await,
            SynNode::Raw(raw) => self.layout_raw(raw).await,

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
            shape(text, ShapeOptions {
                loader: &mut self.ctx.loader.borrow_mut(),
                style: &self.style.text,
                dir: self.ctx.sys.primary,
                align: self.ctx.align,
            })
            .await,
        );
    }

    async fn layout_heading(&mut self, heading: &NodeHeading) {
        let style = self.style.text.clone();
        self.style.text.font_scale *= 1.5 - 0.1 * heading.level.v as f64;
        self.style.text.strong = true;

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
                GenAxis::Primary => self.layouter.add_primary_spacing(space, kind),
                GenAxis::Secondary => self.layouter.add_secondary_spacing(space, kind),
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
                    let space = LayoutSpace {
                        size: style.size,
                        insets: style.insets(),
                        expansion: LayoutExpansion::new(true, true),
                    };
                    self.ctx.base = space.usable();
                    self.layouter.set_spaces(vec![space], true);
                } else {
                    error!(
                        @self.feedback, span,
                        "page style cannot only be changed from root context",
                    );
                }
            }

            SetAlignment(align) => self.ctx.align = align,
            SetSystem(sys) => {
                self.layouter.set_sys(sys);
                self.ctx.sys = sys;
            }
        }
    }
}
