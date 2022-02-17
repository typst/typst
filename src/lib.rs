//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into a [green
//!   tree]. The green tree itself is untyped, but a typed layer over it is
//!   provided in the [AST] module.
//! - **Evaluation:** The next step is to [evaluate] the markup. This produces a
//!   [module], consisting of a scope of values that were exported by the code
//!   and a [template], a hierarchical, styled representation with the contents
//!   of the module. The nodes of this tree are well structured and
//!   order-independent and thus much better suited for layouting than the raw
//!   markup.
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
//! [evaluate]: Vm::evaluate
//! [module]: eval::Module
//! [template]: eval::Template
//! [layouted]: eval::Template::layout
//! [cache]: layout::LayoutCache
//! [PDF]: export::pdf

#![allow(clippy::len_without_is_empty)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::try_err)]

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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::diag::TypResult;
use crate::eval::{Eval, Module, Scope, Scopes, StyleMap};
use crate::export::RenderCache;
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
    pub loader: Arc<dyn Loader>,
    /// Stores loaded source files.
    pub sources: SourceStore,
    /// Stores parsed font faces.
    pub fonts: FontStore,
    /// Stores decoded images.
    pub images: ImageStore,
    /// Caches layouting artifacts.
    #[cfg(feature = "layout-cache")]
    pub layout_cache: LayoutCache,
    /// Caches rendering artifacts.
    pub render_cache: RenderCache,
    /// The standard library scope.
    std: Scope,
    /// The default styles.
    styles: StyleMap,
}

impl Context {
    /// Create a new context with the default settings.
    pub fn new(loader: Arc<dyn Loader>) -> Self {
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
    pub fn styles(&self) -> &StyleMap {
        &self.styles
    }

    /// Typeset a source file into a collection of layouted frames.
    ///
    /// Returns either a vector of frames representing individual pages or
    /// diagnostics in the form of a vector of error message with file and span
    /// information.
    pub fn typeset(&mut self, id: SourceId) -> TypResult<Vec<Arc<Frame>>> {
        Vm::new(self).typeset(id)
    }

    /// Garbage-collect caches.
    pub fn turnaround(&mut self) {
        #[cfg(feature = "layout-cache")]
        self.layout_cache.turnaround();
    }
}

/// A builder for a [`Context`].
///
/// This struct is created by [`Context::builder`].
pub struct ContextBuilder {
    std: Option<Scope>,
    styles: Option<StyleMap>,
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
    pub fn styles(mut self, styles: StyleMap) -> Self {
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
    pub fn build(self, loader: Arc<dyn Loader>) -> Context {
        Context {
            sources: SourceStore::new(Arc::clone(&loader)),
            fonts: FontStore::new(Arc::clone(&loader)),
            images: ImageStore::new(Arc::clone(&loader)),
            loader,
            #[cfg(feature = "layout-cache")]
            layout_cache: LayoutCache::new(self.policy, self.max_size),
            render_cache: RenderCache::new(),
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

/// A virtual machine for a single typesetting process.
pub struct Vm<'a> {
    /// The loader the context was created with.
    pub loader: &'a dyn Loader,
    /// Stores loaded source files.
    pub sources: &'a mut SourceStore,
    /// Stores parsed font faces.
    pub fonts: &'a mut FontStore,
    /// Stores decoded images.
    pub images: &'a mut ImageStore,
    /// Caches layouting artifacts.
    #[cfg(feature = "layout-cache")]
    pub layout_cache: &'a mut LayoutCache,
    /// The default styles.
    pub styles: &'a StyleMap,
    /// The stack of imported files that led to evaluation of the current file.
    pub route: Vec<SourceId>,
    /// Caches imported modules.
    pub modules: HashMap<SourceId, Module>,
    /// The active scopes.
    pub scopes: Scopes<'a>,
    /// How deeply nested the current layout tree position is.
    #[cfg(feature = "layout-cache")]
    pub level: usize,
}

impl<'a> Vm<'a> {
    /// Create a new virtual machine.
    pub fn new(ctx: &'a mut Context) -> Self {
        Self {
            loader: ctx.loader.as_ref(),
            sources: &mut ctx.sources,
            fonts: &mut ctx.fonts,
            images: &mut ctx.images,
            layout_cache: &mut ctx.layout_cache,
            styles: &ctx.styles,
            route: vec![],
            modules: HashMap::new(),
            scopes: Scopes::new(Some(&ctx.std)),
            level: 0,
        }
    }

    /// Evaluate a source file and return the resulting module.
    ///
    /// Returns either a module containing a scope with top-level bindings and a
    /// layoutable template or diagnostics in the form of a vector of error
    /// message with file and span information.
    pub fn evaluate(&mut self, id: SourceId) -> TypResult<Module> {
        // Prevent cyclic evaluation.
        assert!(!self.route.contains(&id));

        // Check whether the module was already loaded.
        if let Some(module) = self.modules.get(&id) {
            return Ok(module.clone());
        }

        // Parse the file.
        let source = self.sources.get(id);
        let ast = source.ast()?;

        // Prepare the new context.
        let fresh = Scopes::new(self.scopes.base);
        let prev = std::mem::replace(&mut self.scopes, fresh);
        self.route.push(id);

        // Evaluate the module.
        let template = ast.eval(self)?;

        // Restore the old context.
        let scope = std::mem::replace(&mut self.scopes, prev).top;
        self.route.pop().unwrap();

        // Save the evaluated module.
        let module = Module { scope, template };
        self.modules.insert(id, module.clone());

        Ok(module)
    }

    /// Typeset a source file into a collection of layouted frames.
    ///
    /// Returns either a vector of frames representing individual pages or
    /// diagnostics in the form of a vector of error message with file and span
    /// information.
    pub fn typeset(&mut self, id: SourceId) -> TypResult<Vec<Arc<Frame>>> {
        let module = self.evaluate(id)?;
        let frames = module.template.layout(self);
        Ok(frames)
    }

    /// Resolve a user-entered path (relative to the source file) to be
    /// relative to the compilation environment's root.
    pub fn resolve(&self, path: &str) -> PathBuf {
        if let Some(&id) = self.route.last() {
            if let Some(dir) = self.sources.get(id).path().parent() {
                return dir.join(path);
            }
        }

        path.into()
    }
}
