//! Mapping of function names to function parsers.

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use super::parsing::CallParser;

/// A map from identifiers to function parsers.
pub struct Scope {
    parsers: HashMap<String, Box<CallParser>>,
    fallback: Box<CallParser>,
}

impl Scope {
    /// Create a new empty scope with a fallback parser that is invoked when no
    /// match is found.
    pub fn new(fallback: Box<CallParser>) -> Self {
        Self {
            parsers: HashMap::new(),
            fallback,
        }
    }

    /// Associate the given function name with a dynamic node type.
    pub fn insert(&mut self, name: impl Into<String>, parser: Box<CallParser>) {
        self.parsers.insert(name.into(), parser);
    }

    /// Return the parser with the given name if there is one.
    pub fn get_parser(&self, name: &str) -> Option<&CallParser> {
        self.parsers.get(name).map(AsRef::as_ref)
    }

    /// Return the fallback parser.
    pub fn get_fallback_parser(&self) -> &CallParser {
        &*self.fallback
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_set().entries(self.parsers.keys()).finish()
    }
}
