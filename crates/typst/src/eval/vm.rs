use comemo::{Track, Tracked, Validate};

use crate::diag::{bail, StrResult};
use crate::eval::FlowEvent;
use crate::foundations::{IntoValue, Scopes};
use crate::layout::Vt;
use crate::syntax::ast::{self, AstNode};
use crate::syntax::{FileId, Span};
use crate::World;

/// A virtual machine.
///
/// Holds the state needed to [evaluate](crate::eval::eval()) Typst sources. A new
/// virtual machine is created for each module evaluation and function call.
pub struct Vm<'a> {
    /// The underlying virtual typesetter.
    pub(crate) vt: Vt<'a>,
    /// The route of source ids the VM took to reach its current location.
    pub(crate) route: Tracked<'a, Route<'a>>,
    /// The id of the currently evaluated file.
    pub(crate) file: Option<FileId>,
    /// A control flow event that is currently happening.
    pub(crate) flow: Option<FlowEvent>,
    /// The stack of scopes.
    pub(crate) scopes: Scopes<'a>,
    /// The current call depth.
    pub(crate) depth: usize,
    /// A span that is currently under inspection.
    pub(crate) inspected: Option<Span>,
}

impl<'a> Vm<'a> {
    /// Create a new virtual machine.
    pub fn new(
        vt: Vt<'a>,
        route: Tracked<'a, Route>,
        file: Option<FileId>,
        scopes: Scopes<'a>,
    ) -> Self {
        let inspected = file.and_then(|id| vt.tracer.inspected(id));
        Self {
            vt,
            route,
            file,
            flow: None,
            scopes,
            depth: 0,
            inspected,
        }
    }

    /// Access the underlying world.
    pub fn world(&self) -> Tracked<'a, dyn World + 'a> {
        self.vt.world
    }

    /// The id of the currently evaluated file.
    ///
    /// Returns `None` if the VM is in a detached context, e.g. when evaluating
    /// a user-provided string.
    pub fn file(&self) -> Option<FileId> {
        self.file
    }

    /// Resolve a path relative to the currently evaluated file.
    pub fn resolve_path(&self, path: &str) -> StrResult<FileId> {
        let Some(file) = self.file else {
            bail!("cannot access file system from here");
        };

        Ok(file.join(path))
    }

    /// Define a variable in the current scope.
    #[tracing::instrument(skip_all)]
    pub fn define(&mut self, var: ast::Ident, value: impl IntoValue) {
        let value = value.into_value();
        if self.inspected == Some(var.span()) {
            self.vt.tracer.value(value.clone());
        }
        self.scopes.top.define(var.get().clone(), value);
    }
}

/// A route of source ids.
#[derive(Default)]
pub struct Route<'a> {
    // We need to override the constraint's lifetime here so that `Tracked` is
    // covariant over the constraint. If it becomes invariant, we're in for a
    // world of lifetime pain.
    outer: Option<Tracked<'a, Self, <Route<'static> as Validate>::Constraint>>,
    id: Option<FileId>,
}

impl<'a> Route<'a> {
    /// Create a new route with just one entry.
    pub fn new(id: Option<FileId>) -> Self {
        Self { id, outer: None }
    }

    /// Insert a new id into the route.
    ///
    /// You must guarantee that `outer` lives longer than the resulting
    /// route is ever used.
    pub fn insert(outer: Tracked<'a, Self>, id: FileId) -> Self {
        Route { outer: Some(outer), id: Some(id) }
    }

    /// Start tracking this locator.
    ///
    /// In comparison to [`Track::track`], this method skips this chain link
    /// if it does not contribute anything.
    pub fn track(&self) -> Tracked<'_, Self> {
        match self.outer {
            Some(outer) if self.id.is_none() => outer,
            _ => Track::track(self),
        }
    }
}

#[comemo::track]
impl<'a> Route<'a> {
    /// Whether the given id is part of the route.
    pub fn contains(&self, id: FileId) -> bool {
        self.id == Some(id) || self.outer.map_or(false, |outer| outer.contains(id))
    }
}
