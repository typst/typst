//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. Then, a [parser] constructs a syntax tree
//!   from the token stream. The structures describing the tree can be found in
//!   the [syntax] module.
//! - **Layouting:** The next step is to transform the syntax tree into a
//!   portable representation of the typesetted document. Types for these can be
//!   found in the [layout] module. A finished layout ready for exporting is a
//!   [`MultiLayout`] consisting of multiple boxes (or pages).
//! - **Exporting:** The finished layout can then be exported into a supported
//!   format. Submodules for these formats are located in the [export] module.
//!   Currently, the only supported output format is [_PDF_].
//!
//! [tokens]: parse/struct.Tokens.html
//! [parser]: parse/fn.parse.html
//! [syntax]: syntax/index.html
//! [layout]: layout/index.html
//! [export]: export/index.html
//! [_PDF_]: export/pdf/index.html
//! [`MultiLayout`]: layout/type.MultiLayout.html

#[macro_use]
pub mod diagnostic;

pub mod color;
pub mod eval;
pub mod export;
pub mod font;
pub mod geom;
pub mod layout;
pub mod length;
pub mod library;
pub mod paper;
pub mod parse;
pub mod prelude;
pub mod shaping;
pub mod syntax;

use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

use crate::diagnostic::Diagnostic;
use crate::eval::{State, Value};
use crate::font::SharedFontLoader;
use crate::layout::{Commands, MultiLayout};
use crate::syntax::{Decoration, Offset, Pos, SpanVec};

/// Process source code directly into a collection of layouts.
pub async fn typeset(
    src: &str,
    state: State,
    loader: SharedFontLoader,
) -> Pass<MultiLayout> {
    let parsed = parse::parse(src);
    let layouted = layout::layout(&parsed.output, state, loader).await;
    let feedback = Feedback::merge(parsed.feedback, layouted.feedback);
    Pass::new(layouted.output, feedback)
}

/// A dynamic future type which allows recursive invocation of async functions
/// when used as the return type.
pub type DynFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

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
    pub fn new(output: T, feedback: Feedback) -> Self {
        Self { output, feedback }
    }

    /// Create a new pass with empty feedback.
    pub fn okay(output: T) -> Self {
        Self { output, feedback: Feedback::new() }
    }

    /// Map the output type and keep the feedback data.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Pass<U> {
        Pass {
            output: f(self.output),
            feedback: self.feedback,
        }
    }
}

impl Pass<Value> {
    /// Create a new pass with a list of layouting commands.
    pub fn commands(commands: Commands, feedback: Feedback) -> Self {
        Pass::new(Value::Commands(commands), feedback)
    }
}

/// Diagnostic and semantic syntax highlighting data.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Feedback {
    /// Diagnostics about the source code.
    pub diagnostics: SpanVec<Diagnostic>,
    /// Decorations of the source code for semantic syntax highlighting.
    pub decorations: SpanVec<Decoration>,
}

impl Feedback {
    /// Create a new feedback instance without errors and decos.
    pub fn new() -> Self {
        Self { diagnostics: vec![], decorations: vec![] }
    }

    /// Merged two feedbacks into one.
    pub fn merge(mut a: Self, b: Self) -> Self {
        a.extend(b);
        a
    }

    /// Add other feedback data to this feedback.
    pub fn extend(&mut self, more: Self) {
        self.diagnostics.extend(more.diagnostics);
        self.decorations.extend(more.decorations);
    }

    /// Add more feedback whose spans are local and need to be offset by an
    /// `offset` to be correct in this feedback's context.
    pub fn extend_offset(&mut self, more: Self, offset: Pos) {
        self.diagnostics.extend(more.diagnostics.offset(offset));
        self.decorations.extend(more.decorations.offset(offset));
    }
}
