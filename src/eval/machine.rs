use std::path::PathBuf;

use super::{Scopes, Value};
use crate::diag::{StrResult, TypError};
use crate::source::SourceId;
use crate::syntax::Span;
use crate::util::PathExt;
use crate::Context;

/// A virtual machine.
pub struct Machine<'a> {
    /// The core context.
    pub ctx: &'a mut Context,
    /// The route of source ids at which the machine is located.
    pub route: Vec<SourceId>,
    /// The stack of scopes.
    pub scopes: Scopes<'a>,
    /// A control flow event that is currently happening.
    pub flow: Option<Flow>,
}

impl<'a> Machine<'a> {
    /// Create a new virtual machine.
    pub fn new(ctx: &'a mut Context, route: Vec<SourceId>, scopes: Scopes<'a>) -> Self {
        Self { ctx, route, scopes, flow: None }
    }

    /// Resolve a user-entered path to be relative to the compilation
    /// environment's root.
    pub fn locate(&self, path: &str) -> StrResult<PathBuf> {
        if let Some(&id) = self.route.last() {
            if let Some(path) = path.strip_prefix('/') {
                return Ok(self.ctx.config.root.join(path).normalize());
            }

            if let Some(dir) = self.ctx.sources.get(id).path().parent() {
                return Ok(dir.join(path).normalize());
            }
        }

        return Err("cannot access file system from here".into());
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
