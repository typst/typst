//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into a [syntax
//!   tree]. The tree itself is untyped, but the [AST] module provides a typed
//!   layer over it.
//! - **Evaluation:** The next step is to [evaluate] the markup. This produces a
//!   [module], consisting of a scope of values that were exported by the code
//!   and [content], a hierarchical, styled representation of the text,
//!   structure, layouts, etc. of the module. The nodes of the content tree are
//!   well structured and order-independent and thus much better suited for
//!   layouting than the raw markup.
//! - **Layouting:** Next, the content is layouted into a portable version of
//!   the typeset document. The output of this is a collection of [`Frame`]s
//!   (one per page), ready for exporting.
//! - **Exporting:** The finished layout can be exported into a supported
//!   format. Currently, the only supported output format is [PDF].
//!
//! [tokens]: syntax::Tokens
//! [parsed]: syntax::parse
//! [syntax tree]: syntax::SyntaxNode
//! [AST]: syntax::ast
//! [evaluate]: model::eval
//! [module]: model::Module
//! [content]: model::Content
//! [PDF]: export::pdf

extern crate self as typst;

#[macro_use]
pub mod util;
#[macro_use]
pub mod geom;
#[macro_use]
pub mod diag;
#[macro_use]
pub mod model;
pub mod export;
pub mod font;
pub mod frame;
pub mod image;
pub mod syntax;

use std::path::Path;

use comemo::{Prehashed, Track};

use crate::diag::{FileResult, SourceResult};
use crate::font::{Font, FontBook};
use crate::frame::Frame;
use crate::model::{Library, Route, StyleChain};
use crate::syntax::{Source, SourceId};
use crate::util::Buffer;

/// Typeset a source file into a collection of layouted frames.
///
/// Returns either a vector of frames representing individual pages or
/// diagnostics in the form of a vector of error message with file and span
/// information.
pub fn typeset(
    world: &(dyn World + 'static),
    source: &Source,
) -> SourceResult<Vec<Frame>> {
    // Evaluate the source file into a module.
    let route = Route::default();
    let module = model::eval(world.track(), route.track(), source)?;

    // Layout the module's contents.
    let library = world.library();
    let styles = StyleChain::with_root(&library.styles);
    (library.items.layout)(&module.content, world.track(), styles)
}

/// The environment in which typesetting occurs.
#[comemo::track]
pub trait World {
    /// The compilation root.
    fn root(&self) -> &Path;

    /// The standard library.
    fn library(&self) -> &Prehashed<Library>;

    /// Metadata about all known fonts.
    fn book(&self) -> &Prehashed<FontBook>;

    /// Try to access the font with the given id.
    fn font(&self, id: usize) -> Option<Font>;

    /// Try to access a file at a path.
    fn file(&self, path: &Path) -> FileResult<Buffer>;

    /// Try to resolve the unique id of a source file.
    fn resolve(&self, path: &Path) -> FileResult<SourceId>;

    /// Access a source file by id.
    fn source(&self, id: SourceId) -> &Source;
}
