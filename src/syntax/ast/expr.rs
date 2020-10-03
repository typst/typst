//! Expressions.

use crate::eval::Value;
use crate::layout::LayoutContext;
use crate::syntax::{Decoration, Ident, Lit, LitDict, SpanWith, Spanned};
use crate::{DynFuture, Feedback};

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
    pub fn eval<'a>(
        &'a self,
        ctx: &'a LayoutContext<'a>,
        f: &'a mut Feedback,
    ) -> DynFuture<'a, Value> {
        Box::pin(async move {
            match self {
                Self::Lit(lit) => lit.eval(ctx, f).await,
                Self::Unary(unary) => unary.eval(ctx, f).await,
                Self::Binary(binary) => binary.eval(ctx, f).await,
                Self::Call(call) => call.eval(ctx, f).await,
            }
        })
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
    pub async fn eval(&self, ctx: &LayoutContext<'_>, f: &mut Feedback) -> Value {
        use Value::*;

        let value = self.expr.v.eval(ctx, f).await;
        if value == Error {
            return Error;
        }

        let span = self.op.span.join(self.expr.span);
        match self.op.v {
            UnOp::Neg => match value {
                Int(x) => Int(-x),
                Float(x) => Float(-x),
                Length(x) => Length(-x),
                Relative(x) => Relative(-x),
                Linear(x) => Linear(-x),
                v => {
                    error!(@f, span, "cannot negate {}", v.name());
                    Value::Error
                }
            },
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
    pub async fn eval(&self, ctx: &LayoutContext<'_>, f: &mut Feedback) -> Value {
        use crate::geom::Linear as Lin;
        use Value::*;

        let lhs = self.lhs.v.eval(ctx, f).await;
        let rhs = self.rhs.v.eval(ctx, f).await;

        if lhs == Error || rhs == Error {
            return Error;
        }

        let span = self.lhs.span.join(self.rhs.span);
        match self.op.v {
            BinOp::Add => match (lhs, rhs) {
                // Numbers to themselves.
                (Int(a), Int(b)) => Int(a + b),
                (Int(i), Float(f)) | (Float(f), Int(i)) => Float(i as f64 + f),
                (Float(a), Float(b)) => Float(a + b),

                // Lengths, relatives and linears to themselves.
                (Length(a), Length(b)) => Length(a + b),
                (Length(a), Relative(b)) => Linear(Lin::abs(a) + Lin::rel(b)),
                (Length(a), Linear(b)) => Linear(Lin::abs(a) + b),

                (Relative(a), Length(b)) => Linear(Lin::rel(a) + Lin::abs(b)),
                (Relative(a), Relative(b)) => Relative(a + b),
                (Relative(a), Linear(b)) => Linear(Lin::rel(a) + b),

                (Linear(a), Length(b)) => Linear(a + Lin::abs(b)),
                (Linear(a), Relative(b)) => Linear(a + Lin::rel(b)),
                (Linear(a), Linear(b)) => Linear(a + b),

                // Complex data types to themselves.
                (Str(a), Str(b)) => Str(a + &b),
                (Dict(a), Dict(b)) => Dict(concat(a, b)),
                (Tree(a), Tree(b)) => Tree(concat(a, b)),
                (Commands(a), Commands(b)) => Commands(concat(a, b)),

                (a, b) => {
                    error!(@f, span, "cannot add {} and {}", a.name(), b.name());
                    Value::Error
                }
            },

            BinOp::Sub => match (lhs, rhs) {
                // Numbers from themselves.
                (Int(a), Int(b)) => Int(a - b),
                (Int(a), Float(b)) => Float(a as f64 - b),
                (Float(a), Int(b)) => Float(a - b as f64),
                (Float(a), Float(b)) => Float(a - b),

                // Lengths, relatives and linears from themselves.
                (Length(a), Length(b)) => Length(a - b),
                (Length(a), Relative(b)) => Linear(Lin::abs(a) - Lin::rel(b)),
                (Length(a), Linear(b)) => Linear(Lin::abs(a) - b),
                (Relative(a), Length(b)) => Linear(Lin::rel(a) - Lin::abs(b)),
                (Relative(a), Relative(b)) => Relative(a - b),
                (Relative(a), Linear(b)) => Linear(Lin::rel(a) - b),
                (Linear(a), Length(b)) => Linear(a - Lin::abs(b)),
                (Linear(a), Relative(b)) => Linear(a - Lin::rel(b)),
                (Linear(a), Linear(b)) => Linear(a - b),

                (a, b) => {
                    error!(@f, span, "cannot subtract {1} from {0}", a.name(), b.name());
                    Value::Error
                }
            },

            BinOp::Mul => match (lhs, rhs) {
                // Numbers with themselves.
                (Int(a), Int(b)) => Int(a * b),
                (Int(a), Float(b)) => Float(a as f64 * b),
                (Float(a), Int(b)) => Float(a * b as f64),
                (Float(a), Float(b)) => Float(a * b),

                // Lengths, relatives and linears with numbers.
                (Length(a), Int(b)) => Length(a * b as f64),
                (Length(a), Float(b)) => Length(a * b),
                (Int(a), Length(b)) => Length(a as f64 * b),
                (Float(a), Length(b)) => Length(a * b),
                (Relative(a), Int(b)) => Relative(a * b as f64),
                (Relative(a), Float(b)) => Relative(a * b),
                (Int(a), Relative(b)) => Relative(a as f64 * b),
                (Float(a), Relative(b)) => Relative(a * b),
                (Linear(a), Int(b)) => Linear(a * b as f64),
                (Linear(a), Float(b)) => Linear(a * b),
                (Int(a), Linear(b)) => Linear(a as f64 * b),
                (Float(a), Linear(b)) => Linear(a * b),

                // Integers with strings.
                (Int(a), Str(b)) => Str(b.repeat(a.max(0) as usize)),
                (Str(a), Int(b)) => Str(a.repeat(b.max(0) as usize)),

                (a, b) => {
                    error!(@f, span, "cannot multiply {} with {}", a.name(), b.name());
                    Value::Error
                }
            },

            BinOp::Div => match (lhs, rhs) {
                // Numbers by themselves.
                (Int(a), Int(b)) => Float(a as f64 / b as f64),
                (Int(a), Float(b)) => Float(a as f64 / b),
                (Float(a), Int(b)) => Float(a / b as f64),
                (Float(a), Float(b)) => Float(a / b),

                // Lengths by numbers.
                (Length(a), Int(b)) => Length(a / b as f64),
                (Length(a), Float(b)) => Length(a / b),
                (Relative(a), Int(b)) => Relative(a / b as f64),
                (Relative(a), Float(b)) => Relative(a / b),
                (Linear(a), Int(b)) => Linear(a / b as f64),
                (Linear(a), Float(b)) => Linear(a / b),

                (a, b) => {
                    error!(@f, span, "cannot divide {} by {}", a.name(), b.name());
                    Value::Error
                }
            },
        }
    }
}

/// Concatenate two collections.
fn concat<T, A>(mut a: T, b: T) -> T
where
    T: Extend<A> + IntoIterator<Item = A>,
{
    a.extend(b);
    a
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
