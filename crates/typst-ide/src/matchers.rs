use ecow::EcoString;
use typst::foundations::{Module, Value};
use typst::syntax::ast::AstNode;
use typst::syntax::{ast, LinkedNode, Span, SyntaxKind};

use crate::{analyze_import, IdeWorld};

/// Find the named items starting from the given position.
pub fn named_items<T>(
    world: &dyn IdeWorld,
    position: LinkedNode,
    mut recv: impl FnMut(NamedItem) -> Option<T>,
) -> Option<T> {
    let mut ancestor = Some(position);
    while let Some(node) = &ancestor {
        let mut sibling = Some(node.clone());
        while let Some(node) = &sibling {
            if let Some(v) = node.cast::<ast::LetBinding>() {
                let kind = if matches!(v.kind(), ast::LetBindingKind::Closure(..)) {
                    NamedItem::Fn
                } else {
                    NamedItem::Var
                };
                for ident in v.kind().bindings() {
                    if let Some(res) = recv(kind(ident)) {
                        return Some(res);
                    }
                }
            }

            if let Some(v) = node.cast::<ast::ModuleImport>() {
                let imports = v.imports();
                let source = v.source();

                let source_value = node
                    .find(source.span())
                    .and_then(|source| analyze_import(world, &source));
                let source_value = source_value.as_ref();

                let module = source_value.and_then(|value| match value {
                    Value::Module(module) => Some(module),
                    _ => None,
                });

                let name_and_span = match (imports, v.new_name()) {
                    // ```plain
                    // import "foo" as name
                    // import "foo" as name: ..
                    // ```
                    (_, Some(name)) => Some((name.get().clone(), name.span())),
                    // ```plain
                    // import "foo"
                    // ```
                    (None, None) => v.bare_name().ok().map(|name| (name, source.span())),
                    // ```plain
                    // import "foo": ..
                    // ```
                    (Some(..), None) => None,
                };

                // Seeing the module itself.
                if let Some((name, span)) = name_and_span {
                    if let Some(res) = recv(NamedItem::Module(&name, span, module)) {
                        return Some(res);
                    }
                }

                // Seeing the imported items.
                match imports {
                    // ```plain
                    // import "foo";
                    // ```
                    None => {}
                    // ```plain
                    // import "foo": *;
                    // ```
                    Some(ast::Imports::Wildcard) => {
                        if let Some(scope) = source_value.and_then(Value::scope) {
                            for (name, binding) in scope.iter() {
                                let item = NamedItem::Import(
                                    name,
                                    binding.span(),
                                    Some(binding.read()),
                                );
                                if let Some(res) = recv(item) {
                                    return Some(res);
                                }
                            }
                        }
                    }
                    // ```plain
                    // import "foo": items;
                    // ```
                    Some(ast::Imports::Items(items)) => {
                        for item in items.iter() {
                            let mut iter = item.path().iter();
                            let mut binding = source_value
                                .and_then(Value::scope)
                                .zip(iter.next())
                                .and_then(|(scope, first)| scope.get(&first));

                            for ident in iter {
                                binding = binding.and_then(|binding| {
                                    binding.read().scope()?.get(&ident)
                                });
                            }

                            let bound = item.bound_name();
                            let (span, value) = match binding {
                                Some(binding) => (binding.span(), Some(binding.read())),
                                None => (bound.span(), None),
                            };

                            let item = NamedItem::Import(bound.get(), span, value);
                            if let Some(res) = recv(item) {
                                return Some(res);
                            }
                        }
                    }
                }
            }

            sibling = node.prev_sibling();
        }

        if let Some(parent) = node.parent() {
            if let Some(v) = parent.cast::<ast::ForLoop>() {
                if node.prev_sibling_kind() != Some(SyntaxKind::In) {
                    let pattern = v.pattern();
                    for ident in pattern.bindings() {
                        if let Some(res) = recv(NamedItem::Var(ident)) {
                            return Some(res);
                        }
                    }
                }
            }

            if let Some(v) = parent.cast::<ast::Closure>().filter(|v| {
                // Check if the node is in the body of the closure.
                let body = parent.find(v.body().span());
                body.is_some_and(|n| n.find(node.span()).is_some())
            }) {
                for param in v.params().children() {
                    match param {
                        ast::Param::Pos(pattern) => {
                            for ident in pattern.bindings() {
                                if let Some(t) = recv(NamedItem::Var(ident)) {
                                    return Some(t);
                                }
                            }
                        }
                        ast::Param::Named(n) => {
                            if let Some(t) = recv(NamedItem::Var(n.name())) {
                                return Some(t);
                            }
                        }
                        ast::Param::Spread(s) => {
                            if let Some(sink_ident) = s.sink_ident() {
                                if let Some(t) = recv(NamedItem::Var(sink_ident)) {
                                    return Some(t);
                                }
                            }
                        }
                    }
                }
            }

            ancestor = Some(parent.clone());
            continue;
        }

        break;
    }

    None
}

/// An item that is named.
pub enum NamedItem<'a> {
    /// A variable item.
    Var(ast::Ident<'a>),
    /// A function item.
    Fn(ast::Ident<'a>),
    /// A (imported) module.
    Module(&'a EcoString, Span, Option<&'a Module>),
    /// An imported item.
    Import(&'a EcoString, Span, Option<&'a Value>),
}

impl<'a> NamedItem<'a> {
    pub(crate) fn name(&self) -> &'a EcoString {
        match self {
            NamedItem::Var(ident) => ident.get(),
            NamedItem::Fn(ident) => ident.get(),
            NamedItem::Module(name, _, _) => name,
            NamedItem::Import(name, _, _) => name,
        }
    }

    pub(crate) fn value(&self) -> Option<Value> {
        match self {
            NamedItem::Var(..) | NamedItem::Fn(..) => None,
            NamedItem::Module(_, _, value) => value.cloned().map(Value::Module),
            NamedItem::Import(_, _, value) => value.cloned(),
        }
    }

    pub(crate) fn span(&self) -> Span {
        match *self {
            NamedItem::Var(name) | NamedItem::Fn(name) => name.span(),
            NamedItem::Module(_, span, _) => span,
            NamedItem::Import(_, span, _) => span,
        }
    }
}

/// Categorize an expression into common classes IDE functionality can operate
/// on.
pub fn deref_target(node: LinkedNode) -> Option<DerefTarget<'_>> {
    // Move to the first ancestor that is an expression.
    let mut ancestor = node;
    while !ancestor.is::<ast::Expr>() {
        ancestor = ancestor.parent()?.clone();
    }

    // Identify convenient expression kinds.
    let expr_node = ancestor;
    let expr = expr_node.cast::<ast::Expr>()?;
    Some(match expr {
        ast::Expr::Label(_) => DerefTarget::Label(expr_node),
        ast::Expr::Ref(_) => DerefTarget::Ref(expr_node),
        ast::Expr::FuncCall(call) => {
            DerefTarget::Callee(expr_node.find(call.callee().span())?)
        }
        ast::Expr::SetRule(set) => {
            DerefTarget::Callee(expr_node.find(set.target().span())?)
        }
        ast::Expr::Ident(_) | ast::Expr::MathIdent(_) | ast::Expr::FieldAccess(_) => {
            DerefTarget::VarAccess(expr_node)
        }
        ast::Expr::Str(_) => {
            let parent = expr_node.parent()?;
            if parent.kind() == SyntaxKind::ModuleImport {
                DerefTarget::ImportPath(expr_node)
            } else if parent.kind() == SyntaxKind::ModuleInclude {
                DerefTarget::IncludePath(expr_node)
            } else {
                DerefTarget::Code(expr_node)
            }
        }
        _ if expr.hash()
            || matches!(expr_node.kind(), SyntaxKind::MathIdent | SyntaxKind::Error) =>
        {
            DerefTarget::Code(expr_node)
        }
        _ => return None,
    })
}

/// Classes of expressions that can be operated on by IDE functionality.
#[derive(Debug, Clone)]
pub enum DerefTarget<'a> {
    /// A variable access expression.
    ///
    /// It can be either an identifier or a field access.
    VarAccess(LinkedNode<'a>),
    /// A function call expression.
    Callee(LinkedNode<'a>),
    /// An import path expression.
    ImportPath(LinkedNode<'a>),
    /// An include path expression.
    IncludePath(LinkedNode<'a>),
    /// Any code expression.
    Code(LinkedNode<'a>),
    /// A label expression.
    Label(LinkedNode<'a>),
    /// A reference expression.
    Ref(LinkedNode<'a>),
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;

    use ecow::EcoString;
    use typst::foundations::Value;
    use typst::syntax::{LinkedNode, Side};

    use super::named_items;
    use crate::tests::{FilePos, TestWorld, WorldLike};

    type Response = Vec<(EcoString, Option<Value>)>;

    trait ResponseExt {
        fn must_include<'a>(&self, includes: impl IntoIterator<Item = &'a str>) -> &Self;
        fn must_exclude<'a>(&self, excludes: impl IntoIterator<Item = &'a str>) -> &Self;
        fn must_include_value(&self, name_value: (&str, Option<&Value>)) -> &Self;
    }

    impl ResponseExt for Response {
        #[track_caller]
        fn must_include<'a>(&self, includes: impl IntoIterator<Item = &'a str>) -> &Self {
            for item in includes {
                assert!(
                    self.iter().any(|v| v.0 == item),
                    "{item:?} was not contained in {self:?}",
                );
            }
            self
        }

        #[track_caller]
        fn must_exclude<'a>(&self, excludes: impl IntoIterator<Item = &'a str>) -> &Self {
            for item in excludes {
                assert!(
                    !self.iter().any(|v| v.0 == item),
                    "{item:?} was wrongly contained in {self:?}",
                );
            }
            self
        }

        #[track_caller]
        fn must_include_value(&self, name_value: (&str, Option<&Value>)) -> &Self {
            assert!(
                self.iter().any(|v| (v.0.as_str(), v.1.as_ref()) == name_value),
                "{name_value:?} was not contained in {self:?}",
            );
            self
        }
    }

    #[track_caller]
    fn test(world: impl WorldLike, pos: impl FilePos) -> Response {
        let world = world.acquire();
        let world = world.borrow();
        let (source, cursor) = pos.resolve(world);
        let node = LinkedNode::new(source.root());
        let leaf = node.leaf_at(cursor, Side::After).unwrap();
        let mut items = vec![];
        named_items(world, leaf, |s| {
            items.push((s.name().clone(), s.value().clone()));
            None::<()>
        });
        items
    }

    #[test]
    fn test_named_items_simple() {
        let s = "#let a = 1;#let b = 2;";
        test(s, 8).must_include(["a"]).must_exclude(["b"]);
        test(s, 15).must_include(["b"]);
    }

    #[test]
    fn test_named_items_param() {
        let pos = "#let f(a) = 1;#let b = 2;";
        test(pos, 12).must_include(["a"]);
        test(pos, 19).must_include(["b", "f"]).must_exclude(["a"]);

        let named = "#let f(a: b) = 1;#let b = 2;";
        test(named, 15).must_include(["a", "f"]).must_exclude(["b"]);
    }

    #[test]
    fn test_named_items_import() {
        test("#import \"foo.typ\"", 2).must_include(["foo"]);
        test("#import \"foo.typ\" as bar", 2)
            .must_include(["bar"])
            .must_exclude(["foo"]);
    }

    #[test]
    fn test_named_items_import_items() {
        test("#import \"foo.typ\": a; #(a);", 2)
            .must_include(["a"])
            .must_exclude(["foo"]);

        let world = TestWorld::new("#import \"foo.typ\": a.b; #(b);")
            .with_source("foo.typ", "#import \"a.typ\"")
            .with_source("a.typ", "#let b = 1;");
        test(&world, 2).must_include_value(("b", Some(&Value::Int(1))));
    }
}
