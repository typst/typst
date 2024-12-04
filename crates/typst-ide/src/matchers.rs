use ecow::EcoString;
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
                                let item = NamedItem::Import(name, span, Some(value));
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
                            let bound = item.bound_name();

                            let (span, value) = item.path().iter().fold(
                                (bound.span(), source.map(|(value, _)| value)),
                                |(span, value), path_ident| {
                                    let scope = value.and_then(|v| v.scope());
                                    let span = scope
                                        .and_then(|s| s.get_span(&path_ident))
                                        .unwrap_or(Span::detached())
                                        .or(span);
                                    let value = scope.and_then(|s| s.get(&path_ident));
                                    (span, value)
                                },
                            );

                            if let Some(res) =
                                recv(NamedItem::Import(bound.get(), span, value))
                            {
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
    use typst::syntax::{LinkedNode, Side};

    use crate::{named_items, tests::TestWorld};

    #[track_caller]
    fn has_named_items(text: &str, cursor: usize, containing: &str) -> bool {
        let world = TestWorld::new(text);

        let src = world.main.clone();
        let node = LinkedNode::new(src.root());
        let leaf = node.leaf_at(cursor, Side::After).unwrap();

        let res = named_items(&world, leaf, |s| {
            if containing == s.name() {
                return Some(true);
            }

            None
        });

        res.unwrap_or_default()
    }

    #[test]
    fn test_simple_named_items() {
        // Has named items
        assert!(has_named_items(r#"#let a = 1;#let b = 2;"#, 8, "a"));
        assert!(has_named_items(r#"#let a = 1;#let b = 2;"#, 15, "a"));

        // Doesn't have named items
        assert!(!has_named_items(r#"#let a = 1;#let b = 2;"#, 8, "b"));
    }

    #[test]
    fn test_import_named_items() {
        // Cannot test much.
        assert!(has_named_items(r#"#import "foo.typ": a; #(a);"#, 24, "a"));
    }
}
