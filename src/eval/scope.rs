use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;

use ecow::{eco_format, EcoString};

use super::{IntoValue, Library, Value};
use crate::diag::{bail, StrResult};

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
        eco_format!("unknown variable: {} - if you meant to use subtraction, try adding spaces around the minus sign.", var)
    } else {
        eco_format!("unknown variable: {}", var)
    }
}

/// A map from binding names to values.
#[derive(Default, Clone, Hash)]
pub struct Scope(BTreeMap<EcoString, Slot>, bool);

impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Self(BTreeMap::new(), false)
    }

    /// Create a new scope with duplication prevention.
    pub fn deduplicating() -> Self {
        Self(BTreeMap::new(), true)
    }

    /// Bind a value to a name.
    #[track_caller]
    pub fn define(&mut self, name: impl Into<EcoString>, value: impl IntoValue) {
        let name = name.into();

        #[cfg(debug_assertions)]
        if self.1 && self.0.contains_key(&name) {
            panic!("duplicate definition: {name}");
        }

        self.0.insert(name, Slot::new(value.into_value(), Kind::Normal));
    }

    /// Define a captured, immutable binding.
    pub fn define_captured(&mut self, var: impl Into<EcoString>, value: impl IntoValue) {
        self.0
            .insert(var.into(), Slot::new(value.into_value(), Kind::Captured));
    }

    /// Try to access a variable immutably.
    pub fn get(&self, var: &str) -> Option<&Value> {
        self.0.get(var).map(Slot::read)
    }

    /// Try to access a variable mutably.
    pub fn get_mut(&mut self, var: &str) -> Option<StrResult<&mut Value>> {
        self.0.get_mut(var).map(Slot::write)
    }

    /// Iterate over all definitions.
    pub fn iter(&self) -> impl Iterator<Item = (&EcoString, &Value)> {
        self.0.iter().map(|(k, v)| (k, v.read()))
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Scope ")?;
        f.debug_map()
            .entries(self.0.iter().map(|(k, v)| (k, v.read())))
            .finish()
    }
}

/// A slot where a value is stored.
#[derive(Clone, Hash)]
struct Slot {
    /// The stored value.
    value: Value,
    /// The kind of slot, determines how the value can be accessed.
    kind: Kind,
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
    fn new(value: Value, kind: Kind) -> Self {
        Self { value, kind }
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
                bail!("variables from outside the function are read-only and cannot be modified")
            }
        }
    }
}
