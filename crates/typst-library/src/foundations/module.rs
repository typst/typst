use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use ecow::{EcoString, eco_format};
use typst_syntax::FileId;

use crate::diag::{DeprecationSink, StrResult, bail};
use crate::foundations::{Content, Repr, Scope, Value, ty};

/// A collection of variables and functions that are commonly related to
/// a single theme.
///
/// A module can
/// - be built-in
/// - stem from a [file import]($scripting/#modules)
/// - stem from a [package import]($scripting/#packages) (and thus indirectly
///   its entrypoint file)
/// - result from a call to the [plugin]($plugin) function
///
/// You can access definitions from the module using [field access
/// notation]($scripting/#fields) and interact with it using the [import and
/// include syntaxes]($scripting/#modules).
///
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
///
/// You can check whether a definition is present in a module using the `{in}`
/// operator, with a string on the left-hand side. This can be useful to
/// [conditionally access]($category/foundations/std/#conditional-access)
/// definitions in a module.
///
/// ```example
/// #("table" in std) \
/// #("nope" in std)
/// ```
///
/// Alternatively, it is possible to convert a module to a dictionary, and
/// therefore access its contents dynamically, using the [dictionary
/// constructor]($dictionary/#constructor).
#[ty(cast)]
#[derive(Clone, Hash)]
pub struct Module {
    /// The module's name.
    name: Option<EcoString>,
    /// The reference-counted inner fields.
    inner: Arc<ModuleInner>,
}

/// The internal representation of a [`Module`].
#[derive(Debug, Clone, Hash)]
struct ModuleInner {
    /// The top-level definitions that were bound in this module.
    scope: Scope,
    /// The module's layoutable contents.
    content: Content,
    /// The id of the file which defines the module, if any.
    file_id: Option<FileId>,
}

impl Module {
    /// Create a new module.
    pub fn new(name: impl Into<EcoString>, scope: Scope) -> Self {
        Self {
            name: Some(name.into()),
            inner: Arc::new(ModuleInner {
                scope,
                content: Content::empty(),
                file_id: None,
            }),
        }
    }

    /// Create a new anonymous module without a name.
    pub fn anonymous(scope: Scope) -> Self {
        Self {
            name: None,
            inner: Arc::new(ModuleInner {
                scope,
                content: Content::empty(),
                file_id: None,
            }),
        }
    }

    /// Update the module's name.
    pub fn with_name(mut self, name: impl Into<EcoString>) -> Self {
        self.name = Some(name.into());
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

    /// Update the module's file id.
    pub fn with_file_id(mut self, file_id: FileId) -> Self {
        Arc::make_mut(&mut self.inner).file_id = Some(file_id);
        self
    }

    /// Get the module's name.
    pub fn name(&self) -> Option<&EcoString> {
        self.name.as_ref()
    }

    /// Access the module's scope.
    pub fn scope(&self) -> &Scope {
        &self.inner.scope
    }

    /// Access the module's file id.
    ///
    /// Some modules are not associated with a file, like the built-in modules.
    pub fn file_id(&self) -> Option<FileId> {
        self.inner.file_id
    }

    /// Access the module's scope, mutably.
    pub fn scope_mut(&mut self) -> &mut Scope {
        &mut Arc::make_mut(&mut self.inner).scope
    }

    /// Try to access a definition in the module.
    pub fn field(&self, field: &str, sink: impl DeprecationSink) -> StrResult<&Value> {
        match self.scope().get(field) {
            Some(binding) => Ok(binding.read_checked(sink)),
            None => match &self.name {
                Some(name) => bail!("module `{name}` does not contain `{field}`"),
                None => bail!("module does not contain `{field}`"),
            },
        }
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

impl Repr for Module {
    fn repr(&self) -> EcoString {
        match &self.name {
            Some(module) => eco_format!("<module {module}>"),
            None => "<module>".into(),
        }
    }
}

impl PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && Arc::ptr_eq(&self.inner, &other.inner)
    }
}
