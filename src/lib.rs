//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into a
//!   [green tree]. The green tree itself is untyped, but a typed layer over it
//!   is provided in the [AST] module.
//! - **Evaluation:** The next step is to [evaluate] the markup. This produces a
//!   [module], consisting of a scope of values that were exported by the code
//!   and a [node] with the contents of the module. This node can be converted
//!   into a [layout tree], a hierarchical, styled representation of the
//!   document. The nodes of this tree are well structured and order-independent
//!   and thus much better suited for layouting than the raw markup.
//! - **Layouting:** Next, the tree is [layouted] into a portable version of the
//!   typeset document. The output of this is a collection of [`Frame`]s (one
//!   per page), ready for exporting. This step is supported by an incremental
//!   [cache] that enables reuse of intermediate layouting results.
//! - **Exporting:** The finished layout can be exported into a supported
//!   format. Currently, the only supported output format is [PDF].
//!
//! [tokens]: parse::Tokens
//! [parsed]: parse::parse
//! [green tree]: syntax::GreenNode
//! [AST]: syntax::ast
//! [evaluate]: Context::evaluate
//! [module]: eval::Module
//! [node]: eval::Node
//! [layout tree]: layout::RootNode
//! [layouted]: layout::RootNode::layout
//! [cache]: layout::LayoutCache
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
pub mod syntax;

use std::rc::Rc;

use crate::diag::TypResult;
use crate::eval::{Eval, EvalContext, Module, Scope, Styles};
use crate::font::FontStore;
use crate::frame::Frame;
use crate::image::ImageStore;
#[cfg(feature = "layout-cache")]
use crate::layout::{EvictionPolicy, LayoutCache};
use crate::loading::Loader;
use crate::source::{SourceId, SourceStore};

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
    /// The default styles.
    styles: Styles,
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

    /// A read-only reference to the styles.
    pub fn styles(&self) -> &Styles {
        &self.styles
    }

    /// Evaluate a source file and return the resulting module.
    ///
    /// Returns either a module containing a scope with top-level bindings and a
    /// layoutable node or diagnostics in the form of a vector of error message
    /// with file and span information.
    pub fn evaluate(&mut self, id: SourceId) -> TypResult<Module> {
        let markup = self.sources.get(id).ast()?;
        let mut ctx = EvalContext::new(self, id);
        let node = markup.eval(&mut ctx)?;
        Ok(Module { scope: ctx.scopes.top, node })
    }

    /// Typeset a source file into a collection of layouted frames.
    ///
    /// Returns either a vector of frames representing individual pages or
    /// diagnostics in the form of a vector of error message with file and span
    /// information.
    pub fn typeset(&mut self, id: SourceId) -> TypResult<Vec<Rc<Frame>>> {
        let module = self.evaluate(id)?;
        let tree = module.into_root();
        let frames = tree.layout(self);
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
    styles: Option<Styles>,
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

    /// The default properties for page size, font selection and so on.
    pub fn styles(mut self, styles: Styles) -> Self {
        self.styles = Some(styles);
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
            styles: self.styles.unwrap_or_default(),
        }
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self {
            std: None,
            styles: None,
            #[cfg(feature = "layout-cache")]
            policy: EvictionPolicy::default(),
            #[cfg(feature = "layout-cache")]
            max_size: 2000,
        }
    }
}
