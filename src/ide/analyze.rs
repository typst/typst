use std::path::PathBuf;

use comemo::Track;
use ecow::EcoString;

use crate::doc::Frame;
use crate::eval::{eval, Module, Route, Tracer, Value};
use crate::model::{Introspector, Label};
use crate::syntax::{ast, LinkedNode, Source, SyntaxKind};
use crate::util::PathExt;
use crate::World;

/// Try to determine a set of possible values for an expression.
pub fn analyze_expr(world: &(dyn World + 'static), node: &LinkedNode) -> Vec<Value> {
    match node.cast::<ast::Expr>() {
        Some(ast::Expr::None(_)) => vec![Value::None],
        Some(ast::Expr::Auto(_)) => vec![Value::Auto],
        Some(ast::Expr::Bool(v)) => vec![Value::Bool(v.get())],
        Some(ast::Expr::Int(v)) => vec![Value::Int(v.get())],
        Some(ast::Expr::Float(v)) => vec![Value::Float(v.get())],
        Some(ast::Expr::Numeric(v)) => vec![Value::numeric(v.get())],
        Some(ast::Expr::Str(v)) => vec![Value::Str(v.get().into())],

        Some(ast::Expr::FieldAccess(access)) => {
            let Some(child) = node.children().next() else { return vec![] };
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

            let span = node.span();
            let source = world.source(span.source());
            let route = Route::default();
            let mut tracer = Tracer::new(Some(span));
            eval(world.track(), route.track(), tracer.track_mut(), source).ok();
            tracer.finish()
        }

        _ => vec![],
    }
}

/// Try to load a module from the current source file.
pub fn analyze_import(
    world: &(dyn World + 'static),
    source: &Source,
    path: &str,
) -> Option<Module> {
    let full: PathBuf = if let Some(path) = path.strip_prefix('/') {
        world.root().join(path).normalize()
    } else if let Some(dir) = source.path().parent() {
        dir.join(path).normalize()
    } else {
        path.into()
    };
    let route = Route::default();
    let mut tracer = Tracer::default();
    let id = world.resolve(&full).ok()?;
    let source = world.source(id);
    eval(world.track(), route.track(), tracer.track_mut(), source).ok()
}

/// Find all labels and details for them.
pub fn analyze_labels(
    world: &(dyn World + 'static),
    frames: &[Frame],
) -> (Vec<(Label, Option<EcoString>)>, usize) {
    let mut output = vec![];
    let mut introspector = Introspector::new();
    let items = &world.library().items;
    introspector.update(frames);

    // Labels in the document.
    for node in introspector.iter() {
        let Some(label) = node.label() else { continue };
        let details = node
            .field("caption")
            .or_else(|| node.field("body"))
            .and_then(|field| match field {
                Value::Content(content) => Some(content),
                _ => None,
            })
            .and_then(|content| (items.text_str)(content));
        output.push((label.clone(), details));
    }

    let split = output.len();

    // Bibliography keys.
    for (key, detail) in (items.bibliography_keys)(world.track(), introspector.track()) {
        output.push((Label(key), detail));
    }

    (output, split)
}
