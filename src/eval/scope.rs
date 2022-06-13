use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;

use super::{Args, Func, Machine, Node, Value};
use crate::diag::{StrResult, TypResult};
use crate::util::EcoString;

/// A stack of scopes.
#[derive(Debug, Default, Clone)]
pub struct Scopes<'a> {
    /// The active scope.
    pub top: Scope,
    /// The stack of lower scopes.
    pub scopes: Vec<Scope>,
    /// The base scope.
    pub base: Option<&'a Scope>,
}

impl<'a> Scopes<'a> {
    /// Create a new, empty hierarchy of scopes.
    pub fn new(base: Option<&'a Scope>) -> Self {
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
        Ok(std::iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .chain(self.base.into_iter())
            .find_map(|scope| scope.get(var))
            .ok_or("unknown variable")?)
    }

    /// Try to access a variable mutably.
    pub fn get_mut(&mut self, var: &str) -> StrResult<&mut Value> {
        std::iter::once(&mut self.top)
            .chain(&mut self.scopes.iter_mut().rev())
            .find_map(|scope| scope.get_mut(var))
            .ok_or_else(|| {
                if self.base.map_or(false, |base| base.get(var).is_some()) {
                    "cannot mutate a constant"
                } else {
                    "unknown variable"
                }
            })?
    }
}

/// A map from binding names to values.
#[derive(Default, Clone, Hash)]
pub struct Scope(BTreeMap<EcoString, Slot>);

impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a value to a name.
    pub fn define(&mut self, name: impl Into<EcoString>, value: impl Into<Value>) {
        self.0.insert(name.into(), Slot::new(value.into(), Kind::Normal));
    }

    /// Define a function through a native rust function.
    pub fn def_fn(
        &mut self,
        name: &'static str,
        func: fn(&mut Machine, &mut Args) -> TypResult<Value>,
    ) {
        self.define(name, Func::from_fn(name, func));
    }

    /// Define a function through a native rust node.
    pub fn def_node<T: Node>(&mut self, name: &'static str) {
        self.define(name, Func::from_node::<T>(name));
    }

    /// Define a captured, immutable binding.
    pub fn define_captured(
        &mut self,
        var: impl Into<EcoString>,
        value: impl Into<Value>,
    ) {
        self.0.insert(var.into(), Slot::new(value.into(), Kind::Captured));
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
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.read()))
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
            Kind::Captured => Err("cannot mutate a captured variable")?,
        }
    }
}
