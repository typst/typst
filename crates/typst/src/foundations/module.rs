use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use ecow::{eco_format, EcoString};

use crate::diag::StrResult;
use crate::foundations::{repr, ty, Content, Scope, Value};
use crate::util::PicoStr;

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
#[ty(cast)]
#[derive(Clone, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct Module {
    /// The module's name.
    name: PicoStr,
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
    pub fn new(name: impl Into<PicoStr>, scope: Scope) -> Self {
        Self {
            name: name.into(),
            inner: Arc::new(Repr { scope, content: Content::empty() }),
        }
    }

    /// Update the module's name.
    pub fn with_name(mut self, name: impl Into<PicoStr>) -> Self {
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
    pub fn name(&self) -> PicoStr {
        self.name
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
    pub fn field(&self, name: impl Into<PicoStr>) -> StrResult<&Value> {
        let name = name.into();
        self.scope().get(name).ok_or_else(|| {
            eco_format!(
                "module `{}` does not contain `{}`",
                self.name().resolve(),
                name.resolve()
            )
        })
    }

    pub fn field_index(&self, name: impl Into<PicoStr>) -> StrResult<usize> {
        let name = name.into();
        self.scope().get_index(name).ok_or_else(|| {
            eco_format!(
                "module `{}` does not contain `{}`",
                self.name().resolve(),
                name.resolve()
            )
        })
    }

    pub fn field_by_id(&self, id: usize) -> StrResult<&Value> {
        self.scope().get_by_id(id).ok_or_else(|| {
            eco_format!("module `{}` does not contain `{}`", self.name().resolve(), id)
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

impl Debug for Module {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Module")
            .field("name", &self.name)
            .field("scope", &self.inner.scope)
            .field("content", &self.inner.content)
            .finish()
    }
}

impl repr::Repr for Module {
    fn repr(&self) -> EcoString {
        eco_format!("<module {}>", self.name().resolve())
    }
}

impl PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && Arc::ptr_eq(&self.inner, &other.inner)
    }
}
