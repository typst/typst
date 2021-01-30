use std::rc::Rc;

use super::*;
use crate::syntax::visit::*;

/// A visitor that replaces all captured variables with their values.
#[derive(Debug)]
pub struct CapturesVisitor<'a> {
    external: &'a Scopes<'a>,
    internal: Scopes<'a>,
}

impl<'a> CapturesVisitor<'a> {
    /// Create a new visitor for the given external scopes.
    pub fn new(external: &'a Scopes) -> Self {
        Self { external, internal: Scopes::default() }
    }
}

impl<'a> Visitor<'a> for CapturesVisitor<'a> {
    fn visit_scope_pre(&mut self) {
        self.internal.push();
    }

    fn visit_scope_post(&mut self) {
        self.internal.pop();
    }

    fn visit_def(&mut self, id: &mut Ident) {
        self.internal.def_mut(id.as_str(), Value::None);
    }

    fn visit_expr(&mut self, expr: &'a mut Expr) {
        if let Expr::Ident(ident) = expr {
            // Find out whether the identifier is not locally defined, but
            // captured, and if so, replace it with its value.
            if self.internal.get(ident).is_none() {
                if let Some(value) = self.external.get(ident) {
                    *expr = Expr::Captured(Rc::clone(&value));
                }
            }
        } else {
            walk_expr(self, expr);
        }
    }
}
