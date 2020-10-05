//! Evaluation of syntax trees.

mod args;
mod convert;
mod dict;
mod scope;
mod state;
mod value;

pub use args::*;
pub use convert::*;
pub use dict::*;
pub use scope::*;
pub use state::*;
pub use value::*;

use async_trait::async_trait;

use crate::layout::LayoutContext;
use crate::syntax::*;

/// Evaluate an syntactic item into an output value.
///
/// _Note_: Evaluation is not necessarily pure, it may change the active state.
#[async_trait(?Send)]
pub trait Eval {
    /// The output of evaluating the item.
    type Output;

    /// Evaluate the item to the output value.
    async fn eval(&self, ctx: &mut LayoutContext) -> Self::Output;
}

#[async_trait(?Send)]
impl Eval for Expr {
    type Output = Value;

    async fn eval(&self, ctx: &mut LayoutContext) -> Self::Output {
        match self {
            Self::Lit(lit) => lit.eval(ctx).await,
            Self::Call(call) => call.eval(ctx).await,
            Self::Unary(unary) => unary.eval(ctx).await,
            Self::Binary(binary) => binary.eval(ctx).await,
        }
    }
}

#[async_trait(?Send)]
impl Eval for Lit {
    type Output = Value;

    async fn eval(&self, ctx: &mut LayoutContext) -> Self::Output {
        match *self {
            Lit::Ident(ref v) => Value::Ident(v.clone()),
            Lit::Bool(v) => Value::Bool(v),
            Lit::Int(v) => Value::Int(v),
            Lit::Float(v) => Value::Float(v),
            Lit::Length(v) => Value::Length(v.as_raw()),
            Lit::Percent(v) => Value::Relative(v / 100.0),
            Lit::Color(v) => Value::Color(v),
            Lit::Str(ref v) => Value::Str(v.clone()),
            Lit::Dict(ref v) => Value::Dict(v.eval(ctx).await),
            Lit::Content(ref v) => Value::Content(v.clone()),
        }
    }
}
#[async_trait(?Send)]
impl Eval for LitDict {
    type Output = ValueDict;

    async fn eval(&self, ctx: &mut LayoutContext) -> Self::Output {
        let mut dict = ValueDict::new();

        for entry in &self.0 {
            let val = entry.expr.v.eval(ctx).await;
            let spanned = val.span_with(entry.expr.span);
            if let Some(key) = &entry.key {
                dict.insert(&key.v, SpannedEntry::new(key.span, spanned));
            } else {
                dict.push(SpannedEntry::value(spanned));
            }
        }

        dict
    }
}

#[async_trait(?Send)]
impl Eval for ExprCall {
    type Output = Value;

    async fn eval(&self, ctx: &mut LayoutContext) -> Self::Output {
        let name = &self.name.v;
        let span = self.name.span;
        let dict = self.args.v.eval(ctx).await;

        if let Some(func) = ctx.state.scope.get(name) {
            let args = Args(dict.span_with(self.args.span));
            ctx.f.decos.push(Deco::Resolved.span_with(span));
            (func.clone())(args, ctx).await
        } else {
            if !name.is_empty() {
                ctx.diag(error!(span, "unknown function"));
                ctx.f.decos.push(Deco::Unresolved.span_with(span));
            }
            Value::Dict(dict)
        }
    }
}

#[async_trait(?Send)]
impl Eval for ExprUnary {
    type Output = Value;

    async fn eval(&self, ctx: &mut LayoutContext) -> Self::Output {
        use Value::*;

        let value = self.expr.v.eval(ctx).await;
        if value == Error {
            return Error;
        }

        let span = self.op.span.join(self.expr.span);
        match self.op.v {
            UnOp::Neg => neg(ctx, span, value),
        }
    }
}

#[async_trait(?Send)]
impl Eval for ExprBinary {
    type Output = Value;

    async fn eval(&self, ctx: &mut LayoutContext) -> Self::Output {
        let lhs = self.lhs.v.eval(ctx).await;
        let rhs = self.rhs.v.eval(ctx).await;

        if lhs == Value::Error || rhs == Value::Error {
            return Value::Error;
        }

        let span = self.lhs.span.join(self.rhs.span);
        match self.op.v {
            BinOp::Add => add(ctx, span, lhs, rhs),
            BinOp::Sub => sub(ctx, span, lhs, rhs),
            BinOp::Mul => mul(ctx, span, lhs, rhs),
            BinOp::Div => div(ctx, span, lhs, rhs),
        }
    }
}

/// Compute the negation of a value.
fn neg(ctx: &mut LayoutContext, span: Span, value: Value) -> Value {
    use Value::*;
    match value {
        Int(v) => Int(-v),
        Float(v) => Float(-v),
        Length(v) => Length(-v),
        Relative(v) => Relative(-v),
        Linear(v) => Linear(-v),
        v => {
            ctx.diag(error!(span, "cannot negate {}", v.ty()));
            Value::Error
        }
    }
}

/// Compute the sum of two values.
fn add(ctx: &mut LayoutContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use crate::geom::Linear as Lin;
    use Value::*;
    match (lhs, rhs) {
        // Numbers to themselves.
        (Int(a), Int(b)) => Int(a + b),
        (Int(a), Float(b)) => Float(a as f64 + b),
        (Float(a), Int(b)) => Float(a + b as f64),
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
        (Content(a), Content(b)) => Content(concat(a, b)),
        (Commands(a), Commands(b)) => Commands(concat(a, b)),

        (a, b) => {
            ctx.diag(error!(span, "cannot add {} and {}", a.ty(), b.ty()));
            Value::Error
        }
    }
}

/// Compute the difference of two values.
fn sub(ctx: &mut LayoutContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use crate::geom::Linear as Lin;
    use Value::*;
    match (lhs, rhs) {
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
            ctx.diag(error!(span, "cannot subtract {1} from {0}", a.ty(), b.ty()));
            Value::Error
        }
    }
}

/// Compute the product of two values.
fn mul(ctx: &mut LayoutContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
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
            ctx.diag(error!(span, "cannot multiply {} with {}", a.ty(), b.ty()));
            Value::Error
        }
    }
}

/// Compute the quotient of two values.
fn div(ctx: &mut LayoutContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
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
            ctx.diag(error!(span, "cannot divide {} by {}", a.ty(), b.ty()));
            Value::Error
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
