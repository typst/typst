use std::path::PathBuf;

use comemo::Tracked;

use super::{Route, Scopes, Value};
use crate::diag::{SourceError, StrResult};
use crate::syntax::{SourceId, Span};
use crate::util::PathExt;
use crate::World;

/// A virtual machine.
pub struct Vm<'a> {
    /// The core context.
    pub world: Tracked<'a, dyn World>,
    /// The route of source ids the machine took to reach its current location.
    pub route: Tracked<'a, Route>,
    /// The current location.
    pub location: Option<SourceId>,
    /// The stack of scopes.
    pub scopes: Scopes<'a>,
    /// A control flow event that is currently happening.
    pub flow: Option<Flow>,
}

impl<'a> Vm<'a> {
    /// Create a new virtual machine.
    pub fn new(
        world: Tracked<'a, dyn World>,
        route: Tracked<'a, Route>,
        location: Option<SourceId>,
        scopes: Scopes<'a>,
    ) -> Self {
        Self {
            world,
            route,
            location,
            scopes,
            flow: None,
        }
    }

    /// Resolve a user-entered path to be relative to the compilation
    /// environment's root.
    pub fn locate(&self, path: &str) -> StrResult<PathBuf> {
        if let Some(id) = self.location {
            if let Some(path) = path.strip_prefix('/') {
                return Ok(self.world.config().root.join(path).normalize());
            }

            if let Some(dir) = self.world.source(id).path().parent() {
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
    pub fn forbidden(&self) -> SourceError {
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
