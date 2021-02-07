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

    /// Define an internal variable.
    fn define(&mut self, ident: &Ident) {
        self.internal.def_mut(ident.as_str(), Value::None);
    }
}

impl<'ast> Visit<'ast> for CapturesVisitor<'_> {
    fn visit_expr(&mut self, item: &'ast mut Expr) {
        match item {
            Expr::Ident(ident) => {
                // Find out whether the identifier is not locally defined, but
                // captured, and if so, replace it with its value.
                if self.internal.get(ident).is_none() {
                    if let Some(value) = self.external.get(ident) {
                        *item = Expr::Captured(Rc::clone(&value));
                    }
                }
            }
            expr => visit_expr(self, expr),
        }
    }

    fn visit_block(&mut self, item: &'ast mut ExprBlock) {
        // Blocks create a scope except if directly in a template.
        if item.scopes {
            self.internal.push();
        }
        visit_block(self, item);
        if item.scopes {
            self.internal.pop();
        }
    }

    fn visit_template(&mut self, item: &'ast mut ExprTemplate) {
        // Templates always create a scope.
        self.internal.push();
        visit_template(self, item);
        self.internal.pop();
    }

    fn visit_let(&mut self, item: &'ast mut ExprLet) {
        self.define(&item.pat.v);
        visit_let(self, item);
    }

    fn visit_for(&mut self, item: &'ast mut ExprFor) {
        match &mut item.pat.v {
            ForPattern::Value(value) => self.define(value),
            ForPattern::KeyValue(key, value) => {
                self.define(key);
                self.define(value);
            }
        }
        visit_for(self, item);
    }
}
