//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into a [syntax
//!   tree]. The tree itself is untyped, but a typed layer over it is provided
//!   in the [AST] module.
//! - **Evaluation:** The next step is to [evaluate] the markup. This produces a
//!   [module], consisting of a scope of values that were exported by the code
//!   and [content], a hierarchical, styled representation with the contents of
//!   the module. The nodes of the content tree are well structured and
//!   order-independent and thus much better suited for layouting than the raw
//!   markup.
//! - **Layouting:** Next, the tree is [layouted] into a portable version of the
//!   typeset document. The output of this is a collection of [`Frame`]s (one
//!   per page), ready for exporting.
//! - **Exporting:** The finished layout can be exported into a supported
//!   format. Currently, the only supported output format is [PDF].
//!
//! [tokens]: parse::Tokens
//! [parsed]: parse::parse
//! [syntax tree]: syntax::SyntaxNode
//! [AST]: syntax::ast
//! [evaluate]: eval::evaluate
//! [module]: eval::Module
//! [content]: model::Content
//! [layouted]: model::layout
//! [PDF]: export::pdf

#![allow(clippy::len_without_is_empty)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::try_err)]

#[macro_use]
pub mod util;
#[macro_use]
pub mod memo;
#[macro_use]
pub mod geom;
#[macro_use]
pub mod diag;
#[macro_use]
pub mod eval;
pub mod export;
pub mod font;
pub mod frame;
pub mod image;
pub mod library;
pub mod loading;
pub mod model;
pub mod parse;
pub mod source;
pub mod syntax;

use std::collections::HashMap;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;

use crate::diag::TypResult;
use crate::eval::{Module, Scope};
use crate::font::FontStore;
use crate::frame::Frame;
use crate::image::ImageStore;
use crate::loading::Loader;
use crate::memo::Track;
use crate::model::{PinBoard, PinConstraint, StyleMap};
use crate::source::{SourceId, SourceStore};

/// Typeset a source file into a collection of layouted frames.
///
/// Returns either a vector of frames representing individual pages or
/// diagnostics in the form of a vector of error message with file and span
/// information.
pub fn typeset(ctx: &mut Context, id: SourceId) -> TypResult<Vec<Arc<Frame>>> {
    let module = eval::evaluate(ctx, id, vec![])?;
    model::layout(ctx, &module.content)
}

/// The core context which holds the configuration and stores.
pub struct Context {
    /// Stores loaded source files.
    pub sources: SourceStore,
    /// Stores parsed font faces.
    pub fonts: FontStore,
    /// Stores decoded images.
    pub images: ImageStore,
    /// The context's configuration.
    config: Config,
    /// Stores evaluated modules.
    modules: HashMap<SourceId, Module>,
    /// Stores document pins.
    pins: PinBoard,
}

impl Context {
    /// Create a new context.
    pub fn new(loader: Arc<dyn Loader>, config: Config) -> Self {
        Self {
            sources: SourceStore::new(Arc::clone(&loader)),
            fonts: FontStore::new(Arc::clone(&loader)),
            images: ImageStore::new(loader),
            config,
            modules: HashMap::new(),
            pins: PinBoard::new(),
        }
    }
}

impl Track for &mut Context {
    type Constraint = PinConstraint;

    fn key<H: Hasher>(&self, hasher: &mut H) {
        self.pins.key(hasher);
    }

    fn matches(&self, constraint: &Self::Constraint) -> bool {
        self.pins.matches(constraint)
    }
}

/// Compilation configuration.
pub struct Config {
    /// The compilation root.
    pub root: PathBuf,
    /// The standard library scope.
    pub std: Arc<Scope>,
    /// The default styles.
    pub styles: Arc<StyleMap>,
}

impl Config {
    /// Create a new configuration builder.
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// A builder for a [`Config`].
///
/// This struct is created by [`Config::builder`].
#[derive(Debug, Default, Clone)]
pub struct ConfigBuilder {
    root: PathBuf,
    std: Option<Arc<Scope>>,
    styles: Option<Arc<StyleMap>>,
}

impl ConfigBuilder {
    /// The compilation root, relative to which absolute paths are.
    ///
    /// Default: Empty path.
    pub fn root(&mut self, root: impl Into<PathBuf>) -> &mut Self {
        self.root = root.into();
        self
    }

    /// The scope containing definitions that are available everywhere.
    ///
    /// Default: Typst's standard library.
    pub fn std(&mut self, std: impl Into<Arc<Scope>>) -> &mut Self {
        self.std = Some(std.into());
        self
    }

    /// The default properties for page size, font selection and so on.
    ///
    /// Default: Empty style map.
    pub fn styles(&mut self, styles: impl Into<Arc<StyleMap>>) -> &mut Self {
        self.styles = Some(styles.into());
        self
    }

    /// Finish building the configuration.
    pub fn build(&self) -> Config {
        Config {
            root: self.root.clone(),
            std: self.std.clone().unwrap_or_else(|| Arc::new(library::new())),
            styles: self.styles.clone().unwrap_or_default(),
        }
    }
}
