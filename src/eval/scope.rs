use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::iter;
use std::rc::Rc;

use super::Value;

/// A slot where a variable is stored.
pub type Slot = Rc<RefCell<Value>>;

/// A stack of scopes.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Scopes<'a> {
    /// The active scope.
    top: Scope,
    /// The stack of lower scopes.
    scopes: Vec<Scope>,
    /// The base scope.
    base: Option<&'a Scope>,
}

impl<'a> Scopes<'a> {
    /// Create a new, empty hierarchy of scopes.
    pub fn new() -> Self {
        Self {
            top: Scope::new(),
            scopes: vec![],
            base: None,
        }
    }

    /// Create a new hierarchy of scopes with a base scope.
    pub fn with_base(base: &'a Scope) -> Self {
        Self {
            top: Scope::new(),
            scopes: vec![],
            base: Some(base),
        }
    }

    /// Push a new scope.
    pub fn push(&mut self) {
        self.scopes.push(std::mem::take(&mut self.top));
    }

    /// Pop the topmost scope.
    ///
    /// # Panics
    /// Panics if no scope was pushed.
    pub fn pop(&mut self) {
        self.top = self.scopes.pop().expect("no pushed scope");
    }

    /// Define a constant variable in the active scope.
    pub fn def_const(&mut self, var: impl Into<String>, value: impl Into<Value>) {
        self.top.def_const(var, value);
    }

    /// Define a mutable variable in the active scope.
    pub fn def_mut(&mut self, var: impl Into<String>, value: impl Into<Value>) {
        self.top.def_mut(var, value);
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
    values: HashMap<String, Slot>,
}

impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a constant variable.
    pub fn def_const(&mut self, var: impl Into<String>, value: impl Into<Value>) {
        let cell = RefCell::new(value.into());

        // Make it impossible to write to this value again.
        // FIXME: Use Ref::leak once stable.
        std::mem::forget(cell.borrow());

        self.values.insert(var.into(), Rc::new(cell));
    }

    /// Define a mutable variable.
    pub fn def_mut(&mut self, var: impl Into<String>, value: impl Into<Value>) {
        self.values.insert(var.into(), Rc::new(RefCell::new(value.into())));
    }

    /// Define a variable with a slot.
    pub fn def_slot(&mut self, var: impl Into<String>, slot: Slot) {
        self.values.insert(var.into(), slot);
    }

    /// Look up the value of a variable.
    pub fn get(&self, var: &str) -> Option<&Slot> {
        self.values.get(var)
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.values.fmt(f)
    }
}
