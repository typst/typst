use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::iter;
use std::rc::Rc;

use super::{EcoString, EvalContext, FuncArgs, Function, Value};

/// A slot where a variable is stored.
pub type Slot = Rc<RefCell<Value>>;

/// A stack of scopes.
#[derive(Debug, Default, Clone, PartialEq)]
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

    /// Define a constant variable with a value in the active scope.
    pub fn def_const(&mut self, var: impl Into<EcoString>, value: impl Into<Value>) {
        self.top.def_const(var, value);
    }

    /// Define a mutable variable with a value in the active scope.
    pub fn def_mut(&mut self, var: impl Into<EcoString>, value: impl Into<Value>) {
        self.top.def_mut(var, value);
    }

    /// Define a variable with a slot in the active scope.
    pub fn def_slot(&mut self, var: impl Into<EcoString>, slot: Slot) {
        self.top.def_slot(var, slot);
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
#[derive(Default, Clone, PartialEq)]
pub struct Scope {
    /// The mapping from names to slots.
    values: HashMap<EcoString, Slot>,
}

impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a constant variable with a value.
    pub fn def_const(&mut self, var: impl Into<EcoString>, value: impl Into<Value>) {
        let cell = RefCell::new(value.into());

        // Make it impossible to write to this value again.
        // FIXME: Use Ref::leak once stable.
        std::mem::forget(cell.borrow());

        self.values.insert(var.into(), Rc::new(cell));
    }

    /// Define a constant function.
    pub fn def_func<F>(&mut self, name: impl Into<EcoString>, f: F)
    where
        F: Fn(&mut EvalContext, &mut FuncArgs) -> Value + 'static,
    {
        let name = name.into();
        self.def_const(name.clone(), Function::new(Some(name), f));
    }

    /// Define a mutable variable with a value.
    pub fn def_mut(&mut self, var: impl Into<EcoString>, value: impl Into<Value>) {
        self.values.insert(var.into(), Rc::new(RefCell::new(value.into())));
    }

    /// Define a variable with a slot.
    pub fn def_slot(&mut self, var: impl Into<EcoString>, slot: Slot) {
        self.values.insert(var.into(), slot);
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

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.values.fmt(f)
    }
}
