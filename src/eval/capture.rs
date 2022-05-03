use std::sync::Arc;

use super::{Scope, Scopes, Value};
use crate::syntax::ast::{ClosureParam, Expr, Ident, Imports, TypedNode};
use crate::syntax::RedRef;

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

    /// Bind a new internal variable.
    pub fn bind(&mut self, ident: Ident) {
        self.internal.top.def_mut(ident.take(), Value::None);
    }

    /// Capture a variable if it isn't internal.
    pub fn capture(&mut self, ident: Ident) {
        if self.internal.get(&ident).is_none() {
            if let Some(slot) = self.external.get(&ident) {
                self.captures.def_slot(ident.take(), Arc::clone(slot));
            }
        }
    }

    /// Visit any node and collect all captured variables.
    pub fn visit(&mut self, node: RedRef) {
        match node.cast() {
            // Every identifier is a potential variable that we need to capture.
            // Identifiers that shouldn't count as captures because they
            // actually bind a new name are handled further below (individually
            // through the expressions that contain them).
            Some(Expr::Ident(ident)) => self.capture(ident),

            // Code and content blocks create a scope.
            Some(Expr::Code(_) | Expr::Content(_)) => {
                self.internal.enter();
                for child in node.children() {
                    self.visit(child);
                }
                self.internal.exit();
            }

            // A closure contains parameter bindings, which are bound before the
            // body is evaluated. Care must be taken so that the default values
            // of named parameters cannot access previous parameter bindings.
            Some(Expr::Closure(expr)) => {
                for param in expr.params() {
                    if let ClosureParam::Named(named) = param {
                        self.visit(named.expr().as_red());
                    }
                }

                for param in expr.params() {
                    match param {
                        ClosureParam::Pos(ident) => self.bind(ident),
                        ClosureParam::Named(named) => self.bind(named.name()),
                        ClosureParam::Sink(ident) => self.bind(ident),
                    }
                }

                self.visit(expr.body().as_red());
            }

            // A let expression contains a binding, but that binding is only
            // active after the body is evaluated.
            Some(Expr::Let(expr)) => {
                if let Some(init) = expr.init() {
                    self.visit(init.as_red());
                }
                self.bind(expr.binding());
            }

            // A show rule contains a binding, but that binding is only active
            // after the target has been evaluated.
            Some(Expr::Show(show)) => {
                self.visit(show.pattern().as_red());
                if let Some(binding) = show.binding() {
                    self.bind(binding);
                }
                self.visit(show.body().as_red());
            }

            // A for loop contains one or two bindings in its pattern. These are
            // active after the iterable is evaluated but before the body is
            // evaluated.
            Some(Expr::For(expr)) => {
                self.visit(expr.iter().as_red());
                let pattern = expr.pattern();
                if let Some(key) = pattern.key() {
                    self.bind(key);
                }
                self.bind(pattern.value());
                self.visit(expr.body().as_red());
            }

            // An import contains items, but these are active only after the
            // path is evaluated.
            Some(Expr::Import(expr)) => {
                self.visit(expr.path().as_red());
                if let Imports::Items(items) = expr.imports() {
                    for item in items {
                        self.bind(item);
                    }
                }
            }

            // Everything else is traversed from left to right.
            _ => {
                for child in node.children() {
                    self.visit(child);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::source::SourceId;
    use crate::syntax::RedNode;

    #[track_caller]
    fn test(src: &str, result: &[&str]) {
        let green = parse(src);
        let red = RedNode::from_root(green, SourceId::from_raw(0));

        let mut scopes = Scopes::new(None);
        scopes.top.def_const("x", 0);
        scopes.top.def_const("y", 0);
        scopes.top.def_const("z", 0);

        let mut visitor = CapturesVisitor::new(&scopes);
        visitor.visit(red.as_ref());

        let captures = visitor.finish();
        let mut names: Vec<_> = captures.iter().map(|(k, _)| k).collect();
        names.sort();

        assert_eq!(names, result);
    }

    #[test]
    fn test_captures() {
        // Let binding and function definition.
        test("#let x = x", &["x"]);
        test("#let x; {x + y}", &["y"]);
        test("#let f(x, y) = x + y", &[]);

        // Closure with different kinds of params.
        test("{(x, y) => x + z}", &["z"]);
        test("{(x: y, z) => x + z}", &["y"]);
        test("{(..x) => x + y}", &["y"]);
        test("{(x, y: x + z) => x + y}", &["x", "z"]);

        // Show rule.
        test("#show x: y as x", &["y"]);
        test("#show x: y as x + z", &["y", "z"]);
        test("#show x: x as x", &["x"]);

        // For loop.
        test("#for x in y { x + z }", &["y", "z"]);
        test("#for x, y in y { x + y }", &["y"]);

        // Import.
        test("#import x, y from z", &["z"]);
        test("#import x, y, z from x + y", &["x", "y"]);

        // Scoping.
        test("{ let x = 1; { let y = 2; y }; x + y }", &["y"]);
        test("[#let x = 1]#x", &["x"]);
    }
}
