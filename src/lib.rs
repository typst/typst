//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. This token stream is [parsed] into a [green
//!   tree]. The green tree itself is untyped, but a typed layer over it is
//!   provided in the [AST] module.
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
//! [green tree]: syntax::GreenNode
//! [AST]: syntax::ast
//! [evaluate]: eval::Eval
//! [module]: eval::Module
//! [content]: model::Content
//! [layouted]: model::Content::layout
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
pub mod eval;
pub mod export;
pub mod font;
pub mod frame;
pub mod image;
pub mod library;
pub mod loading;
pub mod model;
pub mod parse;
pub mod query;
pub mod source;
pub mod syntax;

use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;

use crate::diag::{StrResult, TypResult};
use crate::eval::{Eval, Flow, Module, Scope, Scopes};
use crate::font::FontStore;
use crate::frame::Frame;
use crate::image::ImageStore;
use crate::loading::Loader;
use crate::model::StyleMap;
use crate::source::{SourceId, SourceStore};
use crate::util::PathExt;

/// The core context which holds the loader, stores, and configuration.
pub struct Context {
    /// The loader the context was created with.
    pub loader: Arc<dyn Loader>,
    /// Stores loaded source files.
    pub sources: SourceStore,
    /// Stores parsed font faces.
    pub fonts: FontStore,
    /// Stores decoded images.
    pub images: ImageStore,
    /// The compilation root.
    root: PathBuf,
    /// The standard library scope.
    std: Arc<Scope>,
    /// The default styles.
    styles: Arc<StyleMap>,
    /// Cached modules.
    modules: HashMap<SourceId, Module>,
    /// The stack of imported files that led to evaluation of the current file.
    route: Vec<SourceId>,
    /// The dependencies of the current evaluation process.
    deps: Vec<(SourceId, usize)>,
    /// A control flow event that is currently happening.
    flow: Option<Flow>,
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
    /// Returns either a module containing a scope with top-level bindings and
    /// layoutable contents or diagnostics in the form of a vector of error
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

        // Save the old context.
        let prev_flow = self.flow.take();
        let prev_deps = mem::replace(&mut self.deps, vec![(id, source.rev())]);
        self.route.push(id);

        // Evaluate the module.
        let std = self.std.clone();
        let mut scp = Scopes::new(Some(&std));
        let result = ast.eval(self, &mut scp);

        // Restore the old context and handle control flow.
        self.route.pop().unwrap();
        let deps = mem::replace(&mut self.deps, prev_deps);
        if let Some(flow) = mem::replace(&mut self.flow, prev_flow) {
            return Err(flow.forbidden());
        }

        // Assemble the module.
        let module = Module { scope: scp.top, content: result?, deps };

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
        self.evaluate(id)?.content.layout(self)
    }

    /// Resolve a user-entered path to be relative to the compilation
    /// environment's root.
    pub fn locate(&self, path: &str) -> StrResult<PathBuf> {
        if let Some(&id) = self.route.last() {
            if let Some(path) = path.strip_prefix('/') {
                return Ok(self.root.join(path).normalize());
            }

            if let Some(dir) = self.sources.get(id).path().parent() {
                return Ok(dir.join(path).normalize());
            }
        }

        return Err("cannot access file system from here".into());
    }
}

/// A builder for a [`Context`].
///
/// This struct is created by [`Context::builder`].
#[derive(Default)]
pub struct ContextBuilder {
    root: PathBuf,
    std: Option<Arc<Scope>>,
    styles: Option<Arc<StyleMap>>,
}

impl ContextBuilder {
    /// The compilation root, relative to which absolute paths are.
    pub fn root(&mut self, root: impl Into<PathBuf>) -> &mut Self {
        self.root = root.into();
        self
    }

    /// The scope containing definitions that are available everywhere
    /// (the standard library).
    pub fn std(&mut self, std: impl Into<Arc<Scope>>) -> &mut Self {
        self.std = Some(std.into());
        self
    }

    /// The default properties for page size, font selection and so on.
    pub fn styles(&mut self, styles: impl Into<Arc<StyleMap>>) -> &mut Self {
        self.styles = Some(styles.into());
        self
    }

    /// Finish building the context by providing the `loader` used to load
    /// fonts, images, source files and other resources.
    pub fn build(&self, loader: Arc<dyn Loader>) -> Context {
        Context {
            sources: SourceStore::new(Arc::clone(&loader)),
            fonts: FontStore::new(Arc::clone(&loader)),
            images: ImageStore::new(Arc::clone(&loader)),
            loader,
            root: self.root.clone(),
            std: self.std.clone().unwrap_or_else(|| Arc::new(library::new())),
            styles: self.styles.clone().unwrap_or_default(),
            modules: HashMap::new(),
            route: vec![],
            deps: vec![],
            flow: None,
        }
    }
}
