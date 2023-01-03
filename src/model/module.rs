use std::fmt::{self, Debug, Formatter};
use std::path::Path;
use std::sync::Arc;

use super::{Content, Scope};
use crate::util::EcoString;

/// An evaluated module, ready for importing or typesetting.
#[derive(Clone, Hash)]
pub struct Module(Arc<Repr>);

/// The internal representation.
#[derive(Clone, Hash)]
struct Repr {
    /// The module's name.
    name: EcoString,
    /// The top-level definitions that were bound in this module.
    scope: Scope,
    /// The module's layoutable contents.
    content: Content,
}

impl Module {
    /// Create a new, empty module with the given `name`.
    pub fn new(name: impl Into<EcoString>) -> Self {
        Self(Arc::new(Repr {
            name: name.into(),
            scope: Scope::new(),
            content: Content::empty(),
        }))
    }

    /// Create a new module from an evalauted file.
    pub fn evaluated(path: &Path, scope: Scope, content: Content) -> Self {
        let name = path.file_stem().unwrap_or_default().to_string_lossy().into();
        Self(Arc::new(Repr { name, scope, content }))
    }

    /// Get the module's name.
    pub fn name(&self) -> &EcoString {
        &self.0.name
    }

    /// Access the module's scope.
    pub fn scope(&self) -> &Scope {
        &self.0.scope
    }

    /// Extract the module's content.
    pub fn content(self) -> Content {
        match Arc::try_unwrap(self.0) {
            Ok(repr) => repr.content,
            Err(arc) => arc.content.clone(),
        }
    }
}

impl Debug for Module {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "<module {}>", self.name())
    }
}
