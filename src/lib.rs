//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into a [syntax
//!   tree]. The structures describing the tree can be found in the [syntax]
//!   module.
//! - **Evaluation:** The next step is to [evaluate] the parsed "script" to a
//!   [document], a high-level, fully styled representation. The nodes of the
//!   document tree are fully self-contained and order-independent and thus much
//!   better suited for layouting than the syntax tree.
//! - **Layouting:** The next step is to [layout] the document into a portable
//!   version of the typeset document. The output of this is a vector of
//!   [`BoxLayout`]s (corresponding to pages), ready for exporting.
//! - **Exporting:** The finished layout can be exported into a supported
//!   format. Submodules for these formats are located in the [export] module.
//!   Currently, the only supported output format is [_PDF_].
//!
//! [tokens]: parse::Tokens
//! [parsed]: parse::parse
//! [syntax tree]: syntax::SynTree
//! [evaluate]: eval::eval
//! [document]: layout::Document
//! [layout]: layout::layout
//! [_PDF_]: export::pdf

#[macro_use]
pub mod diag;
#[macro_use]
pub mod eval;
pub mod color;
pub mod env;
pub mod export;
pub mod font;
pub mod geom;
pub mod layout;
pub mod library;
pub mod paper;
pub mod parse;
pub mod prelude;
pub mod shaping;
pub mod syntax;

use std::rc::Rc;

use crate::diag::{Feedback, Pass};
use crate::env::SharedEnv;
use crate::eval::State;
use crate::layout::BoxLayout;

/// Process _Typst_ source code directly into a collection of layouts.
pub fn typeset(src: &str, env: SharedEnv, state: State) -> Pass<Vec<BoxLayout>> {
    let Pass { output: tree, feedback: f1 } = parse::parse(src);
    let Pass { output: document, feedback: f2 } =
        eval::eval(&tree, Rc::clone(&env), state);
    let layouts = layout::layout(&document, env);
    Pass::new(layouts, Feedback::join(f1, f2))
}
