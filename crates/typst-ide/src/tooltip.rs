use std::fmt::Write;

use ecow::{eco_format, EcoString};
use if_chain::if_chain;
use typst::engine::Sink;
use typst::foundations::{repr, Capturer, CastInfo, Repr, Value};
use typst::layout::{Length, PagedDocument};
use typst::syntax::ast::AstNode;
use typst::syntax::{ast, LinkedNode, Side, Source, SyntaxKind};
use typst::utils::{round_with_precision, Numeric};
use typst_eval::CapturesVisitor;

use crate::utils::{plain_docs_sentence, summarize_font_family};
use crate::{analyze_expr, analyze_import, analyze_labels, IdeWorld};

/// Describe the item under the cursor.
///
/// Passing a `document` (from a previous compilation) is optional, but enhances
/// the tooltips. Label tooltips, for instance, are only generated when the
/// document is available.
pub fn tooltip(
    world: &dyn IdeWorld,
    document: Option<&PagedDocument>,
    source: &Source,
    cursor: usize,
    side: Side,
) -> Option<Tooltip> {
    let leaf = LinkedNode::new(source.root()).leaf_at(cursor, side)?;
    if leaf.kind().is_trivia() {
        return None;
    }

    named_param_tooltip(world, &leaf)
        .or_else(|| font_tooltip(world, &leaf))
        .or_else(|| document.and_then(|doc| label_tooltip(doc, &leaf)))
        .or_else(|| import_tooltip(world, &leaf))
        .or_else(|| expr_tooltip(world, &leaf))
        .or_else(|| closure_tooltip(&leaf))
}

/// A hover tooltip.
#[derive(Debug, Clone, PartialEq)]
pub enum Tooltip {
    /// A string of text.
    Text(EcoString),
    /// A string of Typst code.
    Code(EcoString),
}

/// Tooltip for a hovered expression.
fn expr_tooltip(world: &dyn IdeWorld, leaf: &LinkedNode) -> Option<Tooltip> {
    let mut ancestor = leaf;
    while !ancestor.is::<ast::Expr>() {
        ancestor = ancestor.parent()?;
    }

    let expr = ancestor.cast::<ast::Expr>()?;
    if !expr.hash() && !matches!(expr, ast::Expr::MathIdent(_)) {
        return None;
    }

    let values = analyze_expr(world, ancestor);

    if let [(value, _)] = values.as_slice() {
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
    for (value, _) in (&mut iter).take(Sink::MAX_VALUES - 1) {
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

    let tooltip = repr::pretty_comma_list(&pieces, false);
    (!tooltip.is_empty()).then(|| Tooltip::Code(tooltip.into()))
}

/// Tooltips for imports.
fn import_tooltip(world: &dyn IdeWorld, leaf: &LinkedNode) -> Option<Tooltip> {
    if_chain! {
        if leaf.kind() == SyntaxKind::Star;
        if let Some(parent) = leaf.parent();
        if let Some(import) = parent.cast::<ast::ModuleImport>();
        if let Some(node) = parent.find(import.source().span());
        if let Some(value) = analyze_import(world, &node);
        if let Some(scope) = value.scope();
        then {
            let names: Vec<_> =
                scope.iter().map(|(name, ..)| eco_format!("`{name}`")).collect();
            let list = repr::separated_list(&names, "and");
            return Some(Tooltip::Text(eco_format!("This star imports {list}")));
        }
    }

    None
}

/// Tooltip for a hovered closure.
fn closure_tooltip(leaf: &LinkedNode) -> Option<Tooltip> {
    // Only show this tooltip when hovering over the equals sign or arrow of
    // the closure. Showing it across the whole subtree is too noisy.
    if !matches!(leaf.kind(), SyntaxKind::Eq | SyntaxKind::Arrow) {
        return None;
    }

    // Find the closure to analyze.
    let parent = leaf.parent()?;
    if parent.kind() != SyntaxKind::Closure {
        return None;
    }

    // Analyze the closure's captures.
    let mut visitor = CapturesVisitor::new(None, Capturer::Function);
    visitor.visit(parent);

    let captures = visitor.finish();
    let mut names: Vec<_> =
        captures.iter().map(|(name, ..)| eco_format!("`{name}`")).collect();
    if names.is_empty() {
        return None;
    }

    names.sort();

    let tooltip = repr::separated_list(&names, "and");
    Some(Tooltip::Text(eco_format!("This closure captures {tooltip}")))
}

/// Tooltip text for a hovered length.
fn length_tooltip(length: Length) -> Option<Tooltip> {
    length.em.is_zero().then(|| {
        Tooltip::Code(eco_format!(
            "{}pt = {}mm = {}cm = {}in",
            round_with_precision(length.abs.to_pt(), 2),
            round_with_precision(length.abs.to_mm(), 2),
            round_with_precision(length.abs.to_cm(), 2),
            round_with_precision(length.abs.to_inches(), 2),
        ))
    })
}

/// Tooltip for a hovered reference or label.
fn label_tooltip(document: &PagedDocument, leaf: &LinkedNode) -> Option<Tooltip> {
    let target = match leaf.kind() {
        SyntaxKind::RefMarker => leaf.text().trim_start_matches('@'),
        SyntaxKind::Label => leaf.text().trim_start_matches('<').trim_end_matches('>'),
        _ => return None,
    };

    for (label, detail) in analyze_labels(document).0 {
        if label.resolve().as_str() == target {
            return Some(Tooltip::Text(detail?));
        }
    }

    None
}

/// Tooltips for components of a named parameter.
fn named_param_tooltip(world: &dyn IdeWorld, leaf: &LinkedNode) -> Option<Tooltip> {
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
fn font_tooltip(world: &dyn IdeWorld, leaf: &LinkedNode) -> Option<Tooltip> {
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

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;

    use typst::syntax::Side;

    use super::{tooltip, Tooltip};
    use crate::tests::{FilePos, TestWorld, WorldLike};

    type Response = Option<Tooltip>;

    trait ResponseExt {
        fn must_be_none(&self) -> &Self;
        fn must_be_text(&self, text: &str) -> &Self;
        fn must_be_code(&self, code: &str) -> &Self;
    }

    impl ResponseExt for Response {
        #[track_caller]
        fn must_be_none(&self) -> &Self {
            assert_eq!(*self, None);
            self
        }

        #[track_caller]
        fn must_be_text(&self, text: &str) -> &Self {
            assert_eq!(*self, Some(Tooltip::Text(text.into())));
            self
        }

        #[track_caller]
        fn must_be_code(&self, code: &str) -> &Self {
            assert_eq!(*self, Some(Tooltip::Code(code.into())));
            self
        }
    }

    #[track_caller]
    fn test(world: impl WorldLike, pos: impl FilePos, side: Side) -> Response {
        let world = world.acquire();
        let world = world.borrow();
        let (source, cursor) = pos.resolve(world);
        let doc = typst::compile(world).output.ok();
        tooltip(world, doc.as_ref(), &source, cursor, side)
    }

    #[test]
    fn test_tooltip() {
        test("#let x = 1 + 2", -1, Side::After).must_be_none();
        test("#let x = 1 + 2", 5, Side::After).must_be_code("3");
        test("#let x = 1 + 2", 6, Side::Before).must_be_code("3");
        test("#let x = 1 + 2", 6, Side::Before).must_be_code("3");
    }

    #[test]
    fn test_tooltip_empty_contextual() {
        test("#{context}", -1, Side::Before).must_be_code("context()");
    }

    #[test]
    fn test_tooltip_closure() {
        test("#let f(x) = x + y", 11, Side::Before)
            .must_be_text("This closure captures `y`");
        // Same tooltip if `y` is defined first.
        test("#let y = 10; #let f(x) = x + y", 24, Side::Before)
            .must_be_text("This closure captures `y`");
        // Names are sorted.
        test("#let f(x) = x + y + z + a", 11, Side::Before)
            .must_be_text("This closure captures `a`, `y`, and `z`");
        // Names are de-duplicated.
        test("#let f(x) = x + y + z + y", 11, Side::Before)
            .must_be_text("This closure captures `y` and `z`");
        // With arrow syntax.
        test("#let f = (x) => x + y", 15, Side::Before)
            .must_be_text("This closure captures `y`");
        // No recursion with arrow syntax.
        test("#let f = (x) => x + y + f", 13, Side::After)
            .must_be_text("This closure captures `f` and `y`");
    }

    #[test]
    fn test_tooltip_star_import() {
        let world = TestWorld::new("#import \"other.typ\": *")
            .with_source("other.typ", "#let (a, b, c) = (1, 2, 3)");
        test(&world, -2, Side::Before).must_be_none();
        test(&world, -2, Side::After).must_be_text("This star imports `a`, `b`, and `c`");
    }
}
