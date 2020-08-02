//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens](crate::syntax::Tokens). Then, a parser constructs a
//!   syntax tree from the token stream. The structures describing the tree can
//!   be found in the [syntax](crate::syntax) module.
//! - **Layouting:** The next step is to transform the syntax tree into a
//!   portable representation of the typesetted document. Types for these can be
//!   found in the [layout](crate::layout) module. A finished layout reading for
//!   exporting is a [MultiLayout](crate::layout::MultiLayout) consisting of
//!   multiple boxes (or pages).
//! - **Exporting:** The finished layout can then be exported into a supported
//!   format. Submodules for these formats are located in the
//!   [export](crate::export) module. Currently, the only supported output
//!   format is [_PDF_](crate::export::pdf). Alternatively, the layout can be
//!   serialized to pass it to a suitable renderer.

use std::fmt::Debug;

use crate::diagnostic::Diagnostics;
use crate::font::SharedFontLoader;
use crate::layout::MultiLayout;
use crate::style::{LayoutStyle, PageStyle, TextStyle};
use crate::syntax::decoration::Decorations;
use crate::syntax::model::SyntaxModel;
use crate::syntax::parsing::{parse, ParseState};
use crate::syntax::scope::Scope;
use crate::syntax::span::{Offset, Pos};

/// Declare a module and reexport all its contents.
macro_rules! pub_use_mod {
    ($name:ident) => {
        mod $name;
        pub use $name::*;
    };
}

#[macro_use]
mod macros;
#[macro_use]
pub mod diagnostic;
pub mod export;
pub mod font;
#[macro_use]
pub mod func;
pub mod geom;
pub mod layout;
pub mod library;
pub mod length;
pub mod paper;
pub mod style;
pub mod syntax;

/// Transforms source code into typesetted layouts.
///
/// A typesetter can be configured through various methods.
pub struct Typesetter {
    /// The font loader shared by all typesetting processes.
    loader: SharedFontLoader,
    /// The base layouting style.
    style: LayoutStyle,
    /// The base parser state.
    parse_state: ParseState,
    /// Whether to render debug boxes.
    debug: bool,
}

impl Typesetter {
    /// Create a new typesetter.
    pub fn new(loader: SharedFontLoader) -> Typesetter {
        Typesetter {
            loader,
            style: LayoutStyle::default(),
            parse_state: ParseState { scope: Scope::with_std() },
            debug: false,
        }
    }

    /// Set the base text style.
    pub fn set_text_style(&mut self, style: TextStyle) {
        self.style.text = style;
    }

    /// Set the base page style.
    pub fn set_page_style(&mut self, style: PageStyle) {
        self.style.page = style;
    }

    /// Set whether to render debug boxes.
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    /// Parse source code into a syntax tree.
    pub fn parse(&self, src: &str) -> Pass<SyntaxModel> {
        parse(src, Pos::ZERO, &self.parse_state)
    }

    /// Layout a syntax tree and return the produced layout.
    pub async fn layout(&self, model: &SyntaxModel) -> Pass<MultiLayout> {
        use crate::layout::prelude::*;

        let margins = self.style.page.margins();
        crate::layout::layout(
            &model,
            LayoutContext {
                loader: &self.loader,
                style: &self.style,
                base: self.style.page.dimensions.unpadded(margins),
                spaces: vec![LayoutSpace {
                    dimensions: self.style.page.dimensions,
                    padding: margins,
                    expansion: LayoutExpansion::new(true, true),
                }],
                repeat: true,
                axes: LayoutAxes::new(LTT, TTB),
                align: LayoutAlign::new(Start, Start),
                nested: false,
                debug: self.debug,
            },
        ).await
    }

    /// Process source code directly into a collection of layouts.
    pub async fn typeset(&self, src: &str) -> Pass<MultiLayout> {
        let parsed = self.parse(src);
        let layouted = self.layout(&parsed.output).await;
        let feedback = Feedback::merge(parsed.feedback, layouted.feedback);
        Pass::new(layouted.output, feedback)
    }
}

/// The result of some pass: Some output `T` and feedback data.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Pass<T> {
    /// The output of this compilation pass.
    pub output: T,
    /// User feedback data accumulated in this pass.
    pub feedback: Feedback,
}

impl<T> Pass<T> {
    /// Create a new pass from output and feedback data.
    pub fn new(output: T, feedback: Feedback) -> Pass<T> {
        Pass { output, feedback }
    }

    /// Map the output type and keep the feedback data.
    pub fn map<F, U>(self, f: F) -> Pass<U> where F: FnOnce(T) -> U {
        Pass {
            output: f(self.output),
            feedback: self.feedback,
        }
    }
}

/// User feedback data accumulated during a compilation pass.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Feedback {
    /// Diagnostics in the source code.
    pub diagnostics: Diagnostics,
    /// Decorations of the source code for semantic syntax highlighting.
    pub decorations: Decorations,
}

impl Feedback {
    /// Create a new feedback instance without errors and decos.
    pub fn new() -> Feedback {
        Feedback {
            diagnostics: vec![],
            decorations: vec![],
        }
    }

    /// Merged two feedbacks into one.
    pub fn merge(mut a: Feedback, b: Feedback) -> Feedback {
        a.extend(b);
        a
    }

    /// Add other feedback data to this feedback.
    pub fn extend(&mut self, other: Feedback) {
        self.diagnostics.extend(other.diagnostics);
        self.decorations.extend(other.decorations);
    }

    /// Add more feedback whose spans are local and need to be offset by an
    /// `offset` to be correct in this feedback's context.
    pub fn extend_offset(&mut self, more: Feedback, offset: Pos) {
        self.diagnostics.extend(more.diagnostics.offset(offset));
        self.decorations.extend(more.decorations.offset(offset));
    }
}
