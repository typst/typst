//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into a [syntax
//!   tree]. The structures describing the tree can be found in the [syntax]
//!   module.
//! - **Evaluation:** The next step is to [evaluate] the parsed "script" into a
//!   [layout tree], a high-level, fully styled representation. The nodes of
//!   this tree are fully self-contained and order-independent and thus much
//!   better suited for layouting than the syntax tree.
//! - **Layouting:** Next, the tree is to [layouted] into a portable version of
//!   the typeset document. The output of this is a vector of [`Frame`]s
//!   (corresponding to pages), ready for exporting.
//! - **Exporting:** The finished layout can be exported into a supported
//!   format. Submodules for these formats are located in the [export] module.
//!   Currently, the only supported output format is [_PDF_].
//!
//! [tokens]: parse::Tokens
//! [parsed]: parse::parse
//! [syntax tree]: syntax::Tree
//! [evaluate]: eval::eval
//! [layout tree]: layout::Tree
//! [layouted]: layout::layout
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
pub mod pretty;
pub mod shaping;
pub mod syntax;

use crate::diag::{Feedback, Pass};
use crate::env::Env;
use crate::eval::{Scope, State};
use crate::layout::Frame;

/// Process _Typst_ source code directly into a collection of frames.
pub fn typeset(
    src: &str,
    env: &mut Env,
    scope: &Scope,
    state: State,
) -> Pass<Vec<Frame>> {
    let Pass { output: syntax_tree, feedback: f1 } = parse::parse(src);
    let Pass { output: layout_tree, feedback: f2 } =
        eval::eval(&syntax_tree, env, scope, state);
    let frames = layout::layout(&layout_tree, env);
    Pass::new(frames, Feedback::join(f1, f2))
}
