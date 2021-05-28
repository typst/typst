//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into a [syntax
//!   tree]. The structures describing the tree can be found in the [syntax]
//!   module.
//! - **Evaluation:** The next step is to [evaluate] the syntax tree. This
//!   computes the value of each node in the document and produces a [module].
//! - **Execution:** Now, we can [execute] the parsed and evaluated module.
//!   This produces a [layout tree], a high-level, fully styled representation
//!   of the document. The nodes of this tree are self-contained and
//!   order-independent and thus much better suited for layouting than the
//!   syntax tree.
//! - **Layouting:** Next, the tree is [layouted] into a portable version of the
//!   typeset document. The output of this is a collection of [`Frame`]s (one
//!   per page), ready for exporting.
//! - **Exporting:** The finished layout can be exported into a supported
//!   format. Currently, the only supported output format is [PDF].
//!
//! [tokens]: parse::Tokens
//! [parsed]: parse::parse
//! [syntax tree]: syntax::Tree
//! [evaluate]: eval::eval
//! [module]: eval::Module
//! [execute]: exec::exec
//! [layout tree]: layout::Tree
//! [layouted]: layout::layout
//! [PDF]: export::pdf

#[macro_use]
pub mod diag;
#[macro_use]
pub mod eval;
pub mod cache;
pub mod color;
pub mod exec;
pub mod export;
pub mod font;
pub mod geom;
pub mod image;
pub mod layout;
pub mod library;
pub mod loading;
pub mod paper;
pub mod parse;
pub mod pretty;
pub mod syntax;
pub mod util;

use std::rc::Rc;

use crate::cache::Cache;
use crate::diag::Pass;
use crate::eval::Scope;
use crate::exec::State;
use crate::layout::Frame;
use crate::loading::Loader;

/// Process source code directly into a collection of layouted frames.
pub fn typeset(
    loader: &mut dyn Loader,
    cache: &mut Cache,
    src: &str,
    base: &Scope,
    state: State,
) -> Pass<Vec<Frame>> {
    let parsed = parse::parse(src);
    let evaluated = eval::eval(loader, cache, Rc::new(parsed.output), base);
    let executed = exec::exec(&evaluated.output.template, state);
    let layouted = layout::layout(loader, cache, &executed.output);

    let mut diags = parsed.diags;
    diags.extend(evaluated.diags);
    diags.extend(executed.diags);

    Pass::new(layouted, diags)
}
