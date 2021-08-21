use std::rc::Rc;

use super::{Scope, Scopes, Value};
use crate::syntax::visit::{immutable::visit_expr, Visit};
use crate::syntax::{Expr, Ident};

/// A visitor that captures variable slots.
pub struct CapturesVisitor<'a> {
    external: &'a Scopes<'a>,
    internal: Scopes<'a>,
    captures: Scope,
}

impl<'a> CapturesVisitor<'a> {
    /// Create a new visitor for the given external scopes.
    pub fn new(external: &'a Scopes) -> Self {
        Self {
            external,
            internal: Scopes::new(None),
            captures: Scope::new(),
        }
    }

    /// Return the scope of captured variables.
    pub fn finish(self) -> Scope {
        self.captures
    }
}

impl<'ast> Visit<'ast> for CapturesVisitor<'_> {
    fn visit_expr(&mut self, node: &'ast Expr) {
        if let Expr::Ident(ident) = node {
            // Find out whether the name is not locally defined and if so if it
            // can be captured.
            if self.internal.get(ident).is_none() {
                if let Some(slot) = self.external.get(ident) {
                    self.captures.def_slot(ident.as_str(), Rc::clone(slot));
                }
            }
        } else {
            visit_expr(self, node);
        }
    }

    fn visit_binding(&mut self, ident: &'ast Ident) {
        self.internal.def_mut(ident.as_str(), Value::None);
    }

    fn visit_enter(&mut self) {
        self.internal.enter();
    }

    fn visit_exit(&mut self) {
        self.internal.exit();
    }
}
