//! The model layouter layouts models (i.e.
//! [syntax models](crate::syntax::SyntaxModel) and [functions](crate::func))
//! by executing commands issued by the models.

use std::future::Future;
use std::pin::Pin;
use smallvec::smallvec;

use crate::{Pass, Feedback};
use crate::SharedFontLoader;
use crate::style::{LayoutStyle, PageStyle, TextStyle};
use crate::geom::Size;
use crate::syntax::decoration::Decoration;
use crate::syntax::model::{Model, SyntaxModel, Node};
use crate::syntax::span::{Span, Spanned};
use super::line::{LineLayouter, LineContext};
use super::text::{layout_text, TextContext};
use super::*;

/// Performs the model layouting.
#[derive(Debug)]
pub struct ModelLayouter<'a> {
    ctx: LayoutContext<'a>,
    layouter: LineLayouter,
    style: LayoutStyle,
    feedback: Feedback,
}

/// The context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext<'a> {
    /// The font loader to retrieve fonts from when typesetting text
    /// using [`layout_text`].
    pub loader: &'a SharedFontLoader,
    /// The style for pages and text.
    pub style: &'a LayoutStyle,
    /// The base unpadded dimensions of this container (for relative sizing).
    pub base: Size,
    /// The spaces to layout in.
    pub spaces: LayoutSpaces,
    /// Whether to have repeated spaces or to use only the first and only once.
    pub repeat: bool,
    /// The initial axes along which content is laid out.
    pub axes: LayoutAxes,
    /// The alignment of the finished layout.
    pub align: LayoutAlign,
    /// Whether the layout that is to be created will be nested in a parent
    /// container.
    pub nested: bool,
    /// Whether to render debug boxs around layouts if `nested` is true.
    pub debug: bool,
}

/// A sequence of layouting commands.
pub type Commands<'a> = Vec<Command<'a>>;

/// Commands issued to the layouting engine by models.
#[derive(Debug, Clone)]
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
    AddSpacing(f64, SpacingKind, GenAxis),

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
    SetAlignment(LayoutAlign),
    /// Update the layouting axes along which future boxes will be laid
    /// out. This finishes the current line.
    SetAxes(LayoutAxes),
}

/// Layout a syntax model into a list of boxes.
pub async fn layout(model: &SyntaxModel, ctx: LayoutContext<'_>) -> Pass<MultiLayout> {
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
                align: ctx.align,
                repeat: ctx.repeat,
                debug: ctx.debug && ctx.nested,
                line_spacing: ctx.style.text.line_spacing(),
            }),
            style: ctx.style.clone(),
            ctx,
            feedback: Feedback::new(),
        }
    }

    /// Flatly layout a model into this layouting process.
    pub async fn layout<'r>(
        &'r mut self,
        model: Spanned<&'r dyn Model>
    ) {
        // Execute the model's layout function which generates the commands.
        let layouted = model.v.layout(LayoutContext {
            style: &self.style,
            spaces: self.layouter.remaining(),
            nested: true,
            .. self.ctx
        }).await;

        // Add the errors generated by the model to the error list.
        self.feedback.extend_offset(layouted.feedback, model.span.start);

        for command in layouted.output {
            self.execute_command(command, model.span).await;
        }
    }

    /// Layout a syntax model by directly processing the nodes instead of using
    /// the command based architecture.
    pub async fn layout_syntax_model<'r>(
        &'r mut self,
        model: &'r SyntaxModel
    ) {
        use Node::*;

        for Spanned { v: node, span } in &model.nodes {
            let decorate = |this: &mut ModelLayouter, deco| {
                this.feedback.decorations.push(Spanned::new(deco, *span));
            };

            match node {
                Space => self.layout_space(),
                Parbreak => self.layout_paragraph(),
                Linebreak => self.layouter.finish_line(),

                Text(text) => {
                    if self.style.text.italic {
                        decorate(self, Decoration::Italic);
                    }

                    if self.style.text.bolder {
                        decorate(self, Decoration::Bold);
                    }

                    self.layout_text(text).await;
                }

                ToggleItalic => {
                    self.style.text.italic = !self.style.text.italic;
                    decorate(self, Decoration::Italic);
                }

                ToggleBolder => {
                    self.style.text.bolder = !self.style.text.bolder;
                    decorate(self, Decoration::Bold);
                }

                Raw(lines) => {
                    // TODO: Make this more efficient.
                    let fallback = self.style.text.fallback.clone();
                    self.style.text.fallback.list_mut().insert(0, "monospace".to_string());
                    self.style.text.fallback.flatten();

                    // Layout the first line.
                    let mut iter = lines.iter();
                    if let Some(line) = iter.next() {
                        self.layout_text(line).await;
                    }

                    // Put a newline before each following line.
                    for line in iter {
                        self.layouter.finish_line();
                        self.layout_text(line).await;
                    }

                    self.style.text.fallback = fallback;
                }

                Model(model) => {
                    self.layout(Spanned::new(model.as_ref(), *span)).await;
                }
            }
        }
    }

    /// Compute the finished list of boxes.
    pub fn finish(self) -> Pass<MultiLayout> {
        Pass::new(self.layouter.finish(), self.feedback)
    }

    /// Execute a command issued by a model. When the command is errorful, the
    /// given span is stored with the error.
    fn execute_command<'r>(
        &'r mut self,
        command: Command<'r>,
        model_span: Span,
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
                    error!(
                        @self.feedback, model_span,
                        "page break cannot be issued from nested context",
                    );
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
                    error!(
                        @self.feedback, model_span,
                        "page style cannot be changed from nested context",
                    );
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

            SetAlignment(align) => self.ctx.align = align,
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
            align: self.ctx.align,
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
