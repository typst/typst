use super::*;
use crate::diag::Deco;

impl Eval for Spanned<&ExprCall> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let name = &self.v.name.v;
        let span = self.v.name.span;

        if let Some(value) = ctx.scopes.get(name) {
            if let Value::Func(func) = value {
                let func = func.clone();
                ctx.deco(Deco::Resolved.with_span(span));

                let mut args = self.v.args.as_ref().eval(ctx);
                let returned = func(ctx, &mut args);
                args.finish(ctx);

                return returned;
            } else {
                let ty = value.type_name();
                ctx.diag(error!(span, "a value of type {} is not callable", ty));
            }
        } else if !name.is_empty() {
            ctx.diag(error!(span, "unknown function"));
        }

        ctx.deco(Deco::Unresolved.with_span(span));
        Value::Error
    }
}

impl Eval for Spanned<&ExprArgs> {
    type Output = Args;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let mut pos = vec![];
        let mut named = vec![];

        for arg in self.v {
            match arg {
                Argument::Pos(expr) => {
                    pos.push(expr.as_ref().eval(ctx).with_span(expr.span));
                }
                Argument::Named(Named { name, expr }) => {
                    named.push((
                        name.as_ref().map(|id| id.0.clone()),
                        expr.as_ref().eval(ctx).with_span(expr.span),
                    ));
                }
            }
        }

        Args { span: self.span, pos, named }
    }
}

/// Evaluated arguments to a function.
#[derive(Debug)]
pub struct Args {
    /// The span of the whole argument list.
    pub span: Span,
    /// The positional arguments.
    pub pos: SpanVec<Value>,
    /// The named arguments.
    pub named: Vec<(Spanned<String>, Spanned<Value>)>,
}

impl Args {
    /// Find and remove the first convertible positional argument.
    pub fn find<T>(&mut self, ctx: &mut EvalContext) -> Option<T>
    where
        T: Cast<Spanned<Value>>,
    {
        (0 .. self.pos.len()).find_map(move |i| try_cast(ctx, &mut self.pos, i))
    }

    /// Find and remove the first convertible positional argument, producing an
    /// error if no match was found.
    pub fn require<T>(&mut self, ctx: &mut EvalContext, what: &str) -> Option<T>
    where
        T: Cast<Spanned<Value>>,
    {
        let found = self.find(ctx);
        if found.is_none() {
            ctx.diag(error!(self.span, "missing argument: {}", what));
        }
        found
    }

    /// Filter out and remove all convertible positional arguments.
    pub fn filter<'a, 'b: 'a, T>(
        &'a mut self,
        ctx: &'a mut EvalContext<'b>,
    ) -> impl Iterator<Item = T> + Captures<'a> + Captures<'b>
    where
        T: Cast<Spanned<Value>>,
    {
        let mut i = 0;
        std::iter::from_fn(move || {
            while i < self.pos.len() {
                if let Some(val) = try_cast(ctx, &mut self.pos, i) {
                    return Some(val);
                }
                i += 1;
            }
            None
        })
    }

    /// Convert and remove the value for the given named argument, producing an
    /// error if the conversion fails.
    pub fn get<T>(&mut self, ctx: &mut EvalContext, name: &str) -> Option<T>
    where
        T: Cast<Spanned<Value>>,
    {
        let index = self.named.iter().position(|(k, _)| k.v.as_str() == name)?;
        let value = self.named.remove(index).1;
        cast(ctx, value)
    }

    /// Drain all remainings arguments into an array and a dictionary.
    pub fn drain(&mut self) -> (ValueArray, ValueDict) {
        let array = self.pos.drain(..).map(|s| s.v).collect();
        let dict = self.named.drain(..).map(|(k, v)| (k.v, v.v)).collect();
        (array, dict)
    }

    /// Produce "unexpected argument" errors for all remaining arguments.
    pub fn finish(self, ctx: &mut EvalContext) {
        let a = self.pos.iter().map(|v| v.as_ref());
        let b = self.named.iter().map(|(k, v)| (&v.v).with_span(k.span.join(v.span)));
        for value in a.chain(b) {
            if value.v != &Value::Error {
                ctx.diag(error!(value.span, "unexpected argument"));
            }
        }
    }
}

// This is a workaround because `-> impl Trait + 'a + 'b` does not work.
//
// See also: https://github.com/rust-lang/rust/issues/49431
#[doc(hidden)]
pub trait Captures<'a> {}
impl<'a, T: ?Sized> Captures<'a> for T {}

/// Cast the value into `T`, generating an error if the conversion fails.
fn cast<T>(ctx: &mut EvalContext, value: Spanned<Value>) -> Option<T>
where
    T: Cast<Spanned<Value>>,
{
    let span = value.span;
    match T::cast(value) {
        CastResult::Ok(t) => Some(t),
        CastResult::Warn(t, m) => {
            ctx.diag(warning!(span, "{}", m));
            Some(t)
        }
        CastResult::Err(value) => {
            ctx.diag(error!(
                span,
                "expected {}, found {}",
                T::TYPE_NAME,
                value.v.type_name()
            ));
            None
        }
    }
}

/// Try to cast the value in the slot into `T`, putting it back if the
/// conversion fails.
fn try_cast<T>(
    ctx: &mut EvalContext,
    vec: &mut Vec<Spanned<Value>>,
    i: usize,
) -> Option<T>
where
    T: Cast<Spanned<Value>>,
{
    // Replace with error placeholder when conversion works since error values
    // are ignored when generating "unexpected argument" errors.
    let slot = &mut vec[i];
    let value = std::mem::replace(slot, Spanned::zero(Value::None));
    let span = value.span;
    match T::cast(value) {
        CastResult::Ok(t) => {
            vec.remove(i);
            Some(t)
        }
        CastResult::Warn(t, m) => {
            vec.remove(i);
            ctx.diag(warning!(span, "{}", m));
            Some(t)
        }
        CastResult::Err(value) => {
            *slot = value;
            None
        }
    }
}
