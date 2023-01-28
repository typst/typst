use comemo::Track;

use crate::model::{eval, Route, Tracer, Value};
use crate::syntax::{ast, LinkedNode, SyntaxKind};
use crate::World;

/// Try to determine a set of possible values for an expression.
pub fn analyze(world: &(dyn World + 'static), node: &LinkedNode) -> Vec<Value> {
    match node.cast::<ast::Expr>() {
        Some(ast::Expr::Ident(_) | ast::Expr::MathIdent(_) | ast::Expr::FuncCall(_)) => {
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
            return tracer.finish();
        }

        Some(ast::Expr::Str(s)) => return vec![Value::Str(s.get().into())],

        Some(ast::Expr::FieldAccess(access)) => {
            if let Some(child) = node.children().next() {
                return analyze(world, &child)
                    .into_iter()
                    .filter_map(|target| target.field(&access.field()).ok())
                    .collect();
            }
        }

        _ => {}
    }

    vec![]
}
