//! Evaluation of syntax trees into layout trees.

#[macro_use]
mod value;
mod call;
mod context;
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
use crate::env::SharedEnv;
use crate::geom::{Gen, Length, Relative};
use crate::layout::{self, Expansion, NodeSpacing, NodeStack};
use crate::syntax::*;

/// Evaluate a syntax tree into a layout tree.
///
/// The given `state` is the base state that may be updated over the course of
/// evaluation.
pub fn eval(tree: &Tree, env: SharedEnv, state: State) -> Pass<layout::Tree> {
    let mut ctx = EvalContext::new(env, state);
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

impl<'a, T> Eval for &'a Box<Spanned<T>>
where
    Spanned<&'a T>: Eval,
{
    type Output = <Spanned<&'a T> as Eval>::Output;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        (**self).as_ref().eval(ctx)
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
        let families = Rc::make_mut(&mut ctx.state.font.families);
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
            expansion: Gen::uniform(Expansion::Fit),
            children,
        });

        ctx.state.font.families = prev;
    }
}

impl Eval for Spanned<&Expr> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        match self.v {
            Expr::Lit(v) => v.with_span(self.span).eval(ctx),
            Expr::Call(v) => v.with_span(self.span).eval(ctx),
            Expr::Unary(v) => v.with_span(self.span).eval(ctx),
            Expr::Binary(v) => v.with_span(self.span).eval(ctx),
            Expr::Array(v) => Value::Array(v.with_span(self.span).eval(ctx)),
            Expr::Dict(v) => Value::Dict(v.with_span(self.span).eval(ctx)),
            Expr::Content(v) => Value::Content(v.clone()),
        }
    }
}

impl Eval for Spanned<&Lit> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        match *self.v {
            Lit::Ident(ref v) => match ctx.state.scope.get(&v) {
                Some(value) => value.clone(),
                None => {
                    ctx.diag(error!(self.span, "unknown variable"));
                    Value::Error
                }
            },
            Lit::None => Value::None,
            Lit::Bool(v) => Value::Bool(v),
            Lit::Int(v) => Value::Int(v),
            Lit::Float(v) => Value::Float(v),
            Lit::Length(v, unit) => Value::Length(Length::with_unit(v, unit)),
            Lit::Percent(v) => Value::Relative(Relative::new(v / 100.0)),
            Lit::Color(v) => Value::Color(Color::Rgba(v)),
            Lit::Str(ref v) => Value::Str(v.clone()),
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

impl Eval for Spanned<&ExprUnary> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let value = self.v.expr.eval(ctx);

        if let Value::Error = value {
            return Value::Error;
        }

        let span = self.v.op.span.join(self.v.expr.span);
        match self.v.op.v {
            UnOp::Neg => neg(ctx, span, value),
        }
    }
}

impl Eval for Spanned<&ExprBinary> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let lhs = self.v.lhs.eval(ctx);
        let rhs = self.v.rhs.eval(ctx);

        if lhs == Value::Error || rhs == Value::Error {
            return Value::Error;
        }

        let span = self.v.lhs.span.join(self.v.rhs.span);
        match self.v.op.v {
            BinOp::Add => add(ctx, span, lhs, rhs),
            BinOp::Sub => sub(ctx, span, lhs, rhs),
            BinOp::Mul => mul(ctx, span, lhs, rhs),
            BinOp::Div => div(ctx, span, lhs, rhs),
        }
    }
}

/// Compute the negation of a value.
fn neg(ctx: &mut EvalContext, span: Span, value: Value) -> Value {
    use Value::*;
    match value {
        Int(v) => Int(-v),
        Float(v) => Float(-v),
        Length(v) => Length(-v),
        Relative(v) => Relative(-v),
        Linear(v) => Linear(-v),
        v => {
            ctx.diag(error!(span, "cannot negate {}", v.type_name()));
            Value::Error
        }
    }
}

/// Compute the sum of two values.
fn add(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
        // Numbers to themselves.
        (Int(a), Int(b)) => Int(a + b),
        (Int(a), Float(b)) => Float(a as f64 + b),
        (Float(a), Int(b)) => Float(a + b as f64),
        (Float(a), Float(b)) => Float(a + b),

        // Lengths, relatives and linears to themselves.
        (Length(a), Length(b)) => Length(a + b),
        (Length(a), Relative(b)) => Linear(a + b),
        (Length(a), Linear(b)) => Linear(a + b),

        (Relative(a), Length(b)) => Linear(a + b),
        (Relative(a), Relative(b)) => Relative(a + b),
        (Relative(a), Linear(b)) => Linear(a + b),

        (Linear(a), Length(b)) => Linear(a + b),
        (Linear(a), Relative(b)) => Linear(a + b),
        (Linear(a), Linear(b)) => Linear(a + b),

        // Complex data types to themselves.
        (Str(a), Str(b)) => Str(a + &b),
        (Array(a), Array(b)) => Array(concat(a, b)),
        (Dict(a), Dict(b)) => Dict(concat(a, b)),
        (Content(a), Content(b)) => Content(concat(a, b)),

        (a, b) => {
            ctx.diag(error!(
                span,
                "cannot add {} and {}",
                a.type_name(),
                b.type_name()
            ));
            Value::Error
        }
    }
}

/// Compute the difference of two values.
fn sub(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
        // Numbers from themselves.
        (Int(a), Int(b)) => Int(a - b),
        (Int(a), Float(b)) => Float(a as f64 - b),
        (Float(a), Int(b)) => Float(a - b as f64),
        (Float(a), Float(b)) => Float(a - b),

        // Lengths, relatives and linears from themselves.
        (Length(a), Length(b)) => Length(a - b),
        (Length(a), Relative(b)) => Linear(a - b),
        (Length(a), Linear(b)) => Linear(a - b),
        (Relative(a), Length(b)) => Linear(a - b),
        (Relative(a), Relative(b)) => Relative(a - b),
        (Relative(a), Linear(b)) => Linear(a - b),
        (Linear(a), Length(b)) => Linear(a - b),
        (Linear(a), Relative(b)) => Linear(a - b),
        (Linear(a), Linear(b)) => Linear(a - b),

        (a, b) => {
            ctx.diag(error!(
                span,
                "cannot subtract {1} from {0}",
                a.type_name(),
                b.type_name()
            ));
            Value::Error
        }
    }
}

/// Compute the product of two values.
fn mul(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
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
        (Int(a), Str(b)) => Str(b.repeat(0.max(a) as usize)),
        (Str(a), Int(b)) => Str(a.repeat(0.max(b) as usize)),

        (a, b) => {
            ctx.diag(error!(
                span,
                "cannot multiply {} with {}",
                a.type_name(),
                b.type_name()
            ));
            Value::Error
        }
    }
}

/// Compute the quotient of two values.
fn div(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
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
            ctx.diag(error!(
                span,
                "cannot divide {} by {}",
                a.type_name(),
                b.type_name()
            ));
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
