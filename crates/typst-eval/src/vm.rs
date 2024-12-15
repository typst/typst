use comemo::Tracked;
use typst_library::diag::{warning, MaybeDeprecated};
use typst_library::engine::Engine;
use typst_library::foundations::{Context, IntoValue, Scopes, Value};
use typst_library::World;
use typst_syntax::ast::{self, AstNode};
use typst_syntax::Span;

use crate::FlowEvent;

/// A virtual machine.
///
/// Holds the state needed to [evaluate](crate::eval()) Typst sources. A
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
        // This will become an error in the parser if 'is' becomes a keyword.
        if var.get() == "is" {
            self.engine.sink.warn(warning!(
                var.span(),
                "`is` will likely become a keyword in future versions and will \
                not be allowed as an identifier";
                hint: "rename this variable to avoid future errors";
                hint: "try `is_` instead"
            ));
        }
        self.scopes.top.define_ident(var, value);
    }

    /// Imports a variable to the current scope.
    pub fn import(&mut self, ident: ast::Ident, value: MaybeDeprecated<Value>) {
        if self.inspected == Some(ident.span()) {
            self.trace(value.value().clone());
        }
        self.scopes.top.define_ident_maybe_deprecated(ident, value);
    }

    /// Trace a value.
    #[cold]
    pub fn trace(&mut self, value: Value) {
        self.engine
            .sink
            .value(value.clone(), self.context.styles().ok().map(|s| s.to_map()));
    }
}
