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
            node.as_ref().eval(ctx);
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
            Expr::Bool(v) => Value::Bool(*v),
            Expr::Int(v) => Value::Int(*v),
            Expr::Float(v) => Value::Float(*v),
            Expr::Length(v, unit) => Value::Length(Length::with_unit(*v, *unit)),
            Expr::Angle(v, unit) => Value::Angle(Angle::with_unit(*v, *unit)),
            Expr::Percent(v) => Value::Relative(Relative::new(v / 100.0)),
            Expr::Color(v) => Value::Color(Color::Rgba(*v)),
            Expr::Str(v) => Value::Str(v.clone()),
            Expr::Call(v) => v.with_span(self.span).eval(ctx),
            Expr::Unary(v) => v.with_span(self.span).eval(ctx),
            Expr::Binary(v) => v.with_span(self.span).eval(ctx),
            Expr::Array(v) => Value::Array(v.with_span(self.span).eval(ctx)),
            Expr::Dict(v) => Value::Dict(v.with_span(self.span).eval(ctx)),
            Expr::Template(v) => Value::Template(v.clone()),
            Expr::Group(v) => v.as_ref().with_span(self.span).eval(ctx),
            Expr::Block(v) => v.as_ref().with_span(self.span).eval(ctx),
            Expr::Let(v) => {
                let value = match &v.expr {
                    Some(expr) => expr.as_ref().eval(ctx),
                    None => Value::None,
                };
                ctx.scopes.define(v.pat.v.as_str(), value);
                Value::None
            }
        }
    }
}

impl Eval for Spanned<&ExprUnary> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let value = self.v.expr.as_ref().eval(ctx);

        if let Value::Error = value {
            return Value::Error;
        }

        let span = self.v.op.span.join(self.v.expr.span);
        match self.v.op.v {
            UnOp::Pos => ops::pos(ctx, span, value),
            UnOp::Neg => ops::neg(ctx, span, value),
        }
    }
}

impl Eval for Spanned<&ExprBinary> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let lhs = self.v.lhs.as_ref().eval(ctx);
        let rhs = self.v.rhs.as_ref().eval(ctx);

        if lhs == Value::Error || rhs == Value::Error {
            return Value::Error;
        }

        let span = self.v.lhs.span.join(self.v.rhs.span);
        match self.v.op.v {
            BinOp::Add => ops::add(ctx, span, lhs, rhs),
            BinOp::Sub => ops::sub(ctx, span, lhs, rhs),
            BinOp::Mul => ops::mul(ctx, span, lhs, rhs),
            BinOp::Div => ops::div(ctx, span, lhs, rhs),
        }
    }
}

impl Eval for Spanned<&ExprArray> {
    type Output = ValueArray;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        self.v.iter().map(|expr| expr.as_ref().eval(ctx)).collect()
    }
}

impl Eval for Spanned<&ExprDict> {
    type Output = ValueDict;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        self.v
            .iter()
            .map(|Named { name, expr }| (name.v.0.clone(), expr.as_ref().eval(ctx)))
            .collect()
    }
}
