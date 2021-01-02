//! Mapping from identifiers to functions.

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use super::Value;

/// A map from identifiers to functions.
#[derive(Default, Clone, PartialEq)]
pub struct Scope {
    values: HashMap<String, Value>,
}

impl Scope {
    // Create a new empty scope with a fallback function that is invoked when no
    // match is found.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the value of the given variable.
    pub fn get(&self, var: &str) -> Option<&Value> {
        self.values.get(var)
    }

    /// Store the value for the given variable.
    pub fn set(&mut self, var: impl Into<String>, value: impl Into<Value>) {
        self.values.insert(var.into(), value.into());
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.values.fmt(f)
    }
}
