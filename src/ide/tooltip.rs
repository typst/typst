use std::fmt::Write;

use if_chain::if_chain;

use crate::font::{FontInfo, FontStyle};
use crate::model::{CastInfo, Value};
use crate::syntax::ast::{self, AstNode};
use crate::syntax::{LinkedNode, Source, SyntaxKind};
use crate::World;

/// Describe the item under the cursor.
pub fn tooltip(world: &dyn World, source: &Source, cursor: usize) -> Option<String> {
    let leaf = LinkedNode::new(source.root()).leaf_at(cursor)?;

    function_tooltip(world, &leaf)
        .or_else(|| named_param_tooltip(world, &leaf))
        .or_else(|| font_family_tooltip(world, &leaf))
}

/// Tooltip for a function or set rule name.
fn function_tooltip(world: &dyn World, leaf: &LinkedNode) -> Option<String> {
    if_chain! {
        if let SyntaxKind::Ident(ident) = leaf.kind();
        if matches!(
            leaf.parent_kind(),
            Some(SyntaxKind::FuncCall | SyntaxKind::SetRule),
        );
        if let Some(Value::Func(func)) = world.library().scope.get(ident);
        if let Some(info) = func.info();
        then {
            return Some(info.docs.into());
        }
    }

    None
}

/// Tooltips for components of a named parameter.
fn named_param_tooltip(world: &dyn World, leaf: &LinkedNode) -> Option<String> {
    let (info, named) = if_chain! {
        // Ensure that we are in a named pair in the arguments to a function
        // call or set rule.
        if let Some(parent) = leaf.parent();
        if let Some(named) = parent.cast::<ast::Named>();
        if let Some(grand) = parent.parent();
        if matches!(grand.kind(), SyntaxKind::Args);
        if let Some(grand_grand) = grand.parent();
        if let Some(expr) = grand_grand.cast::<ast::Expr>();
        if let Some(callee) = match expr {
            ast::Expr::FuncCall(call) => call.callee().as_untyped().cast(),
            ast::Expr::Set(set) => Some(set.target()),
            _ => None,
        };

        // Find metadata about the function.
        if let Some(Value::Func(func)) = world.library().scope.get(&callee);
        if let Some(info) = func.info();
        then { (info, named) }
        else { return None; }
    };

    // Hovering over the parameter name.
    if_chain! {
        if leaf.index() == 0;
        if let SyntaxKind::Ident(ident) = leaf.kind();
        if let Some(param) = info.param(ident);
        then {
            return Some(param.docs.into());
        }
    }

    // Hovering over a string parameter value.
    if_chain! {
        if let SyntaxKind::Str(string) = leaf.kind();
        if let Some(param) = info.param(&named.name());
        if let Some(docs) = find_string_doc(&param.cast, string);
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
fn font_family_tooltip(world: &dyn World, leaf: &LinkedNode) -> Option<String> {
    if_chain! {
        // Ensure that we are on top of a string.
        if let SyntaxKind::Str(string) = leaf.kind();
        let lower = string.to_lowercase();

        // Ensure that we are in the arguments to the text function.
        if let Some(parent) = leaf.parent();
        if matches!(parent.kind(), SyntaxKind::Args);
        if let Some(grand) = parent.parent();
        if let Some(expr) = grand.cast::<ast::Expr>();
        if let Some(callee) = match expr {
            ast::Expr::FuncCall(call) => call.callee().as_untyped().cast(),
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

/// Create a short description of a font family.
pub(super) fn summarize_font_family<'a>(
    variants: impl Iterator<Item = &'a FontInfo>,
) -> String {
    let mut infos: Vec<_> = variants.collect();
    infos.sort_by_key(|info| info.variant);

    let mut has_italic = false;
    let mut min_weight = u16::MAX;
    let mut max_weight = 0;
    for info in &infos {
        let weight = info.variant.weight.to_number();
        has_italic |= info.variant.style == FontStyle::Italic;
        min_weight = min_weight.min(weight);
        max_weight = min_weight.max(weight);
    }

    let count = infos.len();
    let s = if count == 1 { "" } else { "s" };
    let mut detail = format!("{count} variant{s}.");

    if min_weight == max_weight {
        write!(detail, " Weight {min_weight}.").unwrap();
    } else {
        write!(detail, " Weights {min_weight}–{max_weight}.").unwrap();
    }

    if has_italic {
        detail.push_str(" Has italics.");
    }

    detail
}
