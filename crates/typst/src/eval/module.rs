use std::fmt::Debug;
use std::sync::Arc;

use ecow::{eco_format, EcoString};

use super::{ty, Content, Scope, Value};
use crate::diag::StrResult;

/// An evaluated module, either built-in or resulting from a file.
///
/// You can access definitions from the module using
/// [field access notation]($scripting/#fields) and interact with it using the
/// [import and include syntaxes]($scripting/#modules).
///
/// # Example
/// ```example
/// <<< #import "utils.typ"
/// <<< #utils.add(2, 5)
///
/// <<< #import utils: sub
/// <<< #sub(1, 4)
/// >>> #7
/// >>>
/// >>> #(-3)
/// ```
#[ty]
#[derive(Debug, Clone, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct Module {
    /// The module's name.
    name: EcoString,
    /// The reference-counted inner fields.
    inner: Arc<Repr>,
}

/// The internal representation.
#[derive(Debug, Clone, Hash)]
struct Repr {
    /// The top-level definitions that were bound in this module.
    scope: Scope,
    /// The module's layoutable contents.
    content: Content,
}

impl Module {
    /// Create a new module.
    pub fn new(name: impl Into<EcoString>, scope: Scope) -> Self {
        Self {
            name: name.into(),
            inner: Arc::new(Repr { scope, content: Content::empty() }),
        }
    }

    /// Update the module's name.
    pub fn with_name(mut self, name: impl Into<EcoString>) -> Self {
        self.name = name.into();
        self
    }

    /// Update the module's scope.
    pub fn with_scope(mut self, scope: Scope) -> Self {
        Arc::make_mut(&mut self.inner).scope = scope;
        self
    }

    /// Update the module's content.
    pub fn with_content(mut self, content: Content) -> Self {
        Arc::make_mut(&mut self.inner).content = content;
        self
    }

    /// Get the module's name.
    pub fn name(&self) -> &EcoString {
        &self.name
    }

    /// Access the module's scope.
    pub fn scope(&self) -> &Scope {
        &self.inner.scope
    }

    /// Access the module's scope, mutably.
    pub fn scope_mut(&mut self) -> &mut Scope {
        &mut Arc::make_mut(&mut self.inner).scope
    }

    /// Try to access a definition in the module.
    pub fn field(&self, name: &str) -> StrResult<&Value> {
        self.scope().get(name).ok_or_else(|| {
            eco_format!("module `{}` does not contain `{name}`", self.name())
        })
    }

    /// Extract the module's content.
    pub fn content(self) -> Content {
        match Arc::try_unwrap(self.inner) {
            Ok(repr) => repr.content,
            Err(arc) => arc.content.clone(),
        }
    }
}

impl super::Repr for Module {
    fn repr(&self) -> EcoString {
        eco_format!("<module {}>", self.name())
    }
}

impl PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && Arc::ptr_eq(&self.inner, &other.inner)
    }
}
