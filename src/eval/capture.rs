use std::rc::Rc;

use super::{Scope, Scopes};
use crate::syntax::{NodeKind, RedRef};

/// A visitor that captures variable slots.
pub struct CapturesVisitor<'a> {
    external: &'a Scopes<'a>,
    captures: Scope,
}

impl<'a> CapturesVisitor<'a> {
    /// Create a new visitor for the given external scopes.
    pub fn new(external: &'a Scopes) -> Self {
        Self { external, captures: Scope::new() }
    }

    pub fn visit(&mut self, node: RedRef) {
        match node.kind() {
            NodeKind::Ident(ident) => {
                if let Some(slot) = self.external.get(ident.as_str()) {
                    self.captures.def_slot(ident.as_str(), Rc::clone(slot));
                }
            }
            _ => {
                for child in node.children() {
                    self.visit(child);
                }
            }
        }
    }

    /// Return the scope of captured variables.
    pub fn finish(self) -> Scope {
        self.captures
    }
}
