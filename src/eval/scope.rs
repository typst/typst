use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;

use ecow::EcoString;

use super::{Library, Value};
use crate::diag::StrResult;

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
    #[must_use]
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
    ///
    /// # Errors
    ///
    /// If the variable cannot be found.
    pub fn get(&self, var: &str) -> StrResult<&Value> {
        Ok(std::iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .chain(self.base.map(|base| base.global.scope()))
            .find_map(|scope| scope.get(var))
            .ok_or("unknown variable")?)
    }

    /// Try to access a variable immutably in math.
    ///
    /// # Errors
    ///
    /// If the variable cannot be found.
    pub fn get_in_math(&self, var: &str) -> StrResult<&Value> {
        Ok(std::iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .chain(self.base.map(|base| base.math.scope()))
            .find_map(|scope| scope.get(var))
            .ok_or("unknown variable")?)
    }

    /// Try to access a variable mutably.
    ///
    /// # Errors
    ///
    /// If the variable cannot be found or cannot be mutated.
    pub fn get_mut(&mut self, var: &str) -> StrResult<&mut Value> {
        std::iter::once(&mut self.top)
            .chain(&mut self.scopes.iter_mut().rev())
            .find_map(|scope| scope.get_mut(var))
            .ok_or_else(|| {
                match self.base.and_then(|base| base.global.scope().get(var)) {
                    Some(_) => "cannot mutate a constant",
                    _ => "unknown variable",
                }
            })?
    }
}

/// A map from binding names to values.
#[derive(Default, Clone, Hash)]
pub struct Scope {
    contents: BTreeMap<EcoString, Slot>,
    deduplicating: bool,
}

impl Scope {
    /// Create a new empty scope.
    #[must_use]
    pub fn new() -> Self {
        Self { contents: BTreeMap::new(), deduplicating: false }
    }

    /// Create a new scope with duplication prevention.
    #[must_use]
    pub fn deduplicating() -> Self {
        Self { contents: BTreeMap::new(), deduplicating: true }
    }

    /// Bind a value to a name.
    #[track_caller]
    pub fn define(&mut self, name: impl Into<EcoString>, value: impl Into<Value>) {
        let name = name.into();

        debug_assert!(
            !(self.deduplicating && self.contents.contains_key(&name)),
            "duplicate definition: {name}",
        );

        self.contents.insert(name, Slot::new(value.into(), Kind::Normal));
    }

    /// Define a captured, immutable binding.
    pub fn define_captured(
        &mut self,
        var: impl Into<EcoString>,
        value: impl Into<Value>,
    ) {
        self.contents
            .insert(var.into(), Slot::new(value.into(), Kind::Captured));
    }

    /// Try to access a variable immutably.
    pub fn get(&self, var: &str) -> Option<&Value> {
        self.contents.get(var).map(Slot::read)
    }

    /// Try to access a variable mutably.
    pub fn get_mut(&mut self, var: &str) -> Option<StrResult<&mut Value>> {
        self.contents.get_mut(var).map(Slot::write)
    }

    /// Iterate over all definitions.
    pub fn iter(&self) -> impl Iterator<Item = (&EcoString, &Value)> {
        self.contents.iter().map(|(k, v)| (k, v.read()))
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Scope ")?;
        f.debug_map()
            .entries(self.contents.iter().map(|(k, v)| (k, v.read())))
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
                Err("variables from outside the function are read-only and cannot be modified")?
            }
        }
    }
}
