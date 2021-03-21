use std::rc::Rc;

use super::{Scope, Scopes, Value};
use crate::syntax::visit::{visit_expr, Visit};
use crate::syntax::{Expr, Ident, Node};

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

    /// Find out whether the name is not locally defined and if so if it can be
    /// captured.
    fn process(&mut self, name: &str) {
        if self.internal.get(name).is_none() {
            if let Some(slot) = self.external.get(name) {
                self.captures.def_slot(name, Rc::clone(slot));
            }
        }
    }
}

impl<'ast> Visit<'ast> for CapturesVisitor<'_> {
    fn visit_node(&mut self, node: &'ast Node) {
        match node {
            Node::Text(_) => {}
            Node::Space => {}
            Node::Linebreak(_) => self.process(Node::LINEBREAK),
            Node::Parbreak(_) => self.process(Node::PARBREAK),
            Node::Strong(_) => self.process(Node::STRONG),
            Node::Emph(_) => self.process(Node::EMPH),
            Node::Heading(_) => self.process(Node::HEADING),
            Node::Raw(_) => self.process(Node::RAW),
            Node::Expr(expr) => self.visit_expr(expr),
        }
    }

    fn visit_expr(&mut self, node: &'ast Expr) {
        match node {
            Expr::Ident(ident) => self.process(ident),
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
