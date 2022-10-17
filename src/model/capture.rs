use super::{Scope, Scopes, Value};
use crate::syntax::ast::TypedNode;
use crate::syntax::{ast, SyntaxNode};

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
    pub fn bind(&mut self, ident: ast::Ident) {
        self.internal.top.define(ident.take(), Value::None);
    }

    /// Capture a variable if it isn't internal.
    pub fn capture(&mut self, ident: ast::Ident) {
        if self.internal.get(&ident).is_err() {
            if let Ok(value) = self.external.get(&ident) {
                self.captures.define_captured(ident.take(), value.clone());
            }
        }
    }

    /// Visit any node and collect all captured variables.
    pub fn visit(&mut self, node: &SyntaxNode) {
        match node.cast() {
            // Every identifier is a potential variable that we need to capture.
            // Identifiers that shouldn't count as captures because they
            // actually bind a new name are handled below (individually through
            // the expressions that contain them).
            Some(ast::Expr::Ident(ident)) => self.capture(ident),

            // Code and content blocks create a scope.
            Some(ast::Expr::Code(_) | ast::Expr::Content(_)) => {
                self.internal.enter();
                for child in node.children() {
                    self.visit(child);
                }
                self.internal.exit();
            }

            // A closure contains parameter bindings, which are bound before the
            // body is evaluated. Care must be taken so that the default values
            // of named parameters cannot access previous parameter bindings.
            Some(ast::Expr::Closure(expr)) => {
                for param in expr.params() {
                    if let ast::Param::Named(named) = param {
                        self.visit(named.expr().as_untyped());
                    }
                }

                for param in expr.params() {
                    match param {
                        ast::Param::Pos(ident) => self.bind(ident),
                        ast::Param::Named(named) => self.bind(named.name()),
                        ast::Param::Sink(ident) => self.bind(ident),
                    }
                }

                self.visit(expr.body().as_untyped());
            }

            // A let expression contains a binding, but that binding is only
            // active after the body is evaluated.
            Some(ast::Expr::Let(expr)) => {
                if let Some(init) = expr.init() {
                    self.visit(init.as_untyped());
                }
                self.bind(expr.binding());
            }

            // A show rule contains a binding, but that binding is only active
            // after the target has been evaluated.
            Some(ast::Expr::Show(show)) => {
                self.visit(show.pattern().as_untyped());
                if let Some(binding) = show.binding() {
                    self.bind(binding);
                }
                self.visit(show.body().as_untyped());
            }

            // A for loop contains one or two bindings in its pattern. These are
            // active after the iterable is evaluated but before the body is
            // evaluated.
            Some(ast::Expr::For(expr)) => {
                self.visit(expr.iter().as_untyped());
                let pattern = expr.pattern();
                if let Some(key) = pattern.key() {
                    self.bind(key);
                }
                self.bind(pattern.value());
                self.visit(expr.body().as_untyped());
            }

            // An import contains items, but these are active only after the
            // path is evaluated.
            Some(ast::Expr::Import(expr)) => {
                self.visit(expr.path().as_untyped());
                if let ast::Imports::Items(items) = expr.imports() {
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
    use crate::syntax::parse;

    #[track_caller]
    fn test(text: &str, result: &[&str]) {
        let mut scopes = Scopes::new(None);
        scopes.top.define("x", 0);
        scopes.top.define("y", 0);
        scopes.top.define("z", 0);

        let mut visitor = CapturesVisitor::new(&scopes);
        let root = parse(text);
        visitor.visit(&root);

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

        // Blocks.
        test("{ let x = 1; { let y = 2; y }; x + y }", &["y"]);
        test("[#let x = 1]#x", &["x"]);
    }
}
