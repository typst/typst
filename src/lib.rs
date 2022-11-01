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
//! - **Layouting:** Next, the content is [layouted] into a portable version of
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
//! [layouted]: model::layout
//! [PDF]: export::pdf

#![allow(clippy::len_without_is_empty)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::try_err)]

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
pub mod library;
pub mod syntax;

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use comemo::{Prehashed, Track};

use crate::diag::{FileResult, SourceResult};
use crate::font::{Font, FontBook};
use crate::frame::Frame;
use crate::model::{Content, Route, Scope, StyleMap};
use crate::syntax::{Source, SourceId};
use crate::util::{Buffer, EcoString};

/// Typeset a source file into a collection of layouted frames.
///
/// Returns either a vector of frames representing individual pages or
/// diagnostics in the form of a vector of error message with file and span
/// information.
pub fn typeset(
    world: &(dyn World + 'static),
    main: SourceId,
) -> SourceResult<Vec<Frame>> {
    let route = Route::default();
    let module = model::eval(world.track(), route.track(), main)?;
    model::layout(world.track(), &module.content)
}

/// The environment in which typesetting occurs.
#[comemo::track]
pub trait World {
    /// Access the global configuration.
    fn config(&self) -> &Prehashed<Config>;

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

/// The global configuration for typesetting.
#[derive(Debug, Clone, Hash)]
pub struct Config {
    /// The compilation root, relative to which absolute paths are.
    ///
    /// Default: Empty path.
    pub root: PathBuf,
    /// The scope containing definitions that are available everywhere.
    ///
    /// Default: Typst's standard library.
    pub std: Scope,
    /// Defines which standard library items fulfill which syntactical roles.
    ///
    /// Default: Typst's standard library's language map.
    pub items: LangItems,
    /// The default properties for page size, font selection and so on.
    ///
    /// Default: Empty style map.
    pub styles: StyleMap,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
            std: library::scope(),
            items: library::items(),
            styles: StyleMap::new(),
        }
    }
}

/// Definition of certain standard library items the language is aware of.
#[derive(Debug, Clone, Hash)]
pub struct LangItems {
    pub space: fn() -> Content,
    pub linebreak: fn(justify: bool) -> Content,
    pub text: fn(text: EcoString) -> Content,
    pub smart_quote: fn(double: bool) -> Content,
    pub parbreak: fn() -> Content,
    pub strong: fn(body: Content) -> Content,
    pub emph: fn(body: Content) -> Content,
    pub raw: fn(text: EcoString, tag: Option<EcoString>, block: bool) -> Content,
    pub link: fn(label: EcoString) -> Content,
    pub ref_: fn(target: EcoString) -> Content,
    pub heading: fn(level: NonZeroUsize, body: Content) -> Content,
    pub list_item: fn(body: Content) -> Content,
    pub enum_item: fn(number: Option<usize>, body: Content) -> Content,
    pub desc_item: fn(term: Content, body: Content) -> Content,
}
