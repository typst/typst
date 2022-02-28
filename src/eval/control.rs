use super::{ops, EvalResult, Value};
use crate::diag::{At, Error, TypError};
use crate::syntax::Span;

/// A control flow event that occurred during evaluation.
#[derive(Clone, Debug, PartialEq)]
pub enum Control {
    /// Stop iteration in a loop.
    Break(Value, Span),
    /// Skip the remainder of the current iteration in a loop.
    Continue(Value, Span),
    /// Stop execution of a function early, returning a value. The bool
    /// indicates whether this was an explicit return with value.
    Return(Value, bool, Span),
    /// Stop the execution because an error occurred.
    Err(TypError),
}

impl From<TypError> for Control {
    fn from(error: TypError) -> Self {
        Self::Err(error)
    }
}

impl From<Control> for TypError {
    fn from(control: Control) -> Self {
        match control {
            Control::Break(_, span) => Error::boxed(span, "cannot break outside of loop"),
            Control::Continue(_, span) => {
                Error::boxed(span, "cannot continue outside of loop")
            }
            Control::Return(_, _, span) => {
                Error::boxed(span, "cannot return outside of function")
            }
            Control::Err(e) => e,
        }
    }
}

/// Join a value with an evaluated result.
pub(super) fn join_result(
    prev: Value,
    result: EvalResult<Value>,
    result_span: Span,
) -> EvalResult<Value> {
    match result {
        Ok(value) => Ok(ops::join(prev, value).at(result_span)?),
        Err(Control::Break(value, span)) => Err(Control::Break(
            ops::join(prev, value).at(result_span)?,
            span,
        )),
        Err(Control::Continue(value, span)) => Err(Control::Continue(
            ops::join(prev, value).at(result_span)?,
            span,
        )),
        Err(Control::Return(value, false, span)) => Err(Control::Return(
            ops::join(prev, value).at(result_span)?,
            false,
            span,
        )),
        other => other,
    }
}
