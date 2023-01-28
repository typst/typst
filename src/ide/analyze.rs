use comemo::Track;

use crate::model::{eval, Route, Tracer, Value};
use crate::syntax::{ast, LinkedNode, SyntaxKind};
use crate::World;

/// Try to determine a set of possible values for an expression.
pub fn analyze(world: &(dyn World + 'static), node: &LinkedNode) -> Vec<Value> {
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
            analyze(world, &child)
                .into_iter()
                .filter_map(|target| target.field(&access.field()).ok())
                .collect()
        }

        Some(_) => {
            if let Some(parent) = node.parent() {
                if parent.kind() == SyntaxKind::FieldAccess && node.index() > 0 {
                    return analyze(world, parent);
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
