use comemo::Track;
use ecow::{EcoString, EcoVec, eco_vec};
use rustc_hash::FxHashSet;
use typst::Document;
use typst::foundations::{Label, Styles, Value};
use typst::layout::PagedDocument;
use typst::model::{BibliographyElem, FigureElem};
use typst::syntax::{LinkedNode, SyntaxKind, ast};

use crate::IdeWorld;

/// Try to determine a set of possible values for an expression.
pub fn analyze_expr(
    world: &dyn IdeWorld,
    node: &LinkedNode,
) -> EcoVec<(Value, Option<Styles>)> {
    let Some(expr) = node.cast::<ast::Expr>() else {
        return eco_vec![];
    };

    let val = match expr {
        ast::Expr::None(_) => Value::None,
        ast::Expr::Auto(_) => Value::Auto,
        ast::Expr::Bool(v) => Value::Bool(v.get()),
        ast::Expr::Int(v) => Value::Int(v.get()),
        ast::Expr::Float(v) => Value::Float(v.get()),
        ast::Expr::Numeric(v) => Value::numeric(v.get()),
        ast::Expr::Str(v) => Value::Str(v.get().into()),
        _ => {
            if node.kind() == SyntaxKind::Contextual
                && let Some(child) = node.children().next_back()
            {
                return analyze_expr(world, &child);
            }

            if let Some(parent) = node.parent()
                && parent.kind() == SyntaxKind::FieldAccess
                && node.index() > 0
            {
                return analyze_expr(world, parent);
            }

            return typst::trace::<PagedDocument>(world.upcast(), node.span());
        }
    };

    eco_vec![(val, None)]
}

/// Tries to load a module from the given `source` node.
pub fn analyze_import(world: &dyn IdeWorld, source: &LinkedNode) -> Option<Value> {
    // Use span in the node for resolving imports with relative paths.
    let source_span = source.span();
    let (source, _) = analyze_expr(world, source).into_iter().next()?;
    if source.scope().is_some() {
        return Some(source);
    }

    let Value::Str(path) = source else { return None };

    crate::utils::with_engine(world, |engine| {
        typst_eval::import(engine, &path, source_span).ok().map(Value::Module)
    })
}

/// Find all labels and details for them.
///
/// Returns:
/// - All labels and descriptions for them, if available
/// - A split offset: All labels before this offset belong to nodes, all after
///   belong to a bibliography.
///
/// Note: When multiple labels in the document have the same identifier,
/// this only returns the first one.
pub fn analyze_labels<D: Document + ?Sized>(
    document: &D,
) -> (Vec<(Label, Option<EcoString>)>, usize) {
    let mut output = vec![];
    let mut seen_labels = FxHashSet::default();

    // Labels in the document.
    for elem in document.introspector().all() {
        let Some(label) = elem.label() else { continue };
        if !seen_labels.insert(label) {
            continue;
        }

        let details = elem
            .to_packed::<FigureElem>()
            .and_then(|figure| match figure.caption.as_option() {
                Some(Some(caption)) => Some(caption.pack_ref()),
                _ => None,
            })
            .unwrap_or(elem)
            .get_by_name("body")
            .ok()
            .and_then(|field| match field {
                Value::Content(content) => Some(content),
                _ => None,
            })
            .as_ref()
            .unwrap_or(elem)
            .plain_text();
        output.push((label, Some(details)));
    }

    let split = output.len();

    // Bibliography keys.
    output.extend(BibliographyElem::keys(document.introspector().track()));

    (output, split)
}
