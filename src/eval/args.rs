use super::*;

/// Evaluated arguments to a function.
#[derive(Debug)]
pub struct Args {
    span: Span,
    pos: SpanVec<Value>,
    named: Vec<(Spanned<String>, Spanned<Value>)>,
}

impl Eval for Spanned<&Arguments> {
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

impl Args {
    /// Find the first convertible positional argument.
    pub fn find<T>(&mut self, ctx: &mut EvalContext) -> Option<T>
    where
        T: Cast<Spanned<Value>>,
    {
        self.pos.iter_mut().find_map(move |slot| try_cast(ctx, slot))
    }

    /// Find the first convertible positional argument, producing an error if
    /// no match was found.
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

    /// Filter out all convertible positional arguments.
    pub fn filter<'a, T>(
        &'a mut self,
        ctx: &'a mut EvalContext,
    ) -> impl Iterator<Item = T> + 'a
    where
        T: Cast<Spanned<Value>>,
    {
        self.pos.iter_mut().filter_map(move |slot| try_cast(ctx, slot))
    }

    /// Convert the value for the given named argument.
    ///
    /// Generates an error if the conversion fails.
    pub fn get<'a, T>(&mut self, ctx: &mut EvalContext, name: &str) -> Option<T>
    where
        T: Cast<Spanned<Value>>,
    {
        let index = self.named.iter().position(|(k, _)| k.v.as_str() == name)?;
        let value = self.named.remove(index).1;
        cast(ctx, value)
    }

    /// Generate "unexpected argument" errors for all remaining arguments.
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
fn try_cast<T>(ctx: &mut EvalContext, slot: &mut Spanned<Value>) -> Option<T>
where
    T: Cast<Spanned<Value>>,
{
    // Replace with error placeholder when conversion works since error values
    // are ignored when generating "unexpected argument" errors.
    let value = std::mem::replace(slot, Spanned::zero(Value::Error));
    let span = value.span;
    match T::cast(value) {
        CastResult::Ok(t) => Some(t),
        CastResult::Warn(t, m) => {
            ctx.diag(warning!(span, "{}", m));
            Some(t)
        }
        CastResult::Err(value) => {
            *slot = value;
            None
        }
    }
}
