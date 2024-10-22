use comemo::Tracked;

use crate::engine::Engine;
use crate::eval::FlowEvent;
use crate::foundations::{Context, IntoValue, Scopes, Value};
use crate::syntax::ast::{self, AstNode};
use crate::syntax::Span;
use crate::World;

/// A virtual machine.
///
/// Holds the state needed to [evaluate](crate::eval::eval()) Typst sources. A
/// new virtual machine is created for each module evaluation and function call.
pub struct Vm<'a> {
    /// The underlying virtual typesetter.
    pub engine: Engine<'a>,
    /// A control flow event that is currently happening.
    pub flow: Option<FlowEvent>,
    /// The stack of scopes.
    pub scopes: Scopes<'a>,
    /// A span that is currently under inspection.
    pub inspected: Option<Span>,
    /// Data that is contextually made accessible to code behind the scenes.
    pub context: Tracked<'a, Context<'a>>,
}

impl<'a> Vm<'a> {
    /// Create a new virtual machine.
    pub fn new(
        engine: Engine<'a>,
        context: Tracked<'a, Context<'a>>,
        scopes: Scopes<'a>,
        target: Span,
    ) -> Self {
        let inspected = target.id().and_then(|id| engine.traced.get(id));
        Self { engine, context, flow: None, scopes, inspected }
    }

    /// Access the underlying world.
    pub fn world(&self) -> Tracked<'a, dyn World + 'a> {
        self.engine.world
    }

    /// Define a variable in the current scope.
    pub fn define(&mut self, var: ast::Ident, value: impl IntoValue) {
        let value = value.into_value();
        if self.inspected == Some(var.span()) {
            self.trace(value.clone());
        }
        self.scopes.top.define_ident(var, value);
    }

    /// Trace a value.
    #[cold]
    pub fn trace(&mut self, value: Value) {
        self.engine
            .sink
            .value(value.clone(), self.context.styles().ok().map(|s| s.to_map()));
    }
}
