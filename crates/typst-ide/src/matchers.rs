use ecow::EcoString;
use typst::diag::MaybeDeprecated;
use typst::foundations::{Module, Value};
use typst::syntax::ast::AstNode;
use typst::syntax::{ast, LinkedNode, Span, SyntaxKind, SyntaxNode};

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
                let source = node
                    .children()
                    .find(|child| child.is::<ast::Expr>())
                    .and_then(|source: LinkedNode| {
                        Some((analyze_import(world, &source)?, source))
                    });
                let source = source.as_ref();

                // Seeing the module itself.
                if let Some((value, source)) = source {
                    let site = match (imports, v.new_name()) {
                        // ```plain
                        // import "foo" as name;
                        // import "foo" as name: ..;
                        // ```
                        (_, Some(name)) => Some(name.to_untyped()),
                        // ```plain
                        // import "foo";
                        // ```
                        (None, None) => Some(source.get()),
                        // ```plain
                        // import "foo": ..;
                        // ```
                        (Some(..), None) => None,
                    };

                    if let Some((site, value)) =
                        site.zip(value.clone().cast::<Module>().ok())
                    {
                        if let Some(res) = recv(NamedItem::Module(&value, site)) {
                            return Some(res);
                        }
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
                        if let Some(scope) = source.and_then(|(value, _)| value.scope()) {
                            for (name, value, span) in scope.iter() {
                                let item = NamedItem::Import(
                                    name,
                                    span,
                                    Some(value.into_inner()),
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
                            let original = item.original_name();
                            let bound = item.bound_name();
                            let scope = source.and_then(|(value, _)| value.scope());
                            let span = scope
                                .and_then(|s| s.get_span(&original))
                                .unwrap_or(Span::detached())
                                .or(bound.span());

                            let value = scope.and_then(|s| s.get(&original));
                            if let Some(res) = recv(NamedItem::Import(
                                bound.get(),
                                span,
                                value.map(MaybeDeprecated::into_inner),
                            )) {
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
    /// A (imported) module item.
    Module(&'a Module, &'a SyntaxNode),
    /// An imported item.
    Import(&'a EcoString, Span, Option<&'a Value>),
}

impl<'a> NamedItem<'a> {
    pub(crate) fn name(&self) -> &'a EcoString {
        match self {
            NamedItem::Var(ident) => ident.get(),
            NamedItem::Fn(ident) => ident.get(),
            NamedItem::Module(value, _) => value.name(),
            NamedItem::Import(name, _, _) => name,
        }
    }

    pub(crate) fn value(&self) -> Option<Value> {
        match self {
            NamedItem::Var(..) | NamedItem::Fn(..) => None,
            NamedItem::Module(value, _) => Some(Value::Module((*value).clone())),
            NamedItem::Import(_, _, value) => value.cloned(),
        }
    }

    pub(crate) fn span(&self) -> Span {
        match *self {
            NamedItem::Var(name) | NamedItem::Fn(name) => name.span(),
            NamedItem::Module(_, site) => site.span(),
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
        ast::Expr::Set(set) => DerefTarget::Callee(expr_node.find(set.target().span())?),
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
    use typst::syntax::{LinkedNode, Side};

    use super::named_items;
    use crate::tests::{FilePos, WorldLike};

    type Response = Vec<EcoString>;

    trait ResponseExt {
        fn must_include<'a>(&self, includes: impl IntoIterator<Item = &'a str>) -> &Self;
        fn must_exclude<'a>(&self, excludes: impl IntoIterator<Item = &'a str>) -> &Self;
    }

    impl ResponseExt for Response {
        #[track_caller]
        fn must_include<'a>(&self, includes: impl IntoIterator<Item = &'a str>) -> &Self {
            for item in includes {
                assert!(
                    self.iter().any(|v| v == item),
                    "{item:?} was not contained in {self:?}",
                );
            }
            self
        }

        #[track_caller]
        fn must_exclude<'a>(&self, excludes: impl IntoIterator<Item = &'a str>) -> &Self {
            for item in excludes {
                assert!(
                    !self.iter().any(|v| v == item),
                    "{item:?} was wrongly contained in {self:?}",
                );
            }
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
            items.push(s.name().clone());
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
        test("#import \"foo.typ\": a; #(a);", 2).must_include(["a"]);
    }
}
