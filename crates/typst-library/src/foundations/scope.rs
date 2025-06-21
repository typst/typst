use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};

use ecow::{eco_format, EcoString};
use indexmap::map::Entry;
use indexmap::IndexMap;
use typst_syntax::Span;

use crate::diag::{bail, DeprecationSink, HintedStrResult, HintedString, StrResult};
use crate::foundations::{
    Element, Func, IntoValue, NativeElement, NativeFunc, NativeFuncData, NativeType,
    Type, Value,
};
use crate::{Category, Library};

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

    /// Try to access a binding immutably.
    pub fn get(&self, var: &str) -> HintedStrResult<&Binding> {
        std::iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .find_map(|scope| scope.get(var))
            .or_else(|| {
                self.base.and_then(|base| match base.global.scope().get(var) {
                    Some(binding) => Some(binding),
                    None if var == "std" => Some(&base.std),
                    None => None,
                })
            })
            .ok_or_else(|| unknown_variable(var))
    }

    /// Try to access a binding mutably.
    pub fn get_mut(&mut self, var: &str) -> HintedStrResult<&mut Binding> {
        std::iter::once(&mut self.top)
            .chain(&mut self.scopes.iter_mut().rev())
            .find_map(|scope| scope.get_mut(var))
            .ok_or_else(|| {
                match self.base.and_then(|base| base.global.scope().get(var)) {
                    Some(_) => cannot_mutate_constant(var),
                    _ if var == "std" => cannot_mutate_constant(var),
                    _ => unknown_variable(var),
                }
            })
    }

    /// Try to access a binding immutably in math.
    pub fn get_in_math(&self, var: &str) -> HintedStrResult<&Binding> {
        std::iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .find_map(|scope| scope.get(var))
            .or_else(|| {
                self.base.and_then(|base| match base.math.scope().get(var) {
                    Some(binding) => Some(binding),
                    None if var == "std" => Some(&base.std),
                    None => None,
                })
            })
            .ok_or_else(|| {
                unknown_variable_math(
                    var,
                    self.base.is_some_and(|base| base.global.scope().get(var).is_some()),
                )
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

/// A map from binding names to values.
#[derive(Default, Clone)]
pub struct Scope {
    map: IndexMap<EcoString, Binding>,
    deduplicate: bool,
    category: Option<Category>,
}

/// Scope construction.
impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Default::default()
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
        let data = T::data();
        self.define(data.name, Type::from(data))
    }

    /// Define a native element.
    #[track_caller]
    pub fn define_elem<T: NativeElement>(&mut self) -> &mut Binding {
        let data = T::data();
        self.define(data.name, Element::from(data))
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
        binding.category = self.category;
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

    /// Try to access a binding immutably.
    pub fn get(&self, var: &str) -> Option<&Binding> {
        self.map.get(var)
    }

    /// Try to access a binding mutably.
    pub fn get_mut(&mut self, var: &str) -> Option<&mut Binding> {
        self.map.get_mut(var)
    }

    /// Iterate over all definitions.
    pub fn iter(&self) -> impl Iterator<Item = (&EcoString, &Binding)> {
        self.map.iter()
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Scope ")?;
        f.debug_map()
            .entries(self.map.iter().map(|(k, v)| (k, v.read())))
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

/// A bound value with metadata.
#[derive(Debug, Clone, Hash)]
pub struct Binding {
    /// The bound value.
    value: Value,
    /// The kind of binding, determines how the value can be accessed.
    kind: BindingKind,
    /// A span associated with the binding.
    span: Span,
    /// The category of the binding.
    category: Option<Category>,
    /// A deprecation message for the definition.
    deprecation: Option<&'static str>,
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
            category: None,
            deprecation: None,
        }
    }

    /// Create a binding without a span.
    pub fn detached(value: impl IntoValue) -> Self {
        Self::new(value, Span::detached())
    }

    /// Marks this binding as deprecated, with the given `message`.
    pub fn deprecated(&mut self, message: &'static str) -> &mut Self {
        self.deprecation = Some(message);
        self
    }

    /// Read the value.
    pub fn read(&self) -> &Value {
        &self.value
    }

    /// Read the value, checking for deprecation.
    ///
    /// As the `sink`
    /// - pass `()` to ignore the message.
    /// - pass `(&mut engine, span)` to emit a warning into the engine.
    pub fn read_checked(&self, sink: impl DeprecationSink) -> &Value {
        if let Some(message) = self.deprecation {
            sink.emit(message);
        }
        &self.value
    }

    /// Try to write to the value.
    ///
    /// This fails if the value is a read-only closure capture.
    pub fn write(&mut self) -> StrResult<&mut Value> {
        match self.kind {
            BindingKind::Normal => Ok(&mut self.value),
            BindingKind::Captured(capturer) => bail!(
                "variables from outside the {} are \
                 read-only and cannot be modified",
                match capturer {
                    Capturer::Function => "function",
                    Capturer::Context => "context expression",
                }
            ),
        }
    }

    /// Create a copy of the binding for closure capturing.
    pub fn capture(&self, capturer: Capturer) -> Self {
        Self {
            kind: BindingKind::Captured(capturer),
            ..self.clone()
        }
    }

    /// A span associated with the stored value.
    pub fn span(&self) -> Span {
        self.span
    }

    /// A deprecation message for the value, if any.
    pub fn deprecation(&self) -> Option<&'static str> {
        self.deprecation
    }

    /// The category of the value, if any.
    pub fn category(&self) -> Option<Category> {
        self.category
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

/// The error message when trying to mutate a variable from the standard
/// library.
#[cold]
fn cannot_mutate_constant(var: &str) -> HintedString {
    eco_format!("cannot mutate a constant: {}", var).into()
}

/// The error message when a variable wasn't found.
#[cold]
fn unknown_variable(var: &str) -> HintedString {
    let mut res = HintedString::new(eco_format!("unknown variable: {}", var));

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
    let mut res = HintedString::new(eco_format!("unknown variable: {}", var));

    if matches!(var, "none" | "auto" | "false" | "true") {
        res.hint(eco_format!(
            "if you meant to use a literal, \
             try adding a hash before it: `#{var}`",
        ));
    } else if in_global {
        res.hint(eco_format!(
            "`{var}` is not available directly in math, \
             try adding a hash before it: `#{var}`",
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
