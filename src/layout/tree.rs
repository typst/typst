//! Layouting of syntax trees.

use fontdock::FontStyle;

use super::*;
use crate::eval::Eval;
use crate::shaping;
use crate::syntax::*;
use crate::DynFuture;

/// Layout a syntax tree in a given context.
pub async fn layout_tree(tree: &SynTree, ctx: &mut LayoutContext) -> Vec<BoxLayout> {
    let mut layouter = TreeLayouter::new(ctx);
    layouter.layout_tree(tree).await;
    layouter.finish()
}

/// Layouts trees.
struct TreeLayouter<'a> {
    ctx: &'a mut LayoutContext,
    constraints: LayoutConstraints,
    layouter: LineLayouter,
}

impl<'a> TreeLayouter<'a> {
    fn new(ctx: &'a mut LayoutContext) -> Self {
        let layouter = LineLayouter::new(LineContext {
            spaces: ctx.constraints.spaces.clone(),
            dirs: ctx.state.dirs,
            repeat: ctx.constraints.repeat,
            line_spacing: ctx.state.text.line_spacing(),
        });

        Self {
            layouter,
            constraints: ctx.constraints.clone(),
            ctx,
        }
    }

    fn finish(self) -> Vec<BoxLayout> {
        self.layouter.finish()
    }

    fn layout_tree<'t>(&'t mut self, tree: &'t SynTree) -> DynFuture<'t, ()> {
        Box::pin(async move {
            for node in tree {
                self.layout_node(node).await;
            }
        })
    }

    async fn layout_node(&mut self, node: &Spanned<SynNode>) {
        let decorate = |this: &mut Self, deco: Deco| {
            this.ctx.f.decos.push(deco.span_with(node.span));
        };

        match &node.v {
            SynNode::Space => self.layout_space(),
            SynNode::Text(text) => {
                if self.ctx.state.text.emph {
                    decorate(self, Deco::Emph);
                }
                if self.ctx.state.text.strong {
                    decorate(self, Deco::Strong);
                }
                self.layout_text(text).await;
            }

            SynNode::Linebreak => self.layouter.finish_line(),
            SynNode::Parbreak => self.layout_parbreak(),
            SynNode::Emph => {
                self.ctx.state.text.emph ^= true;
                decorate(self, Deco::Emph);
            }
            SynNode::Strong => {
                self.ctx.state.text.strong ^= true;
                decorate(self, Deco::Strong);
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
            .add_cross_spacing(self.ctx.state.text.word_spacing(), SpacingKind::WORD);
    }

    fn layout_parbreak(&mut self) {
        self.layouter
            .add_main_spacing(self.ctx.state.text.par_spacing(), SpacingKind::PARAGRAPH);
    }

    async fn layout_text(&mut self, text: &str) {
        let mut variant = self.ctx.state.text.variant;

        if self.ctx.state.text.strong {
            variant.weight = variant.weight.thicken(300);
        }

        if self.ctx.state.text.emph {
            variant.style = match variant.style {
                FontStyle::Normal => FontStyle::Italic,
                FontStyle::Italic => FontStyle::Normal,
                FontStyle::Oblique => FontStyle::Normal,
            }
        }

        let boxed = shaping::shape(
            text,
            self.ctx.state.text.font_size(),
            self.ctx.state.dirs.cross,
            &mut self.ctx.loader.borrow_mut(),
            &self.ctx.state.text.fallback,
            variant,
        )
        .await;

        self.layouter.add(boxed, self.ctx.state.aligns);
    }

    async fn layout_heading(&mut self, heading: &NodeHeading) {
        let style = self.ctx.state.text.clone();

        let factor = 1.5 - 0.1 * heading.level.v as f64;
        self.ctx.state.text.font_size.scale *= factor;
        self.ctx.state.text.strong = true;

        self.layout_parbreak();
        self.layout_tree(&heading.contents).await;
        self.layout_parbreak();

        self.ctx.state.text = style;
    }

    async fn layout_raw(&mut self, raw: &NodeRaw) {
        if !raw.inline {
            self.layout_parbreak();
        }

        // TODO: Make this more efficient.
        let fallback = self.ctx.state.text.fallback.clone();
        self.ctx.state.text.fallback.list.insert(0, "monospace".to_string());
        self.ctx.state.text.fallback.flatten();

        let mut first = true;
        for line in &raw.lines {
            if !first {
                self.layouter.finish_line();
            }
            first = false;
            self.layout_text(line).await;
        }

        self.ctx.state.text.fallback = fallback;

        if !raw.inline {
            self.layout_parbreak();
        }
    }

    async fn layout_expr(&mut self, expr: Spanned<&Expr>) {
        self.ctx.constraints = LayoutConstraints {
            root: false,
            base: self.constraints.base,
            spaces: self.layouter.remaining(),
            repeat: self.constraints.repeat,
        };

        let val = expr.v.eval(self.ctx).await;
        let commands = val.span_with(expr.span).into_commands();
        for command in commands {
            self.execute_command(command, expr.span).await;
        }
    }

    async fn execute_command(&mut self, command: Command, span: Span) {
        use Command::*;
        match command {
            LayoutSyntaxTree(tree) => self.layout_tree(&tree).await,

            Add(layout, aligns) => self.layouter.add(layout, aligns),
            AddSpacing(space, kind, axis) => match axis {
                GenAxis::Main => self.layouter.add_main_spacing(space, kind),
                GenAxis::Cross => self.layouter.add_cross_spacing(space, kind),
            },

            BreakLine => self.layouter.finish_line(),
            BreakPage => {
                if self.constraints.root {
                    self.layouter.finish_space(true)
                } else {
                    self.ctx.diag(error!(
                        span,
                        "page break can only be issued from root context",
                    ));
                }
            }

            SetTextState(style) => {
                self.layouter.set_line_spacing(style.line_spacing());
                self.ctx.state.text = style;
            }
            SetPageState(style) => {
                if self.constraints.root {
                    self.ctx.state.page = style;

                    // The line layouter has no idea of page styles and thus we
                    // need to recompute the layouting space resulting of the
                    // new page style and update it within the layouter.
                    let space = LayoutSpace {
                        size: style.size,
                        insets: style.insets(),
                        expansion: Spec2::new(true, true),
                    };
                    self.constraints.base = space.usable();
                    self.layouter.set_spaces(vec![space], true);
                } else {
                    self.ctx.diag(error!(
                        span,
                        "page style can only be changed from root context",
                    ));
                }
            }
            SetAlignment(aligns) => self.ctx.state.aligns = aligns,
        }
    }
}
