use std::rc::Rc;

use super::*;
use crate::syntax::visit::*;

/// A visitor that captures variable slots.
#[derive(Debug)]
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
            internal: Scopes::new(),
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
        match node {
            Expr::Ident(ident) => {
                // Find out whether the identifier is not locally defined, but
                // captured, and if so, replace it with its value.
                if self.internal.get(ident).is_none() {
                    if let Some(slot) = self.external.get(ident) {
                        self.captures.def_slot(ident.as_str(), Rc::clone(slot));
                    }
                }
            }
            expr => visit_expr(self, expr),
        }
    }

    fn visit_binding(&mut self, id: &'ast Ident) {
        self.internal.def_mut(id.as_str(), Value::None);
    }

    fn visit_enter(&mut self) {
        self.internal.enter();
    }

    fn visit_exit(&mut self) {
        self.internal.exit();
    }
}
