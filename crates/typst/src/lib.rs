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
pub mod eval;
pub mod diag;
pub mod doc;
pub mod export;
pub mod font;
pub mod geom;
pub mod ide;
pub mod image;
pub mod model;

#[doc(inline)]
pub use typst_syntax as syntax;

use std::ops::Range;

use comemo::{Prehashed, Track, TrackedMut};
use ecow::EcoString;

use crate::diag::{FileResult, SourceResult};
use crate::doc::Document;
use crate::eval::{Bytes, Datetime, Library, Route, Tracer};
use crate::font::{Font, FontBook};
use crate::syntax::{FileId, PackageSpec, Source, Span};

/// Compile a source file into a fully layouted document.
#[tracing::instrument(skip_all)]
pub fn compile(world: &dyn World, tracer: &mut Tracer) -> SourceResult<Document> {
    let route = Route::default();

    // Call `track` just once to keep comemo's ID stable.
    let world = world.track();
    let mut tracer = tracer.track_mut();

    // Evaluate the source file into a module.
    let module = eval::eval(
        world,
        route.track(),
        TrackedMut::reborrow_mut(&mut tracer),
        &world.main(),
    )?;

    // Typeset it.
    model::typeset(world, tracer, &module.content())
}

/// The environment in which typesetting occurs.
///
/// All loading functions (`main`, `source`, `file`, `font`) should perform
/// internal caching so that they are relatively cheap on repeated invocations
/// with the same argument. [`Source`], [`Bytes`], and [`Font`] are
/// all reference-counted and thus cheap to clone.
///
/// The compiler doesn't do the caching itself because the world has much more
/// information on when something can change. For example, fonts typically don't
/// change and can thus even be cached across multiple compilations (for
/// long-running applications like `typst watch`). Source files on the other
/// hand can change and should thus be cleared after. Advanced clients like
/// language servers can also retain the source files and [edited](Source::edit)
/// them in-place to benefit from better incremental performance.
#[comemo::track]
pub trait World {
    /// The standard library.
    fn library(&self) -> &Prehashed<Library>;

    /// Metadata about all known fonts.
    fn book(&self) -> &Prehashed<FontBook>;

    /// Access the main source file.
    fn main(&self) -> Source;

    /// Try to access the specified source file.
    ///
    /// The returned `Source` file's [id](Source::id) does not have to match the
    /// given `id`. Due to symlinks, two different file id's can point to the
    /// same on-disk file. Implementors can deduplicate and return the same
    /// `Source` if they want to, but do not have to.
    fn source(&self, id: FileId) -> FileResult<Source>;

    /// Try to access the specified file.
    fn file(&self, id: FileId) -> FileResult<Bytes>;

    /// Try to access the font with the given index in the font book.
    fn font(&self, index: usize) -> Option<Font>;

    /// Get the current date.
    ///
    /// If no offset is specified, the local date should be chosen. Otherwise,
    /// the UTC date should be chosen with the corresponding offset in hours.
    ///
    /// If this function returns `None`, Typst's `datetime` function will
    /// return an error.
    fn today(&self, offset: Option<i64>) -> Option<Datetime>;

    /// A list of all available packages and optionally descriptions for them.
    ///
    /// This function is optional to implement. It enhances the user experience
    /// by enabling autocompletion for packages. Details about packages from the
    /// `@preview` namespace are available from
    /// `https://packages.typst.org/preview/index.json`.
    fn packages(&self) -> &[(PackageSpec, Option<EcoString>)] {
        &[]
    }

    /// Get the byte range for a span.
    #[track_caller]
    fn range(&self, span: Span) -> Range<usize> {
        self.source(span.id())
            .expect("span does not point into any source file")
            .range(span)
    }
}
