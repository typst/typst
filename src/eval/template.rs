use super::*;
use crate::syntax::visit::*;

impl Eval for Spanned<&ExprTemplate> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let mut template = self.v.clone();
        let mut visitor = CapturesVisitor::new(ctx);
        visitor.visit_template(&mut template);
        Value::Template(template)
    }
}

/// A visitor that replaces all captured variables with their values.
struct CapturesVisitor<'a> {
    external: &'a Scopes<'a>,
    internal: Scopes<'a>,
}

impl<'a> CapturesVisitor<'a> {
    fn new(ctx: &'a EvalContext) -> Self {
        Self {
            external: &ctx.scopes,
            internal: Scopes::default(),
        }
    }
}

impl<'a> Visitor<'a> for CapturesVisitor<'a> {
    fn visit_scope_pre(&mut self) {
        self.internal.push();
    }

    fn visit_scope_post(&mut self) {
        self.internal.pop();
    }

    fn visit_def(&mut self, id: &mut Ident) {
        self.internal.define(id.as_str(), Value::None);
    }

    fn visit_expr(&mut self, expr: &'a mut Expr) {
        if let Expr::Ident(ident) = expr {
            // Find out whether the identifier is not locally defined, but
            // captured, and if so, replace it with it's value.
            if self.internal.get(ident).is_none() {
                if let Some(value) = self.external.get(ident) {
                    *expr = Expr::CapturedValue(value.clone());
                }
            }
        } else {
            walk_expr(self, expr);
        }
    }
}
