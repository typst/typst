//! Evaluation of syntax trees into layout trees.

#[macro_use]
mod value;
mod call;
mod context;
mod ops;
mod scope;
mod state;

pub use call::*;
pub use context::*;
pub use scope::*;
pub use state::*;
pub use value::*;

use std::rc::Rc;

use crate::color::Color;
use crate::diag::Pass;
use crate::env::Env;
use crate::geom::{Angle, Length, Relative, Spec};
use crate::layout::{self, Expansion, NodeSpacing, NodeStack};
use crate::syntax::*;

/// Evaluate a syntax tree into a layout tree.
///
/// The `state` is the base state that may be updated over the course of
/// evaluation. The `scope` similarly consists of the base definitions that are
/// present from the beginning (typically, the standard library).
pub fn eval(
    tree: &Tree,
    env: &mut Env,
    scope: &Scope,
    state: State,
) -> Pass<layout::Tree> {
    let mut ctx = EvalContext::new(env, scope, state);
    ctx.start_page_group(Softness::Hard);
    tree.eval(&mut ctx);
    ctx.end_page_group(|s| s == Softness::Hard);
    ctx.finish()
}

/// Evaluate an item.
///
/// _Note_: Evaluation is not necessarily pure, it may change the active state.
pub trait Eval {
    /// The output of evaluating the item.
    type Output;

    /// Evaluate the item to the output value.
    fn eval(self, ctx: &mut EvalContext) -> Self::Output;
}

impl<'a, T> Eval for &'a Spanned<T>
where
    Spanned<&'a T>: Eval,
{
    type Output = <Spanned<&'a T> as Eval>::Output;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        self.as_ref().eval(ctx)
    }
}

impl Eval for &[Spanned<Node>] {
    type Output = ();

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        for node in self {
            node.eval(ctx);
        }
    }
}

impl Eval for Spanned<&Node> {
    type Output = ();

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        match self.v {
            Node::Text(text) => {
                let node = ctx.make_text_node(text.clone());
                ctx.push(node);
            }
            Node::Space => {
                let em = ctx.state.font.font_size();
                ctx.push(NodeSpacing {
                    amount: ctx.state.par.word_spacing.resolve(em),
                    softness: Softness::Soft,
                });
            }
            Node::Linebreak => ctx.apply_linebreak(),
            Node::Parbreak => ctx.apply_parbreak(),
            Node::Strong => ctx.state.font.strong ^= true,
            Node::Emph => ctx.state.font.emph ^= true,
            Node::Heading(heading) => heading.with_span(self.span).eval(ctx),
            Node::Raw(raw) => raw.with_span(self.span).eval(ctx),
            Node::Expr(expr) => {
                let value = expr.with_span(self.span).eval(ctx);
                value.eval(ctx)
            }
        }
    }
}

impl Eval for Spanned<&NodeHeading> {
    type Output = ();

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let prev = ctx.state.clone();
        let upscale = 1.5 - 0.1 * self.v.level.v as f64;
        ctx.state.font.scale *= upscale;
        ctx.state.font.strong = true;

        self.v.contents.eval(ctx);
        ctx.apply_parbreak();

        ctx.state = prev;
    }
}

impl Eval for Spanned<&NodeRaw> {
    type Output = ();

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let prev = Rc::clone(&ctx.state.font.families);
        let families = ctx.state.font.families_mut();
        families.list.insert(0, "monospace".to_string());
        families.flatten();

        let em = ctx.state.font.font_size();
        let line_spacing = ctx.state.par.line_spacing.resolve(em);

        let mut children = vec![];
        for line in &self.v.lines {
            children.push(layout::Node::Text(ctx.make_text_node(line.clone())));
            children.push(layout::Node::Spacing(NodeSpacing {
                amount: line_spacing,
                softness: Softness::Hard,
            }));
        }

        ctx.push(NodeStack {
            dirs: ctx.state.dirs,
            align: ctx.state.align,
            expand: Spec::uniform(Expansion::Fit),
            children,
        });

        ctx.state.font.families = prev;
    }
}

impl Eval for Spanned<&Expr> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        match self.v {
            Expr::None => Value::None,
            Expr::Ident(v) => match ctx.scopes.get(v) {
                Some(value) => value.clone(),
                None => {
                    ctx.diag(error!(self.span, "unknown variable"));
                    Value::Error
                }
            },
            &Expr::Bool(v) => Value::Bool(v),
            &Expr::Int(v) => Value::Int(v),
            &Expr::Float(v) => Value::Float(v),
            &Expr::Length(v, unit) => Value::Length(Length::with_unit(v, unit)),
            &Expr::Angle(v, unit) => Value::Angle(Angle::with_unit(v, unit)),
            &Expr::Percent(v) => Value::Relative(Relative::new(v / 100.0)),
            &Expr::Color(v) => Value::Color(Color::Rgba(v)),
            Expr::Str(v) => Value::Str(v.clone()),
            Expr::Array(v) => Value::Array(v.with_span(self.span).eval(ctx)),
            Expr::Dict(v) => Value::Dict(v.with_span(self.span).eval(ctx)),
            Expr::Template(v) => Value::Template(v.clone()),
            Expr::Group(v) => v.eval(ctx),
            Expr::Block(v) => v.with_span(self.span).eval(ctx),
            Expr::Call(v) => v.with_span(self.span).eval(ctx),
            Expr::Unary(v) => v.with_span(self.span).eval(ctx),
            Expr::Binary(v) => v.with_span(self.span).eval(ctx),
            Expr::Let(v) => v.with_span(self.span).eval(ctx),
            Expr::If(v) => v.with_span(self.span).eval(ctx),
        }
    }
}

impl Eval for Spanned<&ExprArray> {
    type Output = ValueArray;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        self.v.iter().map(|expr| expr.eval(ctx)).collect()
    }
}

impl Eval for Spanned<&ExprDict> {
    type Output = ValueDict;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        self.v
            .iter()
            .map(|Named { name, expr }| (name.v.0.clone(), expr.eval(ctx)))
            .collect()
    }
}

impl Eval for Spanned<&ExprBlock> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let mut output = Value::None;
        for expr in &self.v.exprs {
            output = expr.eval(ctx);
        }
        output
    }
}

impl Eval for Spanned<&ExprUnary> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let value = self.v.expr.eval(ctx);
        if value == Value::Error {
            return Value::Error;
        }

        let ty = value.type_name();
        let out = match self.v.op.v {
            UnOp::Pos => ops::pos(value),
            UnOp::Neg => ops::neg(value),
            UnOp::Not => ops::not(value),
        };

        if out == Value::Error {
            ctx.diag(error!(
                self.span,
                "cannot apply '{}' to {}",
                self.v.op.v.as_str(),
                ty,
            ));
        }

        out
    }
}

impl Eval for Spanned<&ExprBinary> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        match self.v.op.v {
            BinOp::Add => self.apply(ctx, ops::add),
            BinOp::Sub => self.apply(ctx, ops::sub),
            BinOp::Mul => self.apply(ctx, ops::mul),
            BinOp::Div => self.apply(ctx, ops::div),
            BinOp::And => self.apply(ctx, ops::and),
            BinOp::Or => self.apply(ctx, ops::or),
            BinOp::Eq => self.apply(ctx, ops::eq),
            BinOp::Neq => self.apply(ctx, ops::neq),
            BinOp::Lt => self.apply(ctx, ops::lt),
            BinOp::Leq => self.apply(ctx, ops::leq),
            BinOp::Gt => self.apply(ctx, ops::gt),
            BinOp::Geq => self.apply(ctx, ops::geq),
            BinOp::Assign => self.assign(ctx, |_, b| b),
            BinOp::AddAssign => self.assign(ctx, ops::add),
            BinOp::SubAssign => self.assign(ctx, ops::sub),
            BinOp::MulAssign => self.assign(ctx, ops::mul),
            BinOp::DivAssign => self.assign(ctx, ops::div),
        }
    }
}

impl Spanned<&ExprBinary> {
    /// Apply a basic binary operation.
    fn apply<F>(&self, ctx: &mut EvalContext, op: F) -> Value
    where
        F: FnOnce(Value, Value) -> Value,
    {
        let lhs = self.v.lhs.eval(ctx);

        // Short-circuit boolean operations.
        match (self.v.op.v, &lhs) {
            (BinOp::And, Value::Bool(false)) => return lhs,
            (BinOp::Or, Value::Bool(true)) => return lhs,
            _ => {}
        }

        let rhs = self.v.rhs.eval(ctx);

        if lhs == Value::Error || rhs == Value::Error {
            return Value::Error;
        }

        let lhty = lhs.type_name();
        let rhty = rhs.type_name();
        let out = op(lhs, rhs);
        if out == Value::Error {
            ctx.diag(error!(
                self.span,
                "cannot apply '{}' to {} and {}",
                self.v.op.v.as_str(),
                lhty,
                rhty,
            ));
        }

        out
    }

    /// Apply an assignment operation.
    fn assign<F>(&self, ctx: &mut EvalContext, op: F) -> Value
    where
        F: FnOnce(Value, Value) -> Value,
    {
        let rhs = self.v.rhs.eval(ctx);
        let span = self.v.lhs.span;

        if let Expr::Ident(id) = &self.v.lhs.v {
            if let Some(slot) = ctx.scopes.get_mut(id) {
                let lhs = std::mem::replace(slot, Value::None);
                *slot = op(lhs, rhs);
                return Value::None;
            } else if ctx.scopes.is_const(id) {
                ctx.diag(error!(span, "cannot assign to constant"));
            } else {
                ctx.diag(error!(span, "unknown variable"));
            }
        } else {
            ctx.diag(error!(span, "cannot assign to this expression"));
        }

        Value::Error
    }
}

impl Eval for Spanned<&ExprLet> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let value = match &self.v.init {
            Some(expr) => expr.eval(ctx),
            None => Value::None,
        };
        ctx.scopes.define(self.v.pat.v.as_str(), value);
        Value::None
    }
}

impl Eval for Spanned<&ExprIf> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let condition = self.v.condition.eval(ctx);
        if let Value::Bool(boolean) = condition {
            return if boolean {
                self.v.if_body.eval(ctx)
            } else if let Some(expr) = &self.v.else_body {
                expr.eval(ctx)
            } else {
                Value::None
            };
        } else if condition != Value::Error {
            ctx.diag(error!(
                self.v.condition.span,
                "expected boolean, found {}",
                condition.type_name(),
            ));
        }

        Value::Error
    }
}
