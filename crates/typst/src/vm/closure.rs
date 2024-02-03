use std::sync::Arc;

use comemo::{Prehashed, Tracked, TrackedMut};
use ecow::{EcoString, EcoVec};
use typst_syntax::Span;

use crate::diag::{bail, At, SourceResult};
use crate::engine::{Engine, Route};
use crate::eval::Tracer;
use crate::foundations::{Args, Func, IntoValue, Label, Value};
use crate::introspection::{Introspector, Locator};
use crate::vm::ControlFlow;
use crate::{Library, World};

use super::opcodes::Opcode;
use super::{
    Access, OptionalWritable, Pattern, Readable, Register, State, VMState, Writable,
};

/// A closure that has been instantiated.
#[derive(Clone, Hash)]
pub struct Closure {
    pub inner: Arc<Inner>,
    /// The parameters of the closure.
    pub params: EcoVec<(OptionalWritable, Param)>,
    /// The captured values and where to store them.
    pub captures: EcoVec<(Writable, Value)>,
    /// Where to store the reference to the closure itself.
    pub self_storage: Option<Writable>,
}

impl Closure {
    /// Creates a new closure.
    pub fn new(
        inner: Arc<Inner>,
        params: EcoVec<(OptionalWritable, Param)>,
        captures: EcoVec<(Writable, Value)>,
        self_storage: Option<Writable>,
    ) -> Self {
        Self { inner, params, captures, self_storage }
    }

    pub fn name(&self) -> Option<&str> {
        self.inner.name.as_deref()
    }

    /// Runs the closure, producing its output.
    pub fn run(&self, engine: &mut Engine, mut args: Args) -> SourceResult<Value> {
        // These are required to prove that the registers can be created
        // at compile time safely.
        const NONE: Value = Value::None;

        let num_pos_params =
            self.params.iter().filter(|(_, p)| matches!(p, Param::Pos(_))).count();

        let num_pos_args = args.to_pos().len();
        let sink_size = num_pos_args.checked_sub(num_pos_params);

        let mut state = VMState {
            state: if self.inner.joined { State::JOINING } else { State::empty() },
            output: self.inner.output,
            global: &self.inner.global,
            instruction_pointer: 0,
            registers: vec![NONE; self.inner.registers],
            joined: None,
            constants: &self.inner.constants,
            strings: &self.inner.strings,
            labels: &self.inner.labels,
            closures: &self.inner.closures,
            accesses: &self.inner.accesses,
            patterns: &self.inner.patterns,
            spans: &self.inner.isr_spans,
            jumps: &self.inner.jumps,
            iterator: None,
        };

        // Write all default values.
        for default in &self.inner.defaults {
            state
                .write_one(default.target, default.value.clone())
                .at(self.inner.span)?;
        }

        // Write all of the captured values to the registers.
        for (target, value) in &self.captures {
            state.write_one(*target, value.clone()).at(self.inner.span)?;
        }

        // Write the self reference to the registers.
        if let Some(self_storage) = self.self_storage {
            state
                .write_one(self_storage, Value::Func(Func::from(self.clone())))
                .at(self.inner.span)?;
        }

        // Write all of the arguments to the registers.
        let mut sink = None;
        for (target, arg) in &self.params {
            match arg {
                Param::Pos(name) => {
                    if let Some(target) = target.ok() {
                        state
                            .write_one(target, args.expect::<Value>(name)?)
                            .at(self.inner.span)?;
                    }
                }
                Param::Named { name, default } => {
                    if let Some(target) = target.ok() {
                        if let Some(value) = args.named::<Value>(name)? {
                            state.write_one(target, value).at(self.inner.span)?;
                        } else if let Some(default) = default {
                            state
                                .write_one(target, default.clone())
                                .at(self.inner.span)?;
                        } else {
                            unreachable!(
                                "named arguments should always have a default value"
                            );
                        }
                    }
                }
                Param::Sink(span, _) => {
                    sink = Some(*target);
                    let mut arguments = Args::new(*span, std::iter::empty::<Value>());
                    if let Some(sink_size) = sink_size {
                        arguments.extend(args.consume(sink_size)?);
                    }

                    if let Some(target) = target.ok() {
                        state.write_one(target, arguments).at(self.inner.span)?;
                    }
                }
            }
        }

        if let Some(sink) = sink {
            if let Some(sink) = sink.ok() {
                let Value::Args(sink) = state.write(sink).unwrap() else {
                    unreachable!("sink should always be an args");
                };

                sink.items.extend(args.take().items);
            } else {
                args.take();
            }
        }

        // Ensure all arguments have been used.
        args.finish()?;

        match crate::vm::run(
            engine,
            &mut state,
            &self.inner.instructions,
            &self.inner.spans,
            self.inner.span,
        )? {
            ControlFlow::Return(value, _) | ControlFlow::Done(value) => Ok(value),
            _ => bail!(self.inner.span, "closure did not return a value"),
        }
    }
}

impl IntoValue for Closure {
    fn into_value(self) -> Value {
        Value::Func(Func::from(self))
    }
}

#[comemo::memoize]
#[typst_macros::time(name = "call closure", span = closure.inner.span)]
pub fn call_closure(
    closure: &Prehashed<Closure>,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    route: Tracked<Route>,
    locator: Tracked<Locator>,
    tracer: TrackedMut<Tracer>,
    args: Args,
) -> SourceResult<Value> {
    let mut locator = Locator::chained(locator);
    let mut engine = Engine {
        world,
        introspector,
        route: Route::extend(route),
        locator: &mut locator,
        tracer,
    };

    closure.run(&mut engine, args)
}

/// A closure that has been compiled but is not yet instantiated.
#[derive(Clone, Hash)]
pub struct CompiledClosure {
    /// The common data.
    pub inner: Arc<Inner>,
    /// The captures of the closure.
    pub captures: Vec<Capture>,
    /// The parameters of the closure.
    pub params: Vec<CompiledParam>,
    /// Where to store the reference to the closure itself.
    pub self_storage: Option<Writable>,
}

#[derive(Clone, Hash)]
pub struct Inner {
    /// The name of the closure.
    pub name: Option<EcoString>,
    /// The span where the closure was defined.
    pub span: Span,
    /// The number of registers needed for the closure.
    pub registers: usize,
    /// The instructions as byte code.
    pub instructions: Vec<Opcode>,
    /// The spans of the instructions.
    pub spans: Vec<Span>,
    /// The global library.
    pub global: Library,
    /// The list of constants.
    pub constants: Vec<Value>,
    /// The list of strings.
    pub strings: Vec<Value>,
    /// The list of closures.
    pub closures: Vec<CompiledClosure>,
    /// The accesses.
    pub accesses: Vec<Access>,
    /// The list of labels.
    pub labels: Vec<Label>,
    /// The list of patterns.
    pub patterns: Vec<Pattern>,
    /// The default values of variables.
    pub defaults: EcoVec<DefaultValue>,
    /// The spans used in the closure.
    pub isr_spans: Vec<Span>,
    /// The jumps used in the closure.
    pub jumps: Vec<usize>,
    /// The output value (if any).
    pub output: Option<Readable>,
    /// Whether this closure returns a joined value.
    pub joined: bool,
}

#[derive(Debug, Clone, Hash)]
pub struct Capture {
    /// The name of the value to capture.
    pub name: EcoString,
    /// The value of the capture **in the parent scope**.
    pub value: Readable,
    /// Where the value is stored **in the closure's scope**.
    pub location: Writable,
    /// The span where the capture was occurs.
    pub span: Span,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum Param {
    /// A positional parameter.
    Pos(EcoString),
    /// A named parameter.
    Named {
        /// The name of the parameter.
        name: EcoString,
        /// The default value of the parameter.
        default: Option<Value>,
    },
    /// A sink parameter.
    Sink(Span, EcoString),
}

#[derive(Clone, Hash)]
pub enum CompiledParam {
    /// A positional parameter.
    Pos(Writable, EcoString),
    /// A named parameter.
    Named {
        /// The span of the parameter.
        span: Span,
        /// The location where the parameter will be stored.
        target: Writable,
        /// The name of the parameter.
        name: EcoString,
        /// The default value of the parameter.
        default: Option<Readable>,
    },
    /// A sink parameter.
    Sink(Span, OptionalWritable, EcoString),
}

#[derive(Clone, Hash)]
pub struct DefaultValue {
    /// The value of the default.
    pub value: Value,
    /// The target where the default value will be stored.
    pub target: Register,
}
