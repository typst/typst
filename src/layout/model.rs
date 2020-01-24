use smallvec::smallvec;

use crate::error::Errors;
use crate::func::Command;
use crate::syntax::{Model, DynFuture, SyntaxModel, Node};
use crate::syntax::{SpanVec, Spanned, Span, offset_spans};
use super::*;


#[derive(Debug, Clone)]
pub struct ModelLayouter<'a, 'p> {
    ctx: LayoutContext<'a, 'p>,
    layouter: LineLayouter,
    style: LayoutStyle,
    errors: Errors,
}

impl<'a, 'p> ModelLayouter<'a, 'p> {
    /// Create a new syntax tree layouter.
    pub fn new(ctx: LayoutContext<'a, 'p>) -> ModelLayouter<'a, 'p> {
        ModelLayouter {
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
            errors: vec![],
        }
    }

    pub fn layout<'r>(
        &'r mut self,
        model: Spanned<&'r dyn Model>
    ) -> DynFuture<'r, ()> { Box::pin(async move {
        let layouted = model.v.layout(LayoutContext {
            style: &self.style,
            spaces: self.layouter.remaining(),
            nested: true,
            debug: false,
            .. self.ctx
        }).await;

        let commands = layouted.output;
        self.errors.extend(offset_spans(layouted.errors, model.span.start));

        for command in commands {
            self.execute_command(command, model.span).await;
        }
    }) }

    pub fn layout_syntax_model<'r>(
        &'r mut self,
        model: &'r SyntaxModel
    ) -> DynFuture<'r, ()> { Box::pin(async move {
        use Node::*;

        for node in &model.nodes {
            match &node.v {
                Space => self.layout_space(),
                Newline => self.layout_paragraph(),
                Text(text) => self.layout_text(text).await,

                ToggleItalic => self.style.text.variant.style.toggle(),
                ToggleBolder => {
                    let fac = if self.style.text.bolder { -1 } else { 1 };
                    self.style.text.variant.weight.0 += 300 * fac;
                    self.style.text.bolder = !self.style.text.bolder;
                }
                ToggleMonospace => {
                    let list = &mut self.style.text.fallback.list;
                    match list.get(0).map(|s| s.as_str()) {
                        Some("monospace") => { list.remove(0); },
                        _ => list.insert(0, "monospace".to_string()),
                    }
                }

                Node::Model(model) => {
                    self.layout(Spanned::new(model.as_ref(), node.span)).await;
                }
            }
        }
    }) }

    pub fn finish(self) -> Layouted<MultiLayout> {
        Layouted {
            output: self.layouter.finish(),
            errors: self.errors,
        }
    }

    fn execute_command<'r>(
        &'r mut self,
        command: Command<'r>,
        span: Span,
    ) -> DynFuture<'r, ()> { Box::pin(async move {
        use Command::*;

        match command {
            LayoutSyntaxModel(model) => self.layout_syntax_model(model).await,

            Add(layout) => self.layouter.add(layout),
            AddMultiple(layouts) => self.layouter.add_multiple(layouts),
            AddSpacing(space, kind, axis) => match axis {
                Primary => self.layouter.add_primary_spacing(space, kind),
                Secondary => self.layouter.add_secondary_spacing(space, kind),
            }

            FinishLine => self.layouter.finish_line(),
            FinishSpace => self.layouter.finish_space(true),
            BreakParagraph => self.layout_paragraph(),
            BreakPage => {
                if self.ctx.nested {
                    self.errors.push(err!(span;
                        "page break cannot be issued from nested context"));
                } else {
                    self.layouter.finish_space(true)
                }
            }

            SetTextStyle(style) => {
                self.layouter.set_line_spacing(style.line_spacing());
                self.style.text = style;
            }
            SetPageStyle(style) => {
                if self.ctx.nested {
                    self.errors.push(err!(span;
                        "page style cannot be changed from nested context"));
                } else {
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
            }

            SetAlignment(alignment) => self.ctx.alignment = alignment,
            SetAxes(axes) => {
                self.layouter.set_axes(axes);
                self.ctx.axes = axes;
            }
        }
    }) }

    async fn layout_text(&mut self, text: &str) {
        self.layouter.add(layout_text(text, TextContext {
            loader: &self.ctx.loader,
            style: &self.style.text,
            axes: self.ctx.axes,
            alignment: self.ctx.alignment,
        }).await)
    }

    fn layout_space(&mut self) {
        self.layouter.add_primary_spacing(
            self.style.text.word_spacing(),
            SpacingKind::WORD,
        );
    }

    fn layout_paragraph(&mut self) {
        self.layouter.add_secondary_spacing(
            self.style.text.paragraph_spacing(),
            SpacingKind::PARAGRAPH,
        );
    }
}
