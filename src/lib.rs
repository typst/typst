//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into a [syntax
//!   tree]. The structures describing the tree can be found in the [syntax]
//!   module.
//! - **Evaluation:** The next step is to [evaluate] the syntax tree. This
//!   computes the value of each node in document and stores them in a map from
//!   node-pointers to values.
//! - **Execution:** Now, we can [execute] the parsed and evaluated "script".
//!   This produces a [layout tree], a high-level, fully styled representation
//!   of the document. The nodes of this tree are self-contained and
//!   order-independent and thus much better suited for layouting than the
//!   syntax tree.
//! - **Layouting:** Next, the tree is [layouted] into a portable version of the
//!   typeset document. The output of this is a collection of [`Frame`]s (one
//!   per page), ready for exporting.
//! - **Exporting:** The finished layout can be exported into a supported
//!   format. Submodules for these formats are located in the [export] module.
//!   Currently, the only supported output format is [_PDF_].
//!
//! [tokens]: parse::Tokens
//! [parsed]: parse::parse
//! [syntax tree]: syntax::Tree
//! [evaluate]: eval::eval
//! [execute]: exec::exec
//! [layout tree]: layout::Tree
//! [layouted]: layout::layout
//! [_PDF_]: export::pdf

#[macro_use]
pub mod diag;
#[macro_use]
pub mod eval;
pub mod color;
pub mod env;
pub mod exec;
pub mod export;
pub mod geom;
pub mod layout;
pub mod library;
pub mod paper;
pub mod parse;
pub mod pretty;
pub mod syntax;

use crate::diag::Pass;
use crate::env::Env;
use crate::eval::Scope;
use crate::exec::State;
use crate::layout::Frame;

/// Process _Typst_ source code directly into a collection of frames.
pub fn typeset(
    env: &mut Env,
    src: &str,
    scope: &Scope,
    state: State,
) -> Pass<Vec<Frame>> {
    let parsed = parse::parse(src);
    let evaluated = eval::eval(env, &parsed.output, scope);
    let executed = exec::exec(env, &parsed.output, &evaluated.output, state);
    let frames = layout::layout(env, &executed.output);

    let mut diags = parsed.diags;
    diags.extend(evaluated.diags);
    diags.extend(executed.diags);

    Pass::new(frames, diags)
}
