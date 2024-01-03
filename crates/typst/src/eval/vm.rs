use comemo::Tracked;

use crate::engine::Engine;
use crate::eval::FlowEvent;
use crate::foundations::{IntoValue, Scopes};
use crate::syntax::ast::{self, AstNode};
use crate::syntax::Span;
use crate::World;

/// A virtual machine.
///
/// Holds the state needed to [evaluate](crate::eval::eval()) Typst sources. A
/// new virtual machine is created for each module evaluation and function call.
pub struct Vm<'a> {
    /// The underlying virtual typesetter.
    pub(crate) engine: Engine<'a>,
    /// A control flow event that is currently happening.
    pub(crate) flow: Option<FlowEvent>,
    /// The stack of scopes.
    pub(crate) scopes: Scopes<'a>,
    /// A span that is currently under inspection.
    pub(crate) inspected: Option<Span>,
}

impl<'a> Vm<'a> {
    /// Create a new virtual machine.
    pub fn new(engine: Engine<'a>, scopes: Scopes<'a>, target: Span) -> Self {
        let inspected = target.id().and_then(|id| engine.tracer.inspected(id));
        Self { engine, flow: None, scopes, inspected }
    }

    /// Access the underlying world.
    pub fn world(&self) -> Tracked<'a, dyn World + 'a> {
        self.engine.world
    }

    /// Define a variable in the current scope.
    pub fn define(&mut self, var: ast::Ident, value: impl IntoValue) {
        let value = value.into_value();
        if self.inspected == Some(var.span()) {
            self.engine.tracer.value(value.clone());
        }
        self.scopes.top.define(var.get().clone(), value);
    }
}
