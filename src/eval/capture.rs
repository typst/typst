use std::rc::Rc;

use super::{Scope, Scopes, Value};
use crate::syntax::{ClosureParam, Expr, Imports, RedRef};

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

    pub fn visit(&mut self, node: RedRef) {
        let expr: Option<Expr> = node.cast();

        match expr.as_ref() {
            Some(Expr::Let(expr)) => {
                self.visit(expr.init_ref());
                let ident = expr.binding();
                self.internal.def_mut(ident.as_str(), Value::None);
            }
            Some(Expr::Closure(closure)) => {
                for arg in closure.params() {
                    match arg {
                        ClosureParam::Pos(ident) | ClosureParam::Sink(ident) => {
                            self.internal.def_mut(ident.as_str(), Value::None);
                        }
                        ClosureParam::Named(name) => {
                            self.internal.def_mut(name.name().as_str(), Value::None);
                        }
                    }
                }
                self.visit(closure.body_ref());
            }
            Some(Expr::For(forloop)) => {
                let pattern = forloop.pattern();
                self.internal.def_mut(pattern.value().as_str(), Value::None);

                if let Some(key) = pattern.key() {
                    self.internal.def_mut(key.as_str(), Value::None);
                }
                self.visit(forloop.body_ref());
            }
            Some(Expr::Import(import)) => {
                if let Imports::Idents(idents) = import.imports() {
                    for ident in idents {
                        self.internal.def_mut(ident.as_str(), Value::None);
                    }
                }
            }
            Some(Expr::Ident(ident)) => {
                if self.internal.get(ident.as_str()).is_none() {
                    if let Some(slot) = self.external.get(ident.as_str()) {
                        self.captures.def_slot(ident.as_str(), Rc::clone(slot));
                    }
                }
            }
            _ => {}
        }

        match expr.as_ref() {
            Some(Expr::Let(_)) | Some(Expr::For(_)) | Some(Expr::Closure(_)) => {}

            Some(Expr::Block(_)) => {
                self.internal.enter();
                for child in node.children() {
                    self.visit(child);
                }
                self.internal.exit();
            }

            Some(Expr::Template(_)) => {
                self.internal.enter();
                for child in node.children() {
                    self.visit(child);
                }
                self.internal.exit();
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
