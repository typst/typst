use super::{Scopes, Value};
use crate::diag::TypError;
use crate::syntax::Span;
use crate::Context;

/// A virtual machine.
pub struct Machine<'a> {
    /// The core context.
    pub ctx: &'a mut Context,
    /// The stack of scopes.
    pub scopes: Scopes<'a>,
    /// A control flow event that is currently happening.
    pub flow: Option<Flow>,
}

impl<'a> Machine<'a> {
    /// Create a new virtual machine.
    pub fn new(ctx: &'a mut Context, scopes: Scopes<'a>) -> Self {
        Self { ctx, scopes, flow: None }
    }
}

/// A control flow event that occurred during evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum Flow {
    /// Stop iteration in a loop.
    Break(Span),
    /// Skip the remainder of the current iteration in a loop.
    Continue(Span),
    /// Stop execution of a function early, optionally returning an explicit
    /// value.
    Return(Span, Option<Value>),
}

impl Flow {
    /// Return an error stating that this control flow is forbidden.
    pub fn forbidden(&self) -> TypError {
        match *self {
            Self::Break(span) => {
                error!(span, "cannot break outside of loop")
            }
            Self::Continue(span) => {
                error!(span, "cannot continue outside of loop")
            }
            Self::Return(span, _) => {
                error!(span, "cannot return outside of function")
            }
        }
    }
}
