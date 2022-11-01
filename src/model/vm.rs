use std::path::PathBuf;

use comemo::Tracked;

use super::{Content, Route, Scopes, Value};
use crate::diag::{SourceError, StrResult};
use crate::syntax::{SourceId, Span};
use crate::util::{EcoString, PathExt};
use crate::{LangItems, World};

/// A virtual machine.
pub struct Vm<'a> {
    /// The core context.
    pub world: Tracked<'a, dyn World>,
    /// The route of source ids the VM took to reach its current location.
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

        Err("cannot access file system from here".into())
    }

    /// The language items.
    pub fn items(&self) -> &LangItems {
        &self.world.config().items
    }

    /// Create text content.
    ///
    /// This is a shorthand for `(vm.items().text)(..)`.
    pub fn text(&self, text: impl Into<EcoString>) -> Content {
        (self.items().text)(text.into())
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
