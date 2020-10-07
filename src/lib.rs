//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into a [syntax
//!   tree]. The structures describing the tree can be found in the [AST]
//!   module.
//! - **Evaluation:** The next step is to [evaluate] the parsed "script" to a
//!   [document], a high-level, fully styled representation. The [nodes] of the
//!   document tree are fully self-contained and order-independent and thus much
//!   better suited for layouting than the syntax tree.
//! - **Layouting:** The next step is to [layout] the document into a portable
//!   version of the typesetted document. The output of this is a vector of
//!   [`BoxLayouts`] (corresponding to pages), ready for exporting.
//! - **Exporting:** The finished layout can be exported into a supported
//!   format. Submodules for these formats are located in the [export] module.
//!   Currently, the only supported output format is [_PDF_].
//!
//! [tokens]: parse/struct.Tokens.html
//! [parsed]: parse/fn.parse.html
//! [syntax tree]: syntax/ast/type.SynTree.html
//! [AST]: syntax/ast/index.html
//! [evaluate]: eval/fn.eval.html
//! [document]: layout/nodes/struct.Document.html
//! [nodes]: layout/nodes/index.html
//! [layout]: layout/fn.layout.html
//! [`BoxLayouts`]: layout/struct.BoxLayout.html
//! [export]: export/index.html
//! [_PDF_]: export/pdf/index.html

#![allow(unused)]

#[macro_use]
pub mod diag;

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

use crate::diag::Diag;
use crate::eval::State;
use crate::font::SharedFontLoader;
use crate::layout::BoxLayout;
use crate::syntax::{Deco, Offset, Pos, SpanVec};

/// Process source code directly into a collection of layouts.
pub async fn typeset(
    src: &str,
    state: State,
    loader: SharedFontLoader,
) -> Pass<Vec<BoxLayout>> {
    let Pass { output: tree, feedback: f1 } = parse::parse(src);
    let Pass { output: document, feedback: f2 } = eval::eval(&tree, state);
    let layouts = layout::layout(&document, loader).await;
    Pass::new(layouts, Feedback::join(f1, f2))
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

/// Diagnostic and semantic syntax highlighting data.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Feedback {
    /// Diagnostics about the source code.
    pub diags: SpanVec<Diag>,
    /// Decorations of the source code for semantic syntax highlighting.
    pub decos: SpanVec<Deco>,
}

impl Feedback {
    /// Create a new feedback instance without errors and decos.
    pub fn new() -> Self {
        Self { diags: vec![], decos: vec![] }
    }

    /// Merge two feedbacks into one.
    pub fn join(mut a: Self, b: Self) -> Self {
        a.extend(b);
        a
    }

    /// Add other feedback data to this feedback.
    pub fn extend(&mut self, more: Self) {
        self.diags.extend(more.diags);
        self.decos.extend(more.decos);
    }

    /// Add more feedback whose spans are local and need to be translated by an
    /// `offset` to be correct in this feedback's context.
    pub fn extend_offset(&mut self, more: Self, offset: Pos) {
        self.diags.extend(more.diags.offset(offset));
        self.decos.extend(more.decos.offset(offset));
    }
}
