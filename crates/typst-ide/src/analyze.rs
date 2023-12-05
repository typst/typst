use comemo::Track;
use ecow::{eco_vec, EcoString, EcoVec};
use typst::engine::{Engine, Route};
use typst::eval::{Tracer, Vm};
use typst::foundations::{Label, Scopes, Value};
use typst::introspection::{Introspector, Locator};
use typst::model::{BibliographyElem, Document};
use typst::syntax::{ast, LinkedNode, Span, SyntaxKind};
use typst::World;

/// Try to determine a set of possible values for an expression.
pub fn analyze_expr(world: &dyn World, node: &LinkedNode) -> EcoVec<Value> {
    match node.cast::<ast::Expr>() {
        Some(ast::Expr::None(_)) => eco_vec![Value::None],
        Some(ast::Expr::Auto(_)) => eco_vec![Value::Auto],
        Some(ast::Expr::Bool(v)) => eco_vec![Value::Bool(v.get())],
        Some(ast::Expr::Int(v)) => eco_vec![Value::Int(v.get())],
        Some(ast::Expr::Float(v)) => eco_vec![Value::Float(v.get())],
        Some(ast::Expr::Numeric(v)) => eco_vec![Value::numeric(v.get())],
        Some(ast::Expr::Str(v)) => eco_vec![Value::Str(v.get().into())],

        Some(ast::Expr::FieldAccess(access)) => {
            let Some(child) = node.children().next() else { return eco_vec![] };
            analyze_expr(world, &child)
                .into_iter()
                .filter_map(|target| target.field(&access.field()).ok())
                .collect()
        }

        Some(_) => {
            if let Some(parent) = node.parent() {
                if parent.kind() == SyntaxKind::FieldAccess && node.index() > 0 {
                    return analyze_expr(world, parent);
                }
            }

            let mut tracer = Tracer::new();
            tracer.inspect(node.span());
            typst::compile(world, &mut tracer).ok();
            tracer.values()
        }

        _ => eco_vec![],
    }
}

/// Try to load a module from the current source file.
pub fn analyze_import(world: &dyn World, source: &LinkedNode) -> Option<Value> {
    let source = analyze_expr(world, source).into_iter().next()?;
    if source.scope().is_some() {
        return Some(source);
    }

    let mut locator = Locator::default();
    let introspector = Introspector::default();
    let mut tracer = Tracer::new();
    let engine = Engine {
        world: world.track(),
        route: Route::default(),
        introspector: introspector.track(),
        locator: &mut locator,
        tracer: tracer.track_mut(),
    };

    let mut vm = Vm::new(engine, Scopes::new(Some(world.library())), Span::detached());
    typst::eval::import(&mut vm, source, Span::detached(), true)
        .ok()
        .map(Value::Module)
}

/// Find all labels and details for them.
///
/// Returns:
/// - All labels and descriptions for them, if available
/// - A split offset: All labels before this offset belong to nodes, all after
///   belong to a bibliography.
pub fn analyze_labels(document: &Document) -> (Vec<(Label, Option<EcoString>)>, usize) {
    let mut output = vec![];

    // Labels in the document.
    for elem in document.introspector.all() {
        let Some(label) = elem.label() else { continue };
        let details = elem
            .get_by_name("caption")
            .or_else(|| elem.get_by_name("body"))
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
    for (key, detail) in BibliographyElem::keys(document.introspector.track()) {
        output.push((Label::new(&key), detail));
    }

    (output, split)
}
