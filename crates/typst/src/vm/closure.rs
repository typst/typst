use std::borrow::Cow;
use std::sync::Arc;

use comemo::{Tracked, TrackedMut};
use typst_syntax::Span;

use crate::compiler::{CompiledCode, CompiledParam, Remapper};
use crate::diag::{bail, At, SourceResult};
use crate::engine::{Engine, Route};
use crate::foundations::{Args, Func, IntoValue, Value};
use crate::introspection::{Introspector, Locator};
use crate::util::{LazyHash, PicoStr};
use crate::vm::ControlFlow;
use crate::{Library, World};

use super::{Constant, Readable, Register, State, StringId, Tracer, Vm};

/// A closure that has been instantiated.
#[derive(Clone, Hash, PartialEq)]
pub struct Closure {
    pub inner: Arc<LazyHash<Repr>>,
}

#[derive(Hash)]
pub struct Repr {
    /// The compiled code of the closure.
    pub compiled: Arc<LazyHash<CompiledCode>>,
    /// The parameters of the closure.
    params: Vec<(Option<Register>, Param)>,
    /// The captured values and where to store them.
    captures: Vec<(Register, Value)>,
}

impl Closure {
    /// Creates a new closure.
    #[comemo::memoize]
    pub fn new(
        compiled: Arc<LazyHash<CompiledCode>>,
        params: Vec<(Option<Register>, Param)>,
        captures: Vec<(Register, Value)>,
    ) -> Closure {
        Self {
            inner: Arc::new(LazyHash::new(Repr { compiled, params, captures })),
        }
    }

    pub fn no_instance(
        compiled: CompiledCode,
        constants: &Remapper<Constant, Value>,
        library: &Library,
        strings: &Remapper<StringId, Value>,
    ) -> Closure {
        let params = compiled
            .params
            .iter()
            .flat_map(|params| params.iter())
            .map(|param| match param {
                CompiledParam::Pos(output, name) => {
                    (Some(*output), Param::Pos(name.clone()))
                }
                CompiledParam::Named { target, name, default, .. } => {
                    let default = match default {
                        Some(Readable::Reg(_)) => unreachable!(
                            "default should never be a register when in `no_instance`"
                        ),
                        Some(Readable::Auto) => Some(Value::Auto),
                        Some(Readable::None) => Some(Value::None),
                        Some(Readable::Bool(bool_)) => Some(Value::Bool(*bool_)),
                        Some(Readable::Const(const_)) => {
                            Some(constants.get(const_.0 as usize).clone())
                        }
                        Some(Readable::Global(global)) => Some(
                            library
                                .global
                                .field_by_id(global.0 as usize)
                                .unwrap()
                                .clone(),
                        ),
                        Some(Readable::Math(math)) => Some(
                            library.math.field_by_id(math.0 as usize).unwrap().clone(),
                        ),
                        Some(Readable::Str(str_)) => {
                            Some(strings.get(str_.0 as usize).clone())
                        }
                        None => None,
                    };

                    (Some(*target), Param::Named { name: name.clone(), default })
                }
                CompiledParam::Sink(span, dest, name) => {
                    (*dest, Param::Sink(*span, name.clone()))
                }
            })
            .collect();

        Self::new(Arc::new(LazyHash::new(compiled)), params, Vec::new())
    }

    pub fn name(&self) -> Option<&'static str> {
        self.inner.compiled.name.map(|name| name.resolve())
    }

    /// Runs the closure, producing its output.
    pub fn run(&self, engine: &mut Engine, args: Args) -> SourceResult<Value> {
        const NONE: Cow<Value> = Cow::Borrowed(&Value::None);
        if self.inner.compiled.registers <= 4 {
            let mut storage = [NONE; 4];
            self.run_internal(engine, args, &mut storage)
        } else if self.inner.compiled.registers <= 8 {
            let mut storage = [NONE; 8];
            self.run_internal(engine, args, &mut storage)
        } else if self.inner.compiled.registers <= 16 {
            let mut storage = [NONE; 16];
            self.run_internal(engine, args, &mut storage)
        } else if self.inner.compiled.registers <= 32 {
            let mut storage = [NONE; 32];
            self.run_internal(engine, args, &mut storage)
        } else {
            let mut storage = vec![NONE; self.inner.compiled.registers as usize];
            self.run_internal(engine, args, &mut storage)
        }
    }

    fn run_internal<'a>(
        &'a self,
        engine: &mut Engine,
        mut args: Args,
        registers: &mut [Cow<'a, Value>],
    ) -> SourceResult<Value> {
        let num_pos_params = self
            .inner
            .params
            .iter()
            .filter(|(_, p)| matches!(p, Param::Pos(_)))
            .count();

        let num_pos_args = args.to_pos().len();
        let sink_size = num_pos_args.checked_sub(num_pos_params);

        let mut state = Vm {
            output: None,
            state: State::empty(),
            instruction_pointer: 0,
            registers,
            joined: None,
            code: &**self.inner.compiled,
        };

        // Write all default values.
        for default in self.inner.compiled.defaults.iter() {
            state
                .write_borrowed(default.target, &default.value)
                .at(self.inner.compiled.span)?;
        }

        // Write all of the captured values to the registers.
        for (target, value) in &*self.inner.captures {
            state.write_borrowed(*target, &value).at(self.inner.compiled.span)?;
        }

        // Write the self reference to the registers.
        if let Some(self_storage) = self.inner.compiled.self_storage {
            state
                .write_one(self_storage, Value::Func(Func::from(self.clone())))
                .at(self.inner.compiled.span)?;
        }

        // Write all of the arguments to the registers.
        let mut sink = None;
        for (target, arg) in &self.inner.params {
            match arg {
                Param::Pos(name) => {
                    if let Some(target) = target {
                        state
                            .write_one(*target, args.expect::<Value>(*name)?)
                            .at(self.inner.compiled.span)?;
                    }
                }
                Param::Named { name, default } => {
                    if let Some(target) = target {
                        if let Some(value) = args.named::<Value>(*name)? {
                            state
                                .write_one(*target, value)
                                .at(self.inner.compiled.span)?;
                        } else if let Some(default) = default {
                            state
                                .write_borrowed(*target, default)
                                .at(self.inner.compiled.span)?;
                        } else {
                            unreachable!(
                                "named arguments should always have a default value"
                            );
                        }
                    }
                }
                Param::Sink(span, _) => {
                    sink = Some(*target);
                    if let Some(target) = target {
                        let mut arguments = Args::new(*span, std::iter::empty::<Value>());

                        if let Some(sink_size) = sink_size {
                            arguments.extend(args.consume(sink_size)?);
                        }

                        state
                            .write_one(*target, arguments)
                            .at(self.inner.compiled.span)?;
                    } else if let Some(sink_size) = sink_size {
                        args.consume(sink_size)?;
                    }
                }
            }
        }

        if let Some(sink) = sink {
            if let Some(sink) = sink {
                let Value::Args(sink) = state.write(sink) else {
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
            &self.inner.compiled.instructions,
            &self.inner.compiled.spans,
            None,
        )? {
            ControlFlow::Return(value, _) | ControlFlow::Done(value) => Ok(value),
            _ => bail!(self.inner.compiled.span, "closure did not return a value"),
        }
    }
}

impl IntoValue for Closure {
    fn into_value(self) -> Value {
        Value::Func(Func::from(self))
    }
}

#[comemo::memoize]
pub fn call_closure(
    closure: &Closure,
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

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum Param {
    /// A positional parameter.
    Pos(PicoStr),
    /// A named parameter.
    Named {
        /// The name of the parameter.
        name: PicoStr,
        /// The default value of the parameter.
        default: Option<Value>,
    },
    /// A sink parameter.
    Sink(Span, PicoStr),
}
