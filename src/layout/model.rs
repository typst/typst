//! The model layouter layouts models (i.e.
//! [syntax models](crate::syntax::SyntaxModel) and [functions](crate::func))
//! by executing commands issued by the models.

use std::future::Future;
use std::pin::Pin;
use smallvec::smallvec;
use toddle::query::{SharedFontLoader, FontProvider};

use crate::GlobalFontLoader;
use crate::error::Errors;
use crate::style::{LayoutStyle, PageStyle, TextStyle};
use crate::size::{Size, Size2D};
use crate::syntax::{Model, SyntaxModel, Node};
use crate::syntax::span::{Spanned, Span, offset_spans};
use super::line::{LineLayouter, LineContext};
use super::text::{layout_text, TextContext};
use super::*;


/// Performs the model layouting.
pub struct ModelLayouter<'a> {
    ctx: LayoutContext<'a>,
    layouter: LineLayouter,
    style: LayoutStyle,
    errors: Errors,
}

/// The context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext<'a> {
    /// The font loader to retrieve fonts from when typesetting text
    /// using [`layout_text`].
    pub loader: &'a GlobalFontLoader,
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

/// The result of layouting: Some layouted things and a list of errors.
pub struct Layouted<T> {
    /// The result of the layouting process.
    pub output: T,
    /// Errors that arose in the process of layouting.
    pub errors: Errors,
}

/// A sequence of layouting commands.
pub type Commands<'a> = Vec<Command<'a>>;

/// Commands issued to the layouting engine by models.
#[derive(Debug)]
pub enum Command<'a> {
    /// Layout the given model in the current context (i.e. not nested). The
    /// content of the model is not laid out into a separate box and then added,
    /// but simply laid out flat in the active layouting process.
    ///
    /// This has the effect that the content fits nicely into the active line
    /// layouting, enabling functions to e.g. change the style of some piece of
    /// text while keeping it integrated in the current paragraph.
    LayoutSyntaxModel(&'a SyntaxModel),

    /// Add a already computed layout.
    Add(Layout),
    /// Add multiple layouts, one after another. This is equivalent to multiple
    /// [Add](Command::Add) commands.
    AddMultiple(MultiLayout),

    /// Add spacing of given [kind](super::SpacingKind) along the primary or
    /// secondary axis. The spacing kind defines how the spacing interacts with
    /// surrounding spacing.
    AddSpacing(Size, SpacingKind, GenericAxis),

    /// Start a new line.
    BreakLine,
    /// Start a new paragraph.
    BreakParagraph,
    /// Start a new page, which will exist in the finished layout even if it
    /// stays empty (since the page break is a _hard_ space break).
    BreakPage,

    /// Update the text style.
    SetTextStyle(TextStyle),
    /// Update the page style.
    SetPageStyle(PageStyle),

    /// Update the alignment for future boxes added to this layouting process.
    SetAlignment(LayoutAlignment),
    /// Update the layouting axes along which future boxes will be laid out.
    /// This finishes the current line.
    SetAxes(LayoutAxes),
}

/// Layout a syntax model into a list of boxes.
pub async fn layout(model: &SyntaxModel, ctx: LayoutContext<'_>) -> Layouted<MultiLayout> {
    let mut layouter = ModelLayouter::new(ctx);
    layouter.layout_syntax_model(model).await;
    layouter.finish()
}

/// A dynamic future type which allows recursive invocation of async functions
/// when used as the return type. This is also how the async trait functions
/// work internally.
pub type DynFuture<'a, T> = Pin<Box<dyn Future<Output=T> + 'a>>;

impl<'a> ModelLayouter<'a> {
    /// Create a new model layouter.
    pub fn new(ctx: LayoutContext<'a>) -> ModelLayouter<'a> {
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

    /// Flatly layout a model into this layouting process.
    pub fn layout<'r>(
        &'r mut self,
        model: Spanned<&'r dyn Model>
    ) -> DynFuture<'r, ()> { Box::pin(async move {
        // Execute the model's layout function which generates the commands.
        let layouted = model.v.layout(LayoutContext {
            style: &self.style,
            spaces: self.layouter.remaining(),
            nested: true,
            debug: false,
            .. self.ctx
        }).await;

        // Add the errors generated by the model to the error list.
        self.errors.extend(offset_spans(layouted.errors, model.span.start));

        for command in layouted.output {
            self.execute_command(command, model.span).await;
        }
    }) }

    /// Layout a syntax model by directly processing the nodes instead of using
    /// the command based architecture.
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
                    self.style.text.fallback.flatten();
                }

                Node::Model(model) => {
                    self.layout(Spanned::new(model.as_ref(), node.span)).await;
                }
            }
        }
    }) }

    /// Compute the finished list of boxes.
    pub fn finish(self) -> Layouted<MultiLayout> {
        Layouted {
            output: self.layouter.finish(),
            errors: self.errors,
        }
    }

    /// Execute a command issued by a model. When the command is errorful, the
    /// given span is stored with the error.
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

            BreakLine => self.layouter.finish_line(),
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

                    // The line layouter has no idea of page styles and thus we
                    // need to recompute the layouting space resulting of the
                    // new page style and update it within the layouter.
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

    /// Layout a continous piece of text and add it to the line layouter.
    async fn layout_text(&mut self, text: &str) {
        self.layouter.add(layout_text(text, TextContext {
            loader: &self.ctx.loader,
            style: &self.style.text,
            axes: self.ctx.axes,
            alignment: self.ctx.alignment,
        }).await)
    }

    /// Add the spacing for a syntactic space node.
    fn layout_space(&mut self) {
        self.layouter.add_primary_spacing(
            self.style.text.word_spacing(),
            SpacingKind::WORD,
        );
    }

    /// Finish the paragraph and add paragraph spacing.
    fn layout_paragraph(&mut self) {
        self.layouter.add_secondary_spacing(
            self.style.text.paragraph_spacing(),
            SpacingKind::PARAGRAPH,
        );
    }
}
