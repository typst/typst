use if_chain::if_chain;
use unicode_segmentation::UnicodeSegmentation;

use super::{analyze, plain_docs_sentence, summarize_font_family};
use crate::model::{CastInfo, Tracer, Value};
use crate::syntax::ast;
use crate::syntax::{LinkedNode, Source, SyntaxKind};
use crate::World;

/// Describe the item under the cursor.
pub fn tooltip(
    world: &(dyn World + 'static),
    source: &Source,
    cursor: usize,
) -> Option<String> {
    let leaf = LinkedNode::new(source.root()).leaf_at(cursor)?;

    named_param_tooltip(world, &leaf)
        .or_else(|| font_family_tooltip(world, &leaf))
        .or_else(|| expr_tooltip(world, &leaf))
}

/// Tooltip for a hovered expression.
fn expr_tooltip(world: &(dyn World + 'static), leaf: &LinkedNode) -> Option<String> {
    if !leaf.is::<ast::Expr>() {
        return None;
    }

    let values = analyze(world, leaf);
    if let [value] = values.as_slice() {
        if let Some(docs) = value.docs() {
            return Some(plain_docs_sentence(docs));
        }
    }

    let mut tooltip = String::new();
    let mut iter = values.into_iter().enumerate();
    for (i, value) in (&mut iter).take(Tracer::MAX - 1) {
        if i > 0 && !tooltip.is_empty() {
            tooltip.push_str(", ");
        }
        let repr = value.repr();
        let repr = repr.as_str();
        let len = repr.len();
        if len <= 40 {
            tooltip.push_str(repr);
        } else {
            let mut graphemes = repr.graphemes(true);
            let r = graphemes.next_back().map_or(0, str::len);
            let l = graphemes.take(40).map(str::len).sum();
            tooltip.push_str(&repr[..l]);
            tooltip.push_str("...");
            tooltip.push_str(&repr[len - r..]);
        }
    }

    if iter.next().is_some() {
        tooltip.push_str(", ...");
    }

    (!tooltip.is_empty()).then(|| tooltip)
}

/// Tooltips for components of a named parameter.
fn named_param_tooltip(
    world: &(dyn World + 'static),
    leaf: &LinkedNode,
) -> Option<String> {
    let (info, named) = if_chain! {
        // Ensure that we are in a named pair in the arguments to a function
        // call or set rule.
        if let Some(parent) = leaf.parent();
        if let Some(named) = parent.cast::<ast::Named>();
        if let Some(grand) = parent.parent();
        if matches!(grand.kind(), SyntaxKind::Args);
        if let Some(grand_grand) = grand.parent();
        if let Some(expr) = grand_grand.cast::<ast::Expr>();
        if let Some(ast::Expr::Ident(callee)) = match expr {
            ast::Expr::FuncCall(call) => Some(call.callee()),
            ast::Expr::Set(set) => Some(set.target()),
            _ => None,
        };

        // Find metadata about the function.
        if let Some(Value::Func(func)) = world.library().global.scope().get(&callee);
        if let Some(info) = func.info();
        then { (info, named) }
        else { return None; }
    };

    // Hovering over the parameter name.
    if_chain! {
        if leaf.index() == 0;
        if let Some(ident) = leaf.cast::<ast::Ident>();
        if let Some(param) = info.param(&ident);
        then {
            return Some(plain_docs_sentence(param.docs));
        }
    }

    // Hovering over a string parameter value.
    if_chain! {
        if let Some(string) = leaf.cast::<ast::Str>();
        if let Some(param) = info.param(&named.name());
        if let Some(docs) = find_string_doc(&param.cast, &string.get());
        then {
            return Some(docs.into());
        }
    }

    None
}

/// Find documentation for a castable string.
fn find_string_doc(info: &CastInfo, string: &str) -> Option<&'static str> {
    match info {
        CastInfo::Value(Value::Str(s), docs) if s.as_str() == string => Some(docs),
        CastInfo::Union(options) => {
            options.iter().find_map(|option| find_string_doc(option, string))
        }
        _ => None,
    }
}

/// Tooltip for font family.
fn font_family_tooltip(
    world: &(dyn World + 'static),
    leaf: &LinkedNode,
) -> Option<String> {
    if_chain! {
        // Ensure that we are on top of a string.
        if let Some(string) = leaf.cast::<ast::Str>();
        let lower = string.get().to_lowercase();

        // Ensure that we are in the arguments to the text function.
        if let Some(parent) = leaf.parent();
        if matches!(parent.kind(), SyntaxKind::Args);
        if let Some(grand) = parent.parent();
        if let Some(expr) = grand.cast::<ast::Expr>();
        if let Some(ast::Expr::Ident(callee)) = match expr {
            ast::Expr::FuncCall(call) => Some(call.callee()),
            ast::Expr::Set(set) => Some(set.target()),
            _ => None,
        };

        // Find the font family.
        if callee.as_str() == "text";
        if let Some((_, iter)) = world
            .book()
            .families()
            .find(|&(family, _)| family.to_lowercase().as_str() == lower.as_str());

        then {
            let detail = summarize_font_family(iter);
            return Some(detail);
        }
    };

    None
}
