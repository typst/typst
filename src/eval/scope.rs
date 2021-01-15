use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::iter;

use super::Value;

/// A hierarchy of scopes.
#[derive(Debug, Clone, PartialEq)]
pub struct Scopes<'a> {
    /// The active scope.
    top: Scope,
    /// The stack of lower scopes.
    scopes: Vec<Scope>,
    /// The base scope.
    base: &'a Scope,
}

impl<'a> Scopes<'a> {
    /// Create a new hierarchy of scopes.
    pub fn new(base: &'a Scope) -> Self {
        Self { top: Scope::new(), scopes: vec![], base }
    }

    /// Look up the value of a variable in the scopes.
    pub fn get(&self, var: &str) -> Option<&Value> {
        iter::once(&self.top)
            .chain(&self.scopes)
            .chain(iter::once(self.base))
            .find_map(|scope| scope.get(var))
    }

    /// Define a variable in the active scope.
    pub fn define(&mut self, var: impl Into<String>, value: impl Into<Value>) {
        self.top.set(var, value);
    }
}

/// A map from variable names to values.
#[derive(Default, Clone, PartialEq)]
pub struct Scope {
    values: HashMap<String, Value>,
}

impl Scope {
    // Create a new empty scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up the value of a variable.
    pub fn get(&self, var: &str) -> Option<&Value> {
        self.values.get(var)
    }

    /// Store the value for a variable.
    pub fn set(&mut self, var: impl Into<String>, value: impl Into<Value>) {
        self.values.insert(var.into(), value.into());
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.values.fmt(f)
    }
}
