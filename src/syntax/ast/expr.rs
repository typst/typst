//! Expressions.

use crate::eval::Value;
use crate::layout::LayoutContext;
use crate::syntax::{Decoration, Ident, Lit, LitDict, SpanWith, Spanned};
use crate::Feedback;

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal: `true`, `1cm`, `"hi"`, `{_Hey!_}`.
    Lit(Lit),
    /// A unary operation: `-x`.
    Unary(ExprUnary),
    /// A binary operation: `a + b`, `a / b`.
    Binary(ExprBinary),
    /// An invocation of a function: `[foo: ...]`, `foo(...)`.
    Call(ExprCall),
}

impl Expr {
    /// Evaluate the expression to a value.
    pub async fn eval(&self, ctx: &LayoutContext<'_>, f: &mut Feedback) -> Value {
        match self {
            Self::Lit(lit) => lit.eval(ctx, f).await,
            Self::Unary(unary) => unary.eval(ctx, f).await,
            Self::Binary(binary) => binary.eval(ctx, f).await,
            Self::Call(call) => call.eval(ctx, f).await,
        }
    }
}

/// A unary operation: `-x`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprUnary {
    /// The operator: `-`.
    pub op: Spanned<UnOp>,
    /// The expression to operator on: `x`.
    pub expr: Spanned<Box<Expr>>,
}

impl ExprUnary {
    /// Evaluate the expression to a value.
    pub async fn eval(&self, _: &LayoutContext<'_>, _: &mut Feedback) -> Value {
        match self.op.v {
            UnOp::Neg => todo!("eval neg"),
        }
    }
}

/// A unary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum UnOp {
    /// The negation operator: `-`.
    Neg,
}

/// A binary operation: `a + b`, `a / b`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprBinary {
    /// The left-hand side of the operation: `a`.
    pub lhs: Spanned<Box<Expr>>,
    /// The operator: `+`.
    pub op: Spanned<BinOp>,
    /// The right-hand side of the operation: `b`.
    pub rhs: Spanned<Box<Expr>>,
}

impl ExprBinary {
    /// Evaluate the expression to a value.
    pub async fn eval(&self, _: &LayoutContext<'_>, _: &mut Feedback) -> Value {
        match self.op.v {
            BinOp::Add => todo!("eval add"),
            BinOp::Sub => todo!("eval sub"),
            BinOp::Mul => todo!("eval mul"),
            BinOp::Div => todo!("eval div"),
        }
    }
}

/// A binary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BinOp {
    /// The addition operator: `+`.
    Add,
    /// The subtraction operator: `-`.
    Sub,
    /// The multiplication operator: `*`.
    Mul,
    /// The division operator: `/`.
    Div,
}

/// An invocation of a function: `[foo: ...]`, `foo(...)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprCall {
    /// The name of the function.
    pub name: Spanned<Ident>,
    /// The arguments to the function.
    pub args: LitDict,
}

impl ExprCall {
    /// Evaluate the call expression to a value.
    pub async fn eval(&self, ctx: &LayoutContext<'_>, f: &mut Feedback) -> Value {
        let name = &self.name.v;
        let span = self.name.span;
        let args = self.args.eval(ctx, f).await;

        if let Some(func) = ctx.scope.func(name) {
            let pass = func(span, args, ctx.clone()).await;
            f.extend(pass.feedback);
            f.decorations.push(Decoration::Resolved.span_with(span));
            pass.output
        } else {
            if !name.is_empty() {
                error!(@f, span, "unknown function");
                f.decorations.push(Decoration::Unresolved.span_with(span));
            }
            Value::Dict(args)
        }
    }
}
