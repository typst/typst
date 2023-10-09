use std::fmt::Write;

use ecow::{eco_format, EcoString};
use if_chain::if_chain;
use typst::doc::Frame;
use typst::eval::{CapturesVisitor, CastInfo, Repr, Tracer, Value};
use typst::geom::{round_2, Length, Numeric};
use typst::syntax::ast::{self, AstNode};
use typst::syntax::{LinkedNode, Source, SyntaxKind};
use typst::util::{pretty_comma_list, separated_list};
use typst::World;

use super::analyze::analyze_labels;
use super::{analyze_expr, plain_docs_sentence, summarize_font_family};

/// Describe the item under the cursor.
pub fn tooltip(
    world: &dyn World,
    frames: &[Frame],
    source: &Source,
    cursor: usize,
) -> Option<Tooltip> {
    let leaf = LinkedNode::new(source.root()).leaf_at(cursor)?;
    if leaf.kind().is_trivia() {
        return None;
    }

    named_param_tooltip(world, &leaf)
        .or_else(|| font_tooltip(world, &leaf))
        .or_else(|| ref_tooltip(world, frames, &leaf))
        .or_else(|| expr_tooltip(world, &leaf))
        .or_else(|| closure_tooltip(&leaf))
}

/// A hover tooltip.
#[derive(Debug, Clone)]
pub enum Tooltip {
    /// A string of text.
    Text(EcoString),
    /// A string of Typst code.
    Code(EcoString),
}

/// Tooltip for a hovered expression.
fn expr_tooltip(world: &dyn World, leaf: &LinkedNode) -> Option<Tooltip> {
    let mut ancestor = leaf;
    while !ancestor.is::<ast::Expr>() {
        ancestor = ancestor.parent()?;
    }

    let expr = ancestor.cast::<ast::Expr>()?;
    if !expr.hash() && !matches!(expr, ast::Expr::MathIdent(_)) {
        return None;
    }

    let values = analyze_expr(world, ancestor);

    if let [value] = values.as_slice() {
        if let Some(docs) = value.docs() {
            return Some(Tooltip::Text(plain_docs_sentence(docs)));
        }

        if let &Value::Length(length) = value {
            if let Some(tooltip) = length_tooltip(length) {
                return Some(tooltip);
            }
        }
    }

    if expr.is_literal() {
        return None;
    }

    let mut last = None;
    let mut pieces: Vec<EcoString> = vec![];
    let mut iter = values.iter();
    for value in (&mut iter).take(Tracer::MAX_VALUES - 1) {
        if let Some((prev, count)) = &mut last {
            if *prev == value {
                *count += 1;
                continue;
            } else if *count > 1 {
                write!(pieces.last_mut().unwrap(), " (x{count})").unwrap();
            }
        }
        pieces.push(value.repr());
        last = Some((value, 1));
    }

    if let Some((_, count)) = last {
        if count > 1 {
            write!(pieces.last_mut().unwrap(), " (x{count})").unwrap();
        }
    }

    if iter.next().is_some() {
        pieces.push("...".into());
    }

    let tooltip = pretty_comma_list(&pieces, false);
    (!tooltip.is_empty()).then(|| Tooltip::Code(tooltip.into()))
}

/// Tooltip for a hovered closure.
fn closure_tooltip(leaf: &LinkedNode) -> Option<Tooltip> {
    // Find the closure to analyze.
    let mut ancestor = leaf;
    while !ancestor.is::<ast::Closure>() {
        ancestor = ancestor.parent()?;
    }
    let closure = ancestor.cast::<ast::Closure>()?.to_untyped();

    // Analyze the closure's captures.
    let mut visitor = CapturesVisitor::new(None);
    visitor.visit(closure);

    let captures = visitor.finish();
    let mut names: Vec<_> =
        captures.iter().map(|(name, _)| eco_format!("`{name}`")).collect();
    if names.is_empty() {
        return None;
    }

    names.sort();

    let tooltip = separated_list(&names, "and");
    Some(Tooltip::Text(eco_format!("This closure captures {tooltip}.")))
}

/// Tooltip text for a hovered length.
fn length_tooltip(length: Length) -> Option<Tooltip> {
    length.em.is_zero().then(|| {
        Tooltip::Code(eco_format!(
            "{}pt = {}mm = {}cm = {}in",
            round_2(length.abs.to_pt()),
            round_2(length.abs.to_mm()),
            round_2(length.abs.to_cm()),
            round_2(length.abs.to_inches())
        ))
    })
}

/// Tooltip for a hovered reference.
fn ref_tooltip(
    world: &dyn World,
    frames: &[Frame],
    leaf: &LinkedNode,
) -> Option<Tooltip> {
    if leaf.kind() != SyntaxKind::RefMarker {
        return None;
    }

    let target = leaf.text().trim_start_matches('@');
    for (label, detail) in analyze_labels(world, frames).0 {
        if label.0 == target {
            return Some(Tooltip::Text(detail?));
        }
    }

    None
}

/// Tooltips for components of a named parameter.
fn named_param_tooltip(world: &dyn World, leaf: &LinkedNode) -> Option<Tooltip> {
    let (func, named) = if_chain! {
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
        then { (func, named) }
        else { return None; }
    };

    // Hovering over the parameter name.
    if_chain! {
        if leaf.index() == 0;
        if let Some(ident) = leaf.cast::<ast::Ident>();
        if let Some(param) = func.param(&ident);
        then {
            return Some(Tooltip::Text(plain_docs_sentence(param.docs)));
        }
    }

    // Hovering over a string parameter value.
    if_chain! {
        if let Some(string) = leaf.cast::<ast::Str>();
        if let Some(param) = func.param(&named.name());
        if let Some(docs) = find_string_doc(&param.input, &string.get());
        then {
            return Some(Tooltip::Text(docs.into()));
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

/// Tooltip for font.
fn font_tooltip(world: &dyn World, leaf: &LinkedNode) -> Option<Tooltip> {
    if_chain! {
        // Ensure that we are on top of a string.
        if let Some(string) = leaf.cast::<ast::Str>();
        let lower = string.get().to_lowercase();

        // Ensure that we are in the arguments to the text function.
        if let Some(parent) = leaf.parent();
        if let Some(named) = parent.cast::<ast::Named>();
        if named.name().as_str() == "font";

        // Find the font family.
        if let Some((_, iter)) = world
            .book()
            .families()
            .find(|&(family, _)| family.to_lowercase().as_str() == lower.as_str());

        then {
            let detail = summarize_font_family(iter);
            return Some(Tooltip::Text(detail));
        }
    };

    None
}
