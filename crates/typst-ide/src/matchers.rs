use ecow::EcoString;
use typst::foundations::{Module, Value};
use typst::syntax::ast::AstNode;
use typst::syntax::{ast, LinkedNode, Span, SyntaxKind};
use typst::World;

use crate::analyze::analyze_import;

/// Find the named items starting from the given position.
pub fn named_items<T, F: FnMut(NamedItem) -> Option<T>>(
    world: &dyn World,
    position: LinkedNode,
    mut recv: F,
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
                match imports {
                    None | Some(ast::Imports::Wildcard) => {
                        if let Some(value) = node
                            .children()
                            .find(|child| child.is::<ast::Expr>())
                            .and_then(|source| analyze_import(world, &source))
                        {
                            if imports.is_none() {
                                if let Ok(value) = value.clone().cast::<Module>() {
                                    if let Some(res) = recv(NamedItem::Module(&value)) {
                                        return Some(res);
                                    }
                                }
                            } else if let Some(scope) = value.scope() {
                                for (name, value) in scope.iter() {
                                    let item = NamedItem::Import(
                                        name,
                                        Span::detached(),
                                        Some(value),
                                    );
                                    if let Some(res) = recv(item) {
                                        return Some(res);
                                    }
                                }
                            }
                        }
                    }
                    Some(ast::Imports::Items(items)) => {
                        for item in items.iter() {
                            let name = item.bound_name();
                            if let Some(res) =
                                recv(NamedItem::Import(name.get(), name.span(), None))
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

pub enum NamedItem<'a> {
    Var(ast::Ident<'a>),
    Fn(ast::Ident<'a>),
    Module(&'a Module),
    Import(&'a EcoString, Span, Option<&'a Value>),
}

impl<'a> NamedItem<'a> {
    pub(crate) fn name(&self) -> &'a EcoString {
        match self {
            NamedItem::Var(ident) => ident.get(),
            NamedItem::Fn(ident) => ident.get(),
            NamedItem::Module(value) => value.name(),
            NamedItem::Import(name, _, _) => name,
        }
    }

    pub(crate) fn value(&self) -> Option<Value> {
        match self {
            NamedItem::Var(..) | NamedItem::Fn(..) => None,
            NamedItem::Module(value) => Some(Value::Module((*value).clone())),
            NamedItem::Import(_, _, value) => value.cloned(),
        }
    }
}

/// Find an expression that can be dereferenced on the given node.
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
        ast::Expr::Label(..) => DerefTarget::Label(expr_node),
        ast::Expr::Ref(..) => DerefTarget::Ref(expr_node),
        ast::Expr::FuncCall(call) => {
            DerefTarget::Callee(expr_node.find(call.callee().span())?)
        }
        ast::Expr::Set(set) => DerefTarget::Callee(expr_node.find(set.target().span())?),
        ast::Expr::Ident(..) | ast::Expr::MathIdent(..) | ast::Expr::FieldAccess(..) => {
            DerefTarget::VarAccess(expr_node)
        }
        ast::Expr::Str(..) => {
            let parent = expr_node.parent()?;
            if parent.kind() == SyntaxKind::ModuleImport {
                DerefTarget::ImportPath(expr_node)
            } else if parent.kind() == SyntaxKind::ModuleInclude {
                DerefTarget::IncludePath(expr_node)
            } else {
                DerefTarget::Normal(expr_node.kind(), expr_node)
            }
        }
        _ if expr.hash()
            || matches!(expr_node.kind(), SyntaxKind::MathIdent | SyntaxKind::Error) =>
        {
            DerefTarget::Normal(expr_node.kind(), expr_node)
        }
        _ => return None,
    })
}

#[derive(Debug, Clone)]
pub enum DerefTarget<'a> {
    Label(LinkedNode<'a>),
    Ref(LinkedNode<'a>),
    VarAccess(LinkedNode<'a>),
    Callee(LinkedNode<'a>),
    ImportPath(LinkedNode<'a>),
    IncludePath(LinkedNode<'a>),
    Normal(SyntaxKind, LinkedNode<'a>),
}
