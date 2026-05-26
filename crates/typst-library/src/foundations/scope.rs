use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

use ecow::{EcoString, eco_format};
use indexmap::IndexMap;
use indexmap::map::Entry;
use rustc_hash::FxBuildHasher;
use typst_syntax::Span;

use crate::diag::{
    HintedStrResult, HintedString, SourceDiagnostic, StrResult, WarningSink, error,
};
use crate::engine::Engine;
use crate::foundations::{
    Func, IntoValue, NativeElement, NativeFunc, NativeFuncData, NativeType, Value,
};
use crate::{Category, Feature, Features, Library, World};

/// A stack of scopes.
#[derive(Debug, Default, Clone)]
pub struct Scopes<'a> {
    /// The active scope.
    pub top: Scope,
    /// The stack of lower scopes.
    pub scopes: Vec<Scope>,
    /// The standard library.
    pub base: Option<&'a Library>,
}

impl<'a> Scopes<'a> {
    /// Create a new, empty hierarchy of scopes.
    pub fn new(base: Option<&'a Library>) -> Self {
        Self { top: Scope::new(), scopes: vec![], base }
    }

    /// Enter a new scope.
    pub fn enter(&mut self) {
        self.scopes.push(std::mem::take(&mut self.top));
    }

    /// Exit the topmost scope.
    ///
    /// This panics if no scope was entered.
    pub fn exit(&mut self) {
        self.top = self.scopes.pop().expect("no pushed scope");
    }

    /// Try to access a binding value immutably.
    pub fn get(&self, var: &str, guard: impl BindingGuard) -> HintedStrResult<&Value> {
        self.get_value(var, guard, |library| library.global.scope())
            .map(|res| {
                res.or_cannot(format_args!("access variable `{var}`"))
                    .map_err(HintedString::from)
            })
            .unwrap_or_else(|| Err(unknown_variable(var)))
    }

    /// Try to access a binding value immutably in math.
    pub fn get_in_math(
        &self,
        var: &str,
        guard: impl BindingGuard,
    ) -> HintedStrResult<&Value> {
        self.get_value(var, guard, |library| library.math.scope())
            .map(|res| {
                res.or_cannot(format_args!("access variable `{var}`"))
                    .map_err(HintedString::from)
            })
            .unwrap_or_else(|| {
                Err(unknown_variable_math(
                    var,
                    self.base.is_some_and(|base| base.global.scope().get(var).is_some()),
                ))
            })
    }

    /// Try to access a binding value, a binding is feature-gated, and no other
    /// accessible binding is found, the feature error will be returned.
    /// This exactly mimics how a naive scope implementation would resolve a
    /// variable if the feature-gated items wouldn't exist, while giving a nicer
    /// error message, when an unknown variable error would be thrown instead.
    fn get_value<F>(
        &self,
        var: &str,
        mut guard: impl BindingGuard,
        base_scope: F,
    ) -> Option<Result<&Value, FeatureError>>
    where
        F: Fn(&Library) -> &Scope,
    {
        let mut inaccessible = None;
        std::iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .find_map(|scope| resolve(scope, var, &mut guard, &mut inaccessible))
            .or_else(|| {
                let base = self.base?;
                match resolve(base_scope(base), var, guard, &mut inaccessible) {
                    Some(binding) => Some(binding),
                    None if var == "std" => Some(base.std.read()),
                    None => None,
                }
            })
            .map(Ok)
            .or_else(|| inaccessible.map(Err))
    }

    /// Try to capture a binding.
    pub fn capture(
        &self,
        var: &str,
        guard: &SilentBindingGuard,
    ) -> Option<BindingRead<'_>> {
        self.capture_binding(var, guard, |library| library.global.scope())
    }

    /// Try to capture a binding in math.
    pub fn capture_in_math(
        &self,
        var: &str,
        guard: &SilentBindingGuard,
    ) -> Option<BindingRead<'_>> {
        self.capture_binding(var, guard, |library| library.math.scope())
    }

    /// Returns the binding that will be resolved taking feature gates into
    /// consideration for the variable. If no un-gated binding could be found,
    /// reading from the returned binding might still return a feature error.
    fn capture_binding<F>(
        &self,
        var: &str,
        guard: &SilentBindingGuard,
        base_scope: F,
    ) -> Option<BindingRead<'_>>
    where
        F: Fn(&Library) -> &Scope,
    {
        let mut inaccessible = None;
        std::iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .find_map(|scope| resolve_binding(scope, var, guard, &mut inaccessible))
            .or_else(|| {
                let base = self.base?;
                match resolve_binding(base_scope(base), var, guard, &mut inaccessible) {
                    Some(binding) => Some(binding),
                    None if var == "std" => Some(BindingRead(&base.std)),
                    None => None,
                }
            })
            .or(inaccessible)
    }

    /// Try to access a binding mutably.
    pub fn get_mut(
        &mut self,
        var: &str,
        mut guard: impl BindingGuard,
    ) -> HintedStrResult<&mut Value> {
        let mut inaccessible = None;
        std::iter::once(&mut self.top)
            .chain(&mut self.scopes.iter_mut().rev())
            .find_map(|scope| resolve_mut(scope, var, &mut guard, &mut inaccessible))
            .map(|res| {
                res.or_cannot(format_args!("access variable `{var}`"))
                    .map_err(HintedString::from)
            })
            .or_else(|| {
                let base = self.base?;
                match resolve(base.global.scope(), var, guard, &mut inaccessible) {
                    Some(_) => Some(Err(cannot_mutate_constant(var))),
                    _ if var == "std" => Some(Err(cannot_mutate_constant(var))),
                    _ => None,
                }
            })
            .unwrap_or_else(|| {
                Err(match inaccessible {
                    Some(err) => err.cannot("access variable").into(),
                    None => unknown_variable(var),
                })
            })
    }

    /// Check if an std variable is shadowed.
    pub fn check_std_shadowed(&self, var: &str) -> bool {
        self.base.is_some_and(|base| base.global.scope().get(var).is_some())
            && std::iter::once(&self.top)
                .chain(self.scopes.iter().rev())
                .any(|scope| scope.get(var).is_some())
    }
}

/// Resolves an accessible binding value and stores the first inaccessible
/// binding error while doing so.
fn resolve<'a>(
    scope: &'a Scope,
    var: &str,
    guard: impl BindingGuard,
    inaccessible: &mut Option<FeatureError>,
) -> Option<&'a Value> {
    let binding = scope.get(var)?;

    match binding.read(guard) {
        Ok(value) => Some(value),
        Err(err) => {
            *inaccessible = inaccessible.or(Some(err));
            None
        }
    }
}

/// Resolves an accessible binding value and stores the first inaccessible
/// binding error while doing so.
fn resolve_mut<'a>(
    scope: &'a mut Scope,
    var: &str,
    guard: impl BindingGuard,
    inaccessible: &mut Option<FeatureError>,
) -> Option<Result<&'a mut Value, BindingError>> {
    let binding = scope.get_mut(var)?;

    match binding.write(guard) {
        Ok(value) => Some(Ok(value)),
        Err(BindingError::Feature(err)) => {
            *inaccessible = inaccessible.or(Some(err));
            None
        }
        Err(err) => Some(Err(err)),
    }
}

/// Resolves an accessible binding value and stores the first inaccessible
/// binding while doing so.
fn resolve_binding<'a>(
    scope: &'a Scope,
    var: &str,
    guard: impl BindingGuard,
    inaccessible: &mut Option<BindingRead<'a>>,
) -> Option<BindingRead<'a>> {
    let binding = scope.get(var)?;
    match binding.read(guard) {
        Ok(_) => Some(binding),
        Err(_) => {
            *inaccessible = inaccessible.or(Some(binding));
            None
        }
    }
}

/// A map from binding names to values.
#[derive(Default, Clone)]
pub struct Scope {
    map: IndexMap<EcoString, Binding, FxBuildHasher>,
    deduplicate: bool,
    category: Option<Category>,
}

/// Scope construction.
impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new scope with duplication prevention.
    pub fn deduplicating() -> Self {
        Self { deduplicate: true, ..Default::default() }
    }

    /// Enter a new category.
    pub fn start_category(&mut self, category: Category) {
        self.category = Some(category);
    }

    /// Reset the category.
    pub fn reset_category(&mut self) {
        self.category = None;
    }

    /// Define a native function through a Rust type that shadows the function.
    #[track_caller]
    pub fn define_func<T: NativeFunc>(&mut self) -> &mut Binding {
        let data = T::data();
        self.define(data.name, Func::from(data))
    }

    /// Define a native function with raw function data.
    #[track_caller]
    pub fn define_func_with_data(
        &mut self,
        data: &'static NativeFuncData,
    ) -> &mut Binding {
        self.define(data.name, Func::from(data))
    }

    /// Define a native type.
    #[track_caller]
    pub fn define_type<T: NativeType>(&mut self) -> &mut Binding {
        let ty = T::ty();
        self.define(ty.short_name(), ty)
    }

    /// Define a native element.
    #[track_caller]
    pub fn define_elem<T: NativeElement>(&mut self) -> &mut Binding {
        let elem = T::ELEM;
        self.define(elem.name(), elem)
    }

    /// Define a built-in with compile-time known name and returns a mutable
    /// reference to it.
    ///
    /// When the name isn't compile-time known, you should instead use:
    /// - `Vm::bind` if you already have [`Binding`]
    /// - `Vm::define`  if you only have a [`Value`]
    /// - [`Scope::bind`](Self::bind) if you are not operating in the context of
    ///   a `Vm` or if you are binding to something that is not an AST
    ///   identifier (e.g. when constructing a dynamic
    ///   [`Module`](super::Module))
    #[track_caller]
    pub fn define(&mut self, name: &'static str, value: impl IntoValue) -> &mut Binding {
        #[cfg(debug_assertions)]
        if self.deduplicate && self.map.contains_key(name) {
            panic!("duplicate definition: {name}");
        }

        let mut binding = Binding::detached(value);
        binding.init_info().category = self.category;
        self.bind(name.into(), binding)
    }
}

/// Scope manipulation and access.
impl Scope {
    /// Inserts a binding into this scope and returns a mutable reference to it.
    ///
    /// Prefer `Vm::bind` if you are operating in the context of a `Vm`.
    pub fn bind(&mut self, name: EcoString, binding: Binding) -> &mut Binding {
        match self.map.entry(name) {
            Entry::Occupied(mut entry) => {
                entry.insert(binding);
                entry.into_mut()
            }
            Entry::Vacant(entry) => entry.insert(binding),
        }
    }

    /// Inserts a binding into this scope, if the name is unused.
    ///
    /// Panics if the scope already contains a binding with the same name.
    pub fn prelude(&mut self, name: EcoString, binding: BindingRead<'_>) {
        match self.map.entry(name) {
            Entry::Occupied(_) => {
                panic!("preluding this binding will overwrite an existing value");
            }
            Entry::Vacant(entry) => {
                entry.insert(binding.0.clone());
            }
        }
    }

    /// Mark a binding as captured and insert it into this scope.
    pub fn capture_from(
        &mut self,
        name: EcoString,
        binding: BindingRead<'_>,
        capturer: Capturer,
    ) {
        let captured = Binding {
            kind: BindingKind::Captured(capturer),
            // Reading from the binding, without running any checks is fine,
            // because it's inserted into a scope with it's access check still
            // in place.
            ..binding.0.clone()
        };
        self.bind(name, captured);
    }

    /// Import all bindings from a scope.
    ///
    /// The names of wildcard imports are never explicitly bound. Therefore, if
    /// an imported binding is feature gated, it will only be imported if it
    /// wouldn't shadow a variable in the current scope.
    /// Because this is just a preparation step, a *silent* binding guard is
    /// required, the real checks will be lazily run once the imported binding
    /// is accessed.
    pub fn wildcard_import(&mut self, imported_scope: &Scope, guard: SilentBindingGuard) {
        for (name, read) in imported_scope.iter() {
            let name = name.clone();
            let binding = read.0.clone();
            match read.read_binding(&guard) {
                Ok(_) => {
                    // If the binding will be accessible, overwrite any previous
                    // binding in the scope.
                    self.map.insert(name, binding);
                }
                Err(_) => {
                    // If the binding won't be accessible, only insert it, if
                    // there is no other binding. Otherwise execution behavior
                    // could be changed.
                    self.map.entry(name).or_insert_with(|| binding);
                }
            }
        }
    }

    /// Try to access a binding immutably.
    pub fn get<'a>(&'a self, var: &str) -> Option<BindingRead<'a>> {
        self.map.get(var).map(BindingRead)
    }

    /// Try to access a binding mutably.
    ///
    /// As long as this is stays private, we don't need to do binding acesses
    /// checks here, because all feature gated or deprecated bindings live in
    /// the `std` binding, which is immutable, and thus a cannot modify constant
    /// error will be generated in any case.
    fn get_mut(&mut self, var: &str) -> Option<BindingWrite<'_>> {
        self.map.get_mut(var).map(BindingWrite)
    }

    /// Iterate over all definitions.
    pub fn iter(&self) -> impl Iterator<Item = (&EcoString, BindingRead<'_>)> {
        self.map.iter().map(|(k, b)| (k, BindingRead(b)))
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Scope ")?;
        f.debug_map()
            .entries(self.map.iter().map(|(k, v)| (k, &v.value)))
            .finish()
    }
}

impl Hash for Scope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.map.len());
        for item in &self.map {
            item.hash(state);
        }
        self.deduplicate.hash(state);
        self.category.hash(state);
    }
}

/// Defines the associated scope of a Rust type.
pub trait NativeScope {
    /// The constructor function for the type, if any.
    fn constructor() -> Option<&'static NativeFuncData>;

    /// Get the associated scope for the type.
    fn scope() -> Scope;
}

#[derive(Copy, Clone)]
pub struct BindingRead<'a>(&'a Binding);

impl<'a> BindingRead<'a> {
    /// Try to read the binding value.
    ///
    /// The guard is used to check for deprecation and feature gates.
    ///
    /// # Example
    ///
    /// ```
    /// use typst_library::foundations::{BindingRead, BindingAccess, Value};
    /// use typst_library::diag::{At, SourceResult};
    /// use typst_library::engine::Engine;
    /// use typst_syntax::Span;
    ///
    /// fn read_var(binding: BindingRead<'_>, engine: &mut Engine, span: Span) -> SourceResult<Value> {
    ///     binding.read(engine.binding_guard(span))
    ///         .or_cannot("access variable")
    ///         .at(span)
    ///         .cloned()
    /// }
    /// ```
    pub fn read(self, guard: impl BindingGuard) -> Result<&'a Value, FeatureError> {
        self.read_binding(guard).map(Binding::read)
    }

    /// Try to read the binding.
    ///
    /// The guard is used to check for deprecation and feature gates.
    ///
    /// See [`Self::read`].
    pub fn read_binding(
        self,
        guard: impl BindingGuard,
    ) -> Result<&'a Binding, FeatureError> {
        if self.0.check_access {
            self.0.check_access(guard)?;
        }
        Ok(self.0)
    }

    /// Get the value of this binding without running access checks.
    pub fn unchecked(self, _justification: &'static str) -> &'a Value {
        self.0.read()
    }
}

pub struct BindingWrite<'a>(&'a mut Binding);

impl<'a> BindingWrite<'a> {
    /// Try to write to the binding value.
    ///
    /// This fails if the value is a read-only closure capture, and the guard is
    /// used to check for deprecation and feature gates.
    ///
    /// # Example
    ///
    /// ```
    /// use typst_library::foundations::{BindingWrite, BindingAccess, Value};
    /// use typst_library::diag::{At, SourceResult};
    /// use typst_library::engine::Engine;
    /// use typst_syntax::Span;
    ///
    /// fn write_var<'a>(binding: BindingWrite<'a>, engine: &mut Engine, span: Span) -> SourceResult<&'a mut Value> {
    ///     binding.write(engine.binding_guard(span))
    ///         .or_cannot("access variable")
    ///         .at(span)
    /// }
    /// ```
    pub fn write(self, guard: impl BindingGuard) -> Result<&'a mut Value, BindingError> {
        self.write_binding(guard).map(Binding::write)
    }

    /// Try to write to the binding.
    ///
    /// This fails if the value is a read-only closure capture, and the guard is
    /// used to check for deprecation and feature gates.
    ///
    /// See [`Self::write`].
    pub fn write_binding(
        self,
        guard: impl BindingGuard,
    ) -> Result<&'a mut Binding, BindingError> {
        if self.0.check_access {
            self.0.check_access(guard)?;
        }

        if let BindingKind::Captured(capturer) = self.0.kind {
            return Err(BindingError::Captured(capturer));
        }

        Ok(self.0)
    }
}

/// A bound value with metadata.
#[derive(Debug, Clone, Hash)]
pub struct Binding {
    /// The bound value.
    value: Value,
    /// The kind of binding, determines how the value can be accessed.
    kind: BindingKind,
    /// A span associated with the binding.
    span: Span,
    /// Whether this binding has additional checks that should be performed
    /// before accessing it. This boolean is just an optimization, so the happy
    /// path stays fast. The concrete checks are performed on the properties of
    /// additional binding information in [`Self::info`].
    check_access: bool,
    /// Infrequently accessed properties of the binding that are stored out of
    /// band. These should only ever be set for built-in bindings in the
    /// standard library.
    info: Option<Box<BindingInfo>>,
}

/// The different kinds of slots.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum BindingKind {
    /// A normal, mutable binding.
    Normal,
    /// A captured copy of another variable.
    Captured(Capturer),
}

impl Binding {
    /// Create a new binding with a span marking its definition site.
    pub fn new(value: impl IntoValue, span: Span) -> Self {
        Self {
            value: value.into_value(),
            span,
            kind: BindingKind::Normal,
            check_access: false,
            info: None,
        }
    }

    /// Create a binding without a span.
    pub fn detached(value: impl IntoValue) -> Self {
        Self::new(value, Span::detached())
    }

    /// Sets the category of this binding.
    pub fn with_category(&mut self, category: Category) -> &mut Self {
        self.init_info().category = Some(category);
        self
    }

    /// Gates this binding behind the given [`Feature`].
    pub fn with_feature(&mut self, feature: Feature) -> &mut Self {
        let info = self.init_info();
        info.feature = Some(feature);
        self.check_access = info.has_checked_access();
        self
    }

    /// Marks this binding as deprecated, with the given `message`.
    pub fn with_deprecation(&mut self, deprecation: Deprecation) -> &mut Self {
        let info = self.init_info();
        info.deprecation = Some(deprecation);
        self.check_access = info.has_checked_access();
        self
    }

    fn init_info(&mut self) -> &mut BindingInfo {
        self.info.get_or_insert_default()
    }

    /// Read from the binding.
    pub fn read(&self) -> &Value {
        &self.value
    }

    /// Write to the binding.
    pub fn write(&mut self) -> &mut Value {
        &mut self.value
    }

    /// Check if the binding is gated behind a feature or if it is deprecated.
    #[cold]
    fn check_access(&self, mut guard: impl BindingGuard) -> Result<(), FeatureError> {
        let Some(info) = &self.info else { return Ok(()) };

        if let Some(feature) = info.feature
            && !guard.features().is_enabled(feature)
        {
            return Err(FeatureError(feature));
        }

        if let Some(message) = info.deprecation {
            guard.emit(message.into());
        }

        Ok(())
    }

    /// A span associated with the stored value.
    pub fn span(&self) -> Span {
        self.span
    }

    /// The category of the binding, if any.
    pub fn category(&self) -> Option<Category> {
        self.info.as_ref()?.category
    }

    /// The feature that gates the binding, if any.
    pub fn feature(&self) -> Option<Feature> {
        self.info.as_ref()?.feature
    }

    /// The deprecation of the binding, if any.
    pub fn deprecation(&self) -> Option<Deprecation> {
        self.info.as_ref()?.deprecation
    }
}

/// Infrequently accessed information related to a binding.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
struct BindingInfo {
    /// The category of the binding.
    category: Option<Category>,
    /// A feature required to access this binding.
    feature: Option<Feature>,
    /// The deprecation information if this item is deprecated.
    deprecation: Option<Deprecation>,
}

impl BindingInfo {
    /// Whether the binding info has a property set that requires checking
    /// it when it's accessed.
    fn has_checked_access(self) -> bool {
        self.feature.is_some() || self.deprecation.is_some()
    }
}

/// There was an error accessing a binding.
///
/// A [`Result<T, BindingError>`] can be converted into a [`StrResult`] using
/// the [`BindingAccess`] trait, see [`Binding::read`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BindingError {
    /// The binding cannot be written to, because it is captured.
    Captured(Capturer),
    /// The feature that gates it isn't enabled.
    ///
    /// A [`Result<T, BindingError>`] can be converted into a [`StrResult`]
    /// using the [`BindingAccess`] trait, see [`Binding::read`].
    Feature(FeatureError),
}

impl BindingError {
    pub fn cannot(self, what: impl Display) -> EcoString {
        match self {
            BindingError::Captured(capturer) => {
                error!(
                    "variables from outside the {capturer} are \
                     read-only and cannot be modified",
                )
            }
            BindingError::Feature(error) => error.cannot(what),
        }
    }
}

impl From<FeatureError> for BindingError {
    fn from(v: FeatureError) -> Self {
        Self::Feature(v)
    }
}

/// There was an error accessing a binding.
///
/// A [`Result<T, FeatureError>`] can be converted into a [`StrResult`] using
/// the [`BindingAccess`] trait, see [`Binding::read`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FeatureError(Feature);

impl FeatureError {
    pub fn cannot(self, what: impl Display) -> EcoString {
        let Self(feature) = self;
        error!("cannot {what} because the `{feature}` feature is not enabled")
    }
}

/// Convert a [`BindingError`] to a [`StrResult`] by providing a description of
/// what kind of binding couldn't be accessed.
pub trait BindingAccess<T> {
    /// Add a description of what kind of binding couldn't be accessed.
    fn or_cannot(self, what: impl Display) -> StrResult<T>;
}

impl<T> BindingAccess<T> for Result<T, BindingError> {
    fn or_cannot(self, what: impl Display) -> StrResult<T> {
        self.map_err(|err| err.cannot(what))
    }
}

impl<T> BindingAccess<T> for Result<T, FeatureError> {
    fn or_cannot(self, what: impl Display) -> StrResult<T> {
        self.map_err(|err| err.cannot(what))
    }
}

/// What the variable was captured by.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Capturer {
    /// Captured by a function / closure.
    Function,
    /// Captured by a context expression.
    Context,
}

impl Display for Capturer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Capturer::Function => "function",
            Capturer::Context => "context expression",
        })
    }
}

/// Information about a deprecated binding.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Deprecation {
    /// A deprecation message for the definition.
    message: &'static str,
    /// A version in which the deprecated binding is planned to be removed.
    until: Option<&'static str>,
}

impl Deprecation {
    /// Creates new deprecation info with a default message to display when
    /// emitting the deprecation warning.
    pub fn new() -> Self {
        Self { message: "item is deprecated", until: None }
    }

    /// Set the message to display when emitting the deprecation warning.
    pub fn with_message(mut self, message: &'static str) -> Self {
        self.message = message;
        self
    }

    /// Set the version in which the binding is planned to be removed.
    pub fn with_until(mut self, version: &'static str) -> Self {
        self.until = Some(version);
        self
    }

    /// The message to display when emitting the deprecation warning.
    pub fn message(&self) -> &'static str {
        self.message
    }

    /// The version in which the binding is planned to be removed.
    pub fn until(&self) -> Option<&'static str> {
        self.until
    }
}

impl Default for Deprecation {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Deprecation> for HintedString {
    fn from(deprecation: Deprecation) -> Self {
        HintedString::new(deprecation.message.into()).with_hints(
            deprecation
                .until
                .map(|v| eco_format!("it will be removed in Typst {v}")),
        )
    }
}

/// The error message when trying to mutate a variable from the standard
/// library.
#[cold]
fn cannot_mutate_constant(var: &str) -> HintedString {
    eco_format!("cannot mutate a constant: {var}").into()
}

/// The error message when a variable wasn't found.
#[cold]
fn unknown_variable(var: &str) -> HintedString {
    let mut res = HintedString::new(eco_format!("unknown variable: {var}"));

    if var.contains('-') {
        res.hint(eco_format!(
            "if you meant to use subtraction, \
             try adding spaces around the minus sign{}: `{}`",
            if var.matches('-').count() > 1 { "s" } else { "" },
            var.replace('-', " - ")
        ));
    }

    res
}

/// The error message when a variable wasn't found it math.
#[cold]
fn unknown_variable_math(var: &str, in_global: bool) -> HintedString {
    let mut res = HintedString::new(eco_format!("unknown variable: {var}"));

    if matches!(var, "none" | "auto" | "false" | "true") {
        res.hint(eco_format!(
            "if you meant to use a literal, \
             try adding a hash before it: `#{var}`",
        ));
    } else if in_global {
        res.hint(eco_format!(
            "`{var}` is not available directly in math, but is in the standard library",
        ));
        res.hint(eco_format!(
            "to access `{var}` in code mode you can add a hash: `#{var}`",
        ));
        res.hint(eco_format!(
            "or access `{var}` in math mode by using the `std` module: `std.{var}`",
        ));
    } else {
        res.hint(eco_format!(
            "if you meant to display multiple letters as is, \
             try adding spaces between each letter: `{}`",
            var.chars().flat_map(|c| [' ', c]).skip(1).collect::<EcoString>()
        ));
        res.hint(eco_format!(
            "or if you meant to display this as text, \
             try placing it in quotes: `\"{var}\"`"
        ));
    }

    res
}

/// Provides the currently enabled features when reading from a [`Binding`].
pub trait BindingGuard: WarningSink {
    /// The features enabled in the current [`crate::Library`].
    fn features(&self) -> &Features;

    /// Creates a [`BindingGuard`] that discards emitted warnings.
    fn silent(&self) -> SilentBindingGuard {
        SilentBindingGuard::new(self.features().clone())
    }
}

impl<T: BindingGuard> BindingGuard for &mut T {
    fn features(&self) -> &Features {
        T::features(self)
    }
}

/// Create a [`BindingGuard`] from a [`World`]s libaray, that discards all
/// emitted warnings.
pub trait WorldBindingExt {
    /// Create a [`BindingGuard`] that discards emitted warnings.
    fn silent_binding_guard(&self) -> SilentBindingGuard;
}

impl<T: World + ?Sized> WorldBindingExt for T {
    fn silent_binding_guard(&self) -> SilentBindingGuard {
        SilentBindingGuard { features: self.library().features.clone() }
    }
}

/// A [`BindingGuard`] that emits warnings to the engine's sink.
pub struct NormalBindingGuard<'x, 'y> {
    pub engine: &'x mut Engine<'y>,
    pub span: Span,
}

impl WarningSink for NormalBindingGuard<'_, '_> {
    fn emit(&mut self, message: HintedString) {
        self.engine.sink.warn(
            SourceDiagnostic::warning(self.span, message.message())
                .with_hints(message.hints().iter().cloned()),
        );
    }
}

impl BindingGuard for NormalBindingGuard<'_, '_> {
    fn features(&self) -> &Features {
        &self.engine.library.features
    }
}

/// A [`BindingGuard`] that discards emitted warnings.
#[derive(Clone)]
pub struct SilentBindingGuard {
    features: Features,
}

impl SilentBindingGuard {
    pub fn new(features: Features) -> Self {
        Self { features }
    }
}

impl WarningSink for SilentBindingGuard {
    fn emit(&mut self, _message: HintedString) {
        // Just discard warnings.
    }
}

impl WarningSink for &SilentBindingGuard {
    fn emit(&mut self, _message: HintedString) {
        // Just discard warnings.
    }
}

impl BindingGuard for SilentBindingGuard {
    fn features(&self) -> &Features {
        &self.features
    }
}

impl BindingGuard for &SilentBindingGuard {
    fn features(&self) -> &Features {
        &self.features
    }
}
