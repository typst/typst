use comemo::Track;
use ecow::{eco_vec, EcoString, EcoVec};
use typst::doc::Frame;
use typst::eval::{Route, Scopes, Tracer, Value, Vm};
use typst::model::{DelayedErrors, Introspector, Label, Locator, Vt};
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
    let id = source.span().id()?;
    let source = analyze_expr(world, source).into_iter().next()?;
    if source.scope().is_some() {
        return Some(source);
    }

    let mut locator = Locator::default();
    let introspector = Introspector::default();
    let mut delayed = DelayedErrors::new();
    let mut tracer = Tracer::new();
    let vt = Vt {
        world: world.track(),
        introspector: introspector.track(),
        locator: &mut locator,
        delayed: delayed.track_mut(),
        tracer: tracer.track_mut(),
    };

    let route = Route::default();
    let mut vm = Vm::new(vt, route.track(), Some(id), Scopes::new(Some(world.library())));
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
pub fn analyze_labels(
    world: &dyn World,
    frames: &[Frame],
) -> (Vec<(Label, Option<EcoString>)>, usize) {
    let mut output = vec![];
    let introspector = Introspector::new(frames);
    let items = &world.library().items;

    // Labels in the document.
    for elem in introspector.all() {
        let Some(label) = elem.label().cloned() else { continue };
        let details = elem
            .field("caption")
            .or_else(|| elem.field("body"))
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
    for (key, detail) in (items.bibliography_keys)(introspector.track()) {
        output.push((Label(key), detail));
    }

    (output, split)
}
