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
//!   per page), ready for exporting.
//! - **Exporting:** The finished layout can be exported into a supported
//!   format. Currently, the only supported output format is [PDF].
//!
//! [tokens]: parse::Tokens
//! [parsed]: parse::parse
//! [green tree]: syntax::GreenNode
//! [AST]: syntax::ast
//! [evaluate]: eval::Eval
//! [module]: eval::Module
//! [template]: eval::Template
//! [layouted]: eval::Template::layout
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
pub mod library;
pub mod loading;
pub mod parse;
pub mod source;
pub mod syntax;

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use crate::diag::TypResult;
use crate::eval::{Eval, Module, Scope, Scopes, StyleMap};
use crate::font::FontStore;
use crate::frame::Frame;
use crate::image::ImageStore;
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
    /// The standard library scope.
    std: Arc<Scope>,
    /// The default styles.
    styles: Arc<StyleMap>,
    /// Cached modules.
    modules: HashMap<SourceId, Module>,
    /// Cached queries.
    cache: HashMap<u64, CacheEntry>,
    /// The stack of imported files that led to evaluation of the current file.
    route: Vec<SourceId>,
    /// The dependencies of the current evaluation process.
    deps: Vec<(SourceId, usize)>,
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

    /// Evaluate a source file and return the resulting module.
    ///
    /// Returns either a module containing a scope with top-level bindings and a
    /// layoutable template or diagnostics in the form of a vector of error
    /// messages with file and span information.
    pub fn evaluate(&mut self, id: SourceId) -> TypResult<Module> {
        // Prevent cyclic evaluation.
        if self.route.contains(&id) {
            let path = self.sources.get(id).path().display();
            panic!("Tried to cyclicly evaluate {}", path);
        }

        // Check whether the module was already evaluated.
        if let Some(module) = self.modules.get(&id) {
            if module.valid(&self.sources) {
                return Ok(module.clone());
            } else {
                self.modules.remove(&id);
            }
        }

        // Parse the file.
        let source = self.sources.get(id);
        let ast = source.ast()?;

        let std = self.std.clone();
        let mut scp = Scopes::new(Some(&std));

        // Evaluate the module.
        let prev = std::mem::replace(&mut self.deps, vec![(id, source.rev())]);
        self.route.push(id);
        let template = ast.eval(self, &mut scp);
        self.route.pop().unwrap();
        let deps = std::mem::replace(&mut self.deps, prev);

        // Assemble the module.
        let module = Module {
            scope: scp.top,
            template: template?,
            deps,
        };

        // Save the evaluated module.
        self.modules.insert(id, module.clone());

        Ok(module)
    }

    /// Typeset a source file into a collection of layouted frames.
    ///
    /// Returns either a vector of frames representing individual pages or
    /// diagnostics in the form of a vector of error message with file and span
    /// information.
    pub fn typeset(&mut self, id: SourceId) -> TypResult<Vec<Arc<Frame>>> {
        self.evaluate(id)?.template.layout(self)
    }

    /// Resolve a user-entered path (relative to the current evaluation
    /// location) to be relative to the compilation environment's root.
    pub fn resolve(&self, path: &str) -> PathBuf {
        if let Some(&id) = self.route.last() {
            if let Some(dir) = self.sources.get(id).path().parent() {
                return dir.join(path);
            }
        }

        path.into()
    }
}

/// A builder for a [`Context`].
///
/// This struct is created by [`Context::builder`].
pub struct ContextBuilder {
    std: Option<Arc<Scope>>,
    styles: Option<Arc<StyleMap>>,
}

impl ContextBuilder {
    /// The scope containing definitions that are available everywhere
    /// (the standard library).
    pub fn std(mut self, std: impl Into<Arc<Scope>>) -> Self {
        self.std = Some(std.into());
        self
    }

    /// The default properties for page size, font selection and so on.
    pub fn styles(mut self, styles: impl Into<Arc<StyleMap>>) -> Self {
        self.styles = Some(styles.into());
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
            std: self.std.unwrap_or_else(|| Arc::new(library::new())),
            styles: self.styles.unwrap_or_default(),
            modules: HashMap::new(),
            cache: HashMap::new(),
            route: vec![],
            deps: vec![],
        }
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self { std: None, styles: None }
    }
}

/// An entry in the query cache.
struct CacheEntry {
    /// The query's results.
    data: Box<dyn Any>,
    /// How many evictions have passed since the entry has been last used.
    age: usize,
}

impl Context {
    /// Execute a query.
    ///
    /// This hashes all inputs to the query and then either returns a cached
    /// version or executes the query, saves the results in the cache and
    /// returns a reference to them.
    pub fn query<I, O>(
        &mut self,
        input: I,
        query: fn(ctx: &mut Self, input: I) -> O,
    ) -> &O
    where
        I: Hash,
        O: 'static,
    {
        let hash = fxhash::hash64(&input);
        if !self.cache.contains_key(&hash) {
            let output = query(self, input);
            self.cache.insert(hash, CacheEntry { data: Box::new(output), age: 0 });
        }

        let entry = self.cache.get_mut(&hash).unwrap();
        entry.age = 0;
        entry.data.downcast_ref().expect("oh no, a hash collision")
    }

    /// Garbage-collect the query cache. This deletes elements which haven't
    /// been used in a while.
    ///
    /// Returns details about the eviction.
    pub fn evict(&mut self) -> Eviction {
        const MAX_AGE: usize = 5;

        let before = self.cache.len();
        self.cache.retain(|_, entry| {
            entry.age += 1;
            entry.age <= MAX_AGE
        });

        Eviction { before, after: self.cache.len() }
    }
}

/// Details about a cache eviction.
pub struct Eviction {
    /// The number of items in the cache before the eviction.
    pub before: usize,
    /// The number of items in the cache after the eviction.
    pub after: usize,
}

impl Display for Eviction {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(f, "Before: {}", self.before)?;
        writeln!(f, "Evicted: {}", self.before - self.after)?;
        writeln!(f, "After: {}", self.after)
    }
}
