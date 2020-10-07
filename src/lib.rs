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

use crate::diag::{Feedback, Pass};
use crate::eval::State;
use crate::font::SharedFontLoader;
use crate::layout::BoxLayout;

/// Process _Typst_ source code directly into a collection of layouts.
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
