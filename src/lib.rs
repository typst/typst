//! The compiler for the _Typst_ markup language.
//!
//! # Steps
//! - **Parsing:**
//!   The compiler first transforms a plain string into an iterator of [tokens].
//!   This token stream is [parsed] into a [syntax tree]. The tree itself is
//!   untyped, but the [AST] module provides a typed layer over it.
//! - **Evaluation:**
//!   The next step is to [evaluate] the markup. This produces a [module],
//!   consisting of a scope of values that were exported by the code and
//!   [content], a hierarchical, styled representation of what was written in
//!   the source file. The elements of the content tree are well structured and
//!   order-independent and thus much better suited for further processing than
//!   the raw markup.
//! - **Typesetting:**
//!   Next, the content is [typeset] into a [document] containing one [frame]
//!   per page with items at fixed positions.
//! - **Exporting:**
//!   These frames can finally be exported into an output format (currently
//!   supported are [PDF] and [raster images]).
//!
//! [tokens]: syntax::SyntaxKind
//! [parsed]: syntax::parse
//! [syntax tree]: syntax::SyntaxNode
//! [AST]: syntax::ast
//! [evaluate]: eval::eval
//! [module]: eval::Module
//! [content]: model::Content
//! [typeset]: model::typeset
//! [document]: doc::Document
//! [frame]: doc::Frame
//! [PDF]: export::pdf
//! [raster images]: export::render

#![recursion_limit = "1000"]
#![allow(clippy::comparison_chain)]

extern crate self as typst;

#[macro_use]
pub mod util;
#[macro_use]
pub mod diag;
#[macro_use]
pub mod eval;
pub mod doc;
pub mod export;
pub mod font;
pub mod geom;
pub mod ide;
pub mod image;
pub mod model;
pub mod syntax;

use std::path::Path;

use comemo::{Prehashed, Track, TrackedMut};

use crate::diag::{FileResult, SourceResult};
use crate::doc::Document;
use crate::eval::{Datetime, Library, Route, Tracer};
use crate::font::{Font, FontBook};
use crate::syntax::{Source, SourceId};
use crate::util::Buffer;

/// Compile a source file into a fully layouted document.
#[tracing::instrument(skip(world))]
pub fn compile(world: &dyn World) -> SourceResult<Document> {
    let route = Route::default();
    let mut tracer = Tracer::default();

    // Call `track` just once to keep comemo's ID stable.
    let world = world.track();
    let mut tracer = tracer.track_mut();

    // Evaluate the source file into a module.
    tracing::info!("Starting evaluation");
    let module = eval::eval(
        world,
        route.track(),
        TrackedMut::reborrow_mut(&mut tracer),
        world.main(),
    )?;

    // Typeset the module's contents.
    model::typeset(world, tracer, &module.content())
}

/// The environment in which typesetting occurs.
#[comemo::track]
pub trait World {
    /// The path relative to which absolute paths are.
    ///
    /// Defaults to the empty path.
    fn root(&self) -> &Path {
        Path::new("")
    }

    /// The standard library.
    fn library(&self) -> &Prehashed<Library>;

    /// The main source file.
    fn main(&self) -> &Source;

    /// Try to resolve the unique id of a source file.
    fn resolve(&self, path: &Path) -> FileResult<SourceId>;

    /// Access a source file by id.
    fn source(&self, id: SourceId) -> &Source;

    /// Metadata about all known fonts.
    fn book(&self) -> &Prehashed<FontBook>;

    /// Try to access the font with the given id.
    fn font(&self, id: usize) -> Option<Font>;

    /// Try to access a file at a path.
    fn file(&self, path: &Path) -> FileResult<Buffer>;

    /// Get the current date.
    ///
    /// If no offset is specified, the local date should be chosen. Otherwise,
    /// the UTC date should be chosen with the corresponding offset in hours.
    fn today(&self, offset: Option<i64>) -> Option<Datetime>;
}
