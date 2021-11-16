//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into [markup].
//!   The syntactical structures describing markup and embedded code can be
//!   found in the [syntax] module.
//! - **Evaluation:** The next step is to [evaluate] the markup. This produces a
//!   [module], consisting of a scope of values that were exported by the code
//!   and a template with the contents of the module. This template can be
//!   instantiated with a style to produce a layout tree, a high-level, fully
//!   styled representation, rooted in the [document node]. The nodes of this
//!   tree are self-contained and order-independent and thus much better suited
//!   for layouting than the raw markup.
//! - **Layouting:** Next, the tree is [layouted] into a portable version of the
//!   typeset document. The output of this is a collection of [`Frame`]s (one
//!   per page), ready for exporting.
//! - **Exporting:** The finished layout can be exported into a supported
//!   format. Currently, the only supported output format is [PDF].
//!
//! [tokens]: parse::Tokens
//! [parsed]: parse::parse
//! [markup]: syntax::ast::Markup
//! [evaluate]: eval::eval
//! [module]: eval::Module
//! [layout tree]: layout::LayoutTree
//! [document node]: library::DocumentNode
//! [layouted]: layout::layout
//! [PDF]: export::pdf

#[macro_use]
pub mod util;
#[macro_use]
pub mod diag;
#[macro_use]
pub mod eval;
pub mod export;
pub mod font;
pub mod frame;
pub mod geom;
pub mod image;
pub mod layout;
pub mod library;
pub mod loading;
pub mod parse;
pub mod source;
pub mod style;
pub mod syntax;

use std::rc::Rc;

use crate::diag::TypResult;
use crate::eval::{Module, Scope};
use crate::font::FontStore;
use crate::frame::Frame;
use crate::image::ImageStore;
#[cfg(feature = "layout-cache")]
use crate::layout::{EvictionPolicy, LayoutCache};
use crate::library::DocumentNode;
use crate::loading::Loader;
use crate::source::{SourceId, SourceStore};
use crate::style::Style;

/// The core context which holds the loader, configuration and cached artifacts.
pub struct Context {
    /// The loader the context was created with.
    pub loader: Rc<dyn Loader>,
    /// Stores loaded source files.
    pub sources: SourceStore,
    /// Stores parsed font faces.
    pub fonts: FontStore,
    /// Stores decoded images.
    pub images: ImageStore,
    /// Caches layouting artifacts.
    #[cfg(feature = "layout-cache")]
    pub layouts: LayoutCache,
    /// The standard library scope.
    std: Scope,
    /// The default style.
    style: Style,
}

impl Context {
    /// Create a new context with the default settings.
    pub fn new(loader: Rc<dyn Loader>) -> Self {
        Self::builder().build(loader)
    }

    /// Create a new context with advanced settings.
    pub fn builder() -> ContextBuilder {
        ContextBuilder::default()
    }

    /// A read-only reference to the standard library scope.
    pub fn std(&self) -> &Scope {
        &self.std
    }

    /// A read-only reference to the style.
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// Evaluate a source file and return the resulting module.
    pub fn evaluate(&mut self, id: SourceId) -> TypResult<Module> {
        let ast = self.sources.get(id).ast()?;
        eval::eval(self, id, &ast)
    }

    /// Execute a source file and produce the resulting page nodes.
    pub fn execute(&mut self, id: SourceId) -> TypResult<DocumentNode> {
        let module = self.evaluate(id)?;
        Ok(module.template.to_document(&self.style))
    }

    /// Typeset a source file into a collection of layouted frames.
    ///
    /// Returns either a vector of frames representing individual pages or
    /// diagnostics in the form of a vector of error message with file and span
    /// information.
    pub fn typeset(&mut self, id: SourceId) -> TypResult<Vec<Rc<Frame>>> {
        let tree = self.execute(id)?;
        let frames = layout::layout(self, &tree);
        Ok(frames)
    }

    /// Garbage-collect caches.
    pub fn turnaround(&mut self) {
        #[cfg(feature = "layout-cache")]
        self.layouts.turnaround();
    }
}

/// A builder for a [`Context`].
///
/// This struct is created by [`Context::builder`].
pub struct ContextBuilder {
    std: Option<Scope>,
    style: Option<Style>,
    #[cfg(feature = "layout-cache")]
    policy: EvictionPolicy,
    #[cfg(feature = "layout-cache")]
    max_size: usize,
}

impl ContextBuilder {
    /// The scope containing definitions that are available everywhere
    /// (the standard library).
    pub fn std(mut self, std: Scope) -> Self {
        self.std = Some(std);
        self
    }

    /// The initial properties for page size, font selection and so on.
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// The policy for eviction of the layout cache.
    #[cfg(feature = "layout-cache")]
    pub fn cache_policy(mut self, policy: EvictionPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The maximum number of entries the layout cache should have.
    ///
    /// Note that this can be exceeded if more entries are categorized as [must
    /// keep][crate::layout::PatternProperties::must_keep].
    #[cfg(feature = "layout-cache")]
    pub fn cache_max_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }

    /// Finish building the context by providing the `loader` used to load
    /// fonts, images, source files and other resources.
    pub fn build(self, loader: Rc<dyn Loader>) -> Context {
        Context {
            sources: SourceStore::new(Rc::clone(&loader)),
            fonts: FontStore::new(Rc::clone(&loader)),
            images: ImageStore::new(Rc::clone(&loader)),
            loader,
            #[cfg(feature = "layout-cache")]
            layouts: LayoutCache::new(self.policy, self.max_size),
            std: self.std.unwrap_or_else(library::new),
            style: self.style.unwrap_or_default(),
        }
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self {
            std: None,
            style: None,
            #[cfg(feature = "layout-cache")]
            policy: EvictionPolicy::default(),
            #[cfg(feature = "layout-cache")]
            max_size: 2000,
        }
    }
}
