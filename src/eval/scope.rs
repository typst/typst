use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::iter;
use std::sync::Arc;

use parking_lot::RwLock;

use super::{Args, Func, Machine, Node, Value};
use crate::diag::TypResult;
use crate::util::EcoString;

/// A slot where a variable is stored.
pub type Slot = Arc<RwLock<Value>>;

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

    /// Look up the slot of a variable.
    pub fn get(&self, var: &str) -> Option<&Slot> {
        iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .chain(self.base.into_iter())
            .find_map(|scope| scope.get(var))
    }
}

/// A map from variable names to variable slots.
#[derive(Default, Clone)]
pub struct Scope {
    /// The mapping from names to slots.
    values: BTreeMap<EcoString, Slot>,
}

impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a constant variable with a value.
    pub fn def_const(&mut self, var: impl Into<EcoString>, value: impl Into<Value>) {
        let cell = RwLock::new(value.into());

        // Make it impossible to write to this value again.
        std::mem::forget(cell.read());

        self.values.insert(var.into(), Arc::new(cell));
    }

    /// Define a mutable variable with a value.
    pub fn def_mut(&mut self, var: impl Into<EcoString>, value: impl Into<Value>) {
        self.values.insert(var.into(), Arc::new(RwLock::new(value.into())));
    }

    /// Define a variable with a slot.
    pub fn def_slot(&mut self, var: impl Into<EcoString>, slot: Slot) {
        self.values.insert(var.into(), slot);
    }

    /// Define a function through a native rust function.
    pub fn def_fn(
        &mut self,
        name: &'static str,
        func: fn(&mut Machine, &mut Args) -> TypResult<Value>,
    ) {
        self.def_const(name, Func::from_fn(name, func));
    }

    /// Define a function through a native rust node.
    pub fn def_node<T: Node>(&mut self, name: &'static str) {
        self.def_const(name, Func::from_node::<T>(name));
    }

    /// Look up the value of a variable.
    pub fn get(&self, var: &str) -> Option<&Slot> {
        self.values.get(var)
    }

    /// Iterate over all definitions.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Slot)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl Hash for Scope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.values.len().hash(state);
        for (name, value) in self.values.iter() {
            name.hash(state);
            value.read().hash(state);
        }
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Scope ")?;
        f.debug_map()
            .entries(self.values.iter().map(|(k, v)| (k, v.read())))
            .finish()
    }
}
