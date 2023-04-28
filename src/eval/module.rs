use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use ecow::{eco_format, EcoString};

use super::{Content, Scope, Value};
use crate::diag::StrResult;

/// An evaluated module, ready for importing or typesetting.
#[derive(Clone, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
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
    /// Create a new module.
    pub fn new(name: impl Into<EcoString>) -> Self {
        Self(Arc::new(Repr {
            name: name.into(),
            scope: Scope::new(),
            content: Content::empty(),
        }))
    }

    /// Update the module's scope.
    pub fn with_scope(mut self, scope: Scope) -> Self {
        Arc::make_mut(&mut self.0).scope = scope;
        self
    }

    /// Update the module's content.
    pub fn with_content(mut self, content: Content) -> Self {
        Arc::make_mut(&mut self.0).content = content;
        self
    }

    /// Get the module's name.
    pub fn name(&self) -> &EcoString {
        &self.0.name
    }

    /// Access the module's scope.
    pub fn scope(&self) -> &Scope {
        &self.0.scope
    }

    /// Access the module's scope, mutably.
    pub fn scope_mut(&mut self) -> &mut Scope {
        &mut Arc::make_mut(&mut self.0).scope
    }

    /// Try to access a definition in the module.
    pub fn get(&self, name: &str) -> StrResult<&Value> {
        self.scope().get(name).ok_or_else(|| {
            eco_format!("module `{}` does not contain `{name}`", self.name())
        })
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

impl PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
