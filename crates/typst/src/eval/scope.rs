use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};

use ecow::{eco_format, EcoString};
use indexmap::IndexMap;

use super::{
    Func, IntoValue, Library, Module, NativeFunc, NativeFuncData, NativeType, Type, Value,
};
use crate::diag::{bail, StrResult};
use crate::model::{Element, NativeElement};

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

    /// Try to access a variable immutably.
    pub fn get(&self, var: &str) -> StrResult<&Value> {
        std::iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .chain(self.base.map(|base| base.global.scope()))
            .find_map(|scope| scope.get(var))
            .ok_or_else(|| unknown_variable(var))
    }

    /// Try to access a variable immutably in math.
    pub fn get_in_math(&self, var: &str) -> StrResult<&Value> {
        std::iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .chain(self.base.map(|base| base.math.scope()))
            .find_map(|scope| scope.get(var))
            .ok_or_else(|| eco_format!("unknown variable: {}", var))
    }

    /// Try to access a variable mutably.
    pub fn get_mut(&mut self, var: &str) -> StrResult<&mut Value> {
        std::iter::once(&mut self.top)
            .chain(&mut self.scopes.iter_mut().rev())
            .find_map(|scope| scope.get_mut(var))
            .ok_or_else(|| {
                match self.base.and_then(|base| base.global.scope().get(var)) {
                    Some(_) => eco_format!("cannot mutate a constant: {}", var),
                    _ => unknown_variable(var),
                }
            })?
    }
}

/// The error message when a variable is not found.
#[cold]
fn unknown_variable(var: &str) -> EcoString {
    if var.contains('-') {
        eco_format!(
            "unknown variable: {} - if you meant to use subtraction, \
             try adding spaces around the minus sign.",
            var
        )
    } else {
        eco_format!("unknown variable: {}", var)
    }
}

/// A map from binding names to values.
#[derive(Default, Clone)]
pub struct Scope {
    map: IndexMap<EcoString, Slot>,
    deduplicate: bool,
    category: Option<&'static str>,
}

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
    pub fn category(&mut self, name: &'static str) {
        self.category = Some(name);
    }

    /// Reset the category.
    pub fn reset_category(&mut self) {
        self.category = None;
    }

    /// Bind a value to a name.
    #[track_caller]
    pub fn define(&mut self, name: impl Into<EcoString>, value: impl IntoValue) {
        let name = name.into();

        #[cfg(debug_assertions)]
        if self.deduplicate && self.map.contains_key(&name) {
            panic!("duplicate definition: {name}");
        }

        self.map
            .insert(name, Slot::new(value.into_value(), Kind::Normal, self.category));
    }

    /// Define a native function through a Rust type that shadows the function.
    pub fn define_func<T: NativeFunc>(&mut self) {
        let data = T::data();
        self.define(data.name, Func::from(data));
    }

    /// Define a native function with raw function data.
    pub fn define_func_with_data(&mut self, data: &'static NativeFuncData) {
        self.define(data.name, Func::from(data));
    }

    /// Define a native type.
    pub fn define_type<T: NativeType>(&mut self) {
        let data = T::data();
        self.define(data.name, Type::from(data));
    }

    /// Define a native element.
    pub fn define_elem<T: NativeElement>(&mut self) {
        let data = T::data();
        self.define(data.name, Element::from(data));
    }

    /// Define a module.
    pub fn define_module(&mut self, module: Module) {
        self.define(module.name().clone(), module);
    }

    /// Define a captured, immutable binding.
    pub fn define_captured(&mut self, var: impl Into<EcoString>, value: impl IntoValue) {
        self.map.insert(
            var.into(),
            Slot::new(value.into_value(), Kind::Captured, self.category),
        );
    }

    /// Try to access a variable immutably.
    pub fn get(&self, var: &str) -> Option<&Value> {
        self.map.get(var).map(Slot::read)
    }

    /// Try to access a variable mutably.
    pub fn get_mut(&mut self, var: &str) -> Option<StrResult<&mut Value>> {
        self.map.get_mut(var).map(Slot::write)
    }

    /// Get the category of a definition.
    pub fn get_category(&self, var: &str) -> Option<&'static str> {
        self.map.get(var)?.category
    }

    /// Iterate over all definitions.
    pub fn iter(&self) -> impl Iterator<Item = (&EcoString, &Value)> {
        self.map.iter().map(|(k, v)| (k, v.read()))
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

/// A slot where a value is stored.
#[derive(Clone, Hash)]
struct Slot {
    /// The stored value.
    value: Value,
    /// The kind of slot, determines how the value can be accessed.
    kind: Kind,
    /// The category of the slot.
    category: Option<&'static str>,
}

/// The different kinds of slots.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Kind {
    /// A normal, mutable binding.
    Normal,
    /// A captured copy of another variable.
    Captured,
}

impl Slot {
    /// Create a new slot.
    fn new(value: Value, kind: Kind, category: Option<&'static str>) -> Self {
        Self { value, kind, category }
    }

    /// Read the value.
    fn read(&self) -> &Value {
        &self.value
    }

    /// Try to write to the value.
    fn write(&mut self) -> StrResult<&mut Value> {
        match self.kind {
            Kind::Normal => Ok(&mut self.value),
            Kind::Captured => {
                bail!(
                    "variables from outside the function are \
                     read-only and cannot be modified"
                )
            }
        }
    }
}

/// Defines the associated scope of a Rust type.
pub trait NativeScope {
    /// The constructor function for the type, if any.
    fn constructor() -> Option<&'static NativeFuncData>;

    /// Get the associated scope for the type.
    fn scope() -> Scope;
}
