//! Mapping from identifiers to functions.

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use super::value::ValueFunc;

/// A map from identifiers to functions.
#[derive(Default, Clone, PartialEq)]
pub struct Scope {
    functions: HashMap<String, ValueFunc>,
}

impl Scope {
    // Create a new empty scope with a fallback function that is invoked when no
    // match is found.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the function with the given name if there is one.
    pub fn get(&self, name: &str) -> Option<&ValueFunc> {
        self.functions.get(name)
    }

    /// Associate the given name with the function.
    pub fn set(&mut self, name: impl Into<String>, function: ValueFunc) {
        self.functions.insert(name.into(), function);
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_set().entries(self.functions.keys()).finish()
    }
}
