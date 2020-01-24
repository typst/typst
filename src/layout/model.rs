use std::future::Future;
use std::pin::Pin;
use smallvec::smallvec;
use toddle::query::SharedFontLoader;

use crate::error::Errors;
use crate::style::{LayoutStyle, PageStyle, TextStyle};
use crate::size::{Size, Size2D};
use crate::syntax::{Model, SyntaxModel, Node};
use crate::syntax::span::{Spanned, Span, offset_spans};
use super::line::{LineLayouter, LineContext};
use super::text::{layout_text, TextContext};
use super::*;


#[derive(Debug, Clone)]
pub struct ModelLayouter<'a, 'p> {
    ctx: LayoutContext<'a, 'p>,
    layouter: LineLayouter,
    style: LayoutStyle,
    errors: Errors,
}

/// The general context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext<'a, 'p> {
    /// The font loader to retrieve fonts from when typesetting text
    /// using [`layout_text`].
    pub loader: &'a SharedFontLoader<'p>,
    /// The style for pages and text.
    pub style: &'a LayoutStyle,
    /// The base unpadded dimensions of this container (for relative sizing).
    pub base: Size2D,
    /// The spaces to layout in.
    pub spaces: LayoutSpaces,
    /// Whether to have repeated spaces or to use only the first and only once.
    pub repeat: bool,
    /// The initial axes along which content is laid out.
    pub axes: LayoutAxes,
    /// The alignment of the finished layout.
    pub alignment: LayoutAlignment,
    /// Whether the layout that is to be created will be nested in a parent
    /// container.
    pub nested: bool,
    /// Whether to debug render a box around the layout.
    pub debug: bool,
}

pub struct Layouted<T> {
    pub output: T,
    pub errors: Errors,
}

impl<T> Layouted<T> {
    pub fn map<F, U>(self, f: F) -> Layouted<U> where F: FnOnce(T) -> U {
        Layouted {
            output: f(self.output),
            errors: self.errors,
        }
    }
}

/// A sequence of layouting commands.
pub type Commands<'a> = Vec<Command<'a>>;

/// Layouting commands from functions to the typesetting engine.
#[derive(Debug)]
pub enum Command<'a> {
    LayoutSyntaxModel(&'a SyntaxModel),

    Add(Layout),
    AddMultiple(MultiLayout),
    AddSpacing(Size, SpacingKind, GenericAxis),

    FinishLine,
    FinishSpace,
    BreakParagraph,
    BreakPage,

    SetTextStyle(TextStyle),
    SetPageStyle(PageStyle),
    SetAlignment(LayoutAlignment),
    SetAxes(LayoutAxes),
}

pub async fn layout(model: &SyntaxModel, ctx: LayoutContext<'_, '_>) -> Layouted<MultiLayout> {
    let mut layouter = ModelLayouter::new(ctx);
    layouter.layout_syntax_model(model).await;
    layouter.finish()
}

pub type DynFuture<'a, T> = Pin<Box<dyn Future<Output=T> + 'a>>;

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
