use comemo::{Tracked, TrackedMut};
use ecow::{EcoString, EcoVec};
use smallvec::{smallvec, SmallVec};
use typst_syntax::Span;

use crate::compile::{
    Call, ClosureParam, CompiledClosure, Executor, ExecutorFlags, Instruction, Pattern, Register, RegisterTable, Value
};
use crate::diag::{bail, SourceResult};
use crate::engine::{Engine, Route};
use crate::eval::Tracer;
use crate::foundations::{Args, Func, IntoValue, Label};
use crate::introspection::{Introspector, Locator};
use crate::World;

use super::{AccessPattern, Capture, LocalId};

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Closure {
    /// The local that will contain the closure.
    pub this: Option<LocalId>,
    /// The span of the closure.
    pub span: Span,
    /// The name of the closure.
    pub name: EcoString,
    /// The output if there is no return statement.
    pub output: Register,
    /// The instructions that make up the closure.
    pub instructions: EcoVec<Instruction>,
    /// The spans of the instructions.
    pub spans: EcoVec<Span>,
    /// The parameters of the closure.
    pub params: EcoVec<IntansiatedClosureParam>,
    /// The calls of the closure.
    pub calls: EcoVec<Call>,
    /// The captured variables.
    pub captures: EcoVec<Value>,
    /// The number of local variables.
    pub locals: usize,
    /// The constants of the closure.
    pub constants: EcoVec<Value>,
    /// The strings of the closure.
    pub strings: EcoVec<EcoString>,
    /// The patterns of the closure.
    pub patterns: EcoVec<Pattern>,
    /// The closures of the closure.
    pub closures: EcoVec<CompiledClosure>,
    /// The labels of the closure.
    pub labels: EcoVec<usize>,
    /// The content labels of the closure.
    pub content_labels: EcoVec<Label>,
    /// The accesses of the closure.
    pub accesses: EcoVec<AccessPattern>,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum IntansiatedClosureParam {
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

impl Closure {
    #[typst_macros::time(name = "closure instantiation", span = compiled_closure.span)]
    pub fn instantiate(
        executor: &Executor<'_>,
        compiled_closure: &CompiledClosure,
    ) -> SourceResult<Self> {
        let captures = compiled_closure
            .captures
            .iter()
            .map(|&captured| match captured {
                Capture::Local { scope, local } => {
                    let Some(value) = executor.local(scope, local) else {
                        bail!(
                            compiled_closure.span,
                            "cannot capture local variable ({}, {})",
                            scope.0,
                            local.0
                        );
                    };

                    Ok(value.clone())
                }
                Capture::Captured { captured } => Ok(executor.captured(captured).clone()),
            })
            .collect::<SourceResult<EcoVec<_>>>()?;

        let params = compiled_closure
            .params
            .iter()
            .map(|param| match param {
                ClosureParam::Pos(name) => IntansiatedClosureParam::Pos(name.clone()),
                ClosureParam::Named { name, default } => IntansiatedClosureParam::Named {
                    name: name.clone(),
                    default: default.map(|def| executor.get(def).clone()),
                },
                ClosureParam::Sink(span, name) => {
                    IntansiatedClosureParam::Sink(*span, name.clone())
                }
            })
            .collect();

        Ok(Self {
            this: compiled_closure.this,
            span: compiled_closure.span,
            name: compiled_closure.name.clone(),
            output: compiled_closure.output,
            instructions: compiled_closure.instructions.clone(),
            spans: compiled_closure.spans.clone(),
            params,
            calls: compiled_closure.calls.clone(),
            captures,
            locals: compiled_closure.locals,
            constants: compiled_closure.constants.clone(),
            strings: compiled_closure.strings.clone(),
            patterns: compiled_closure.patterns.clone(),
            closures: compiled_closure.closures.clone(),
            labels: compiled_closure.labels.clone(),
            content_labels: compiled_closure.content_labels.clone(),
            accesses: compiled_closure.accesses.clone(),
        })
    }

    pub fn call(&self, engine: &mut Engine, mut arguments: Args) -> SourceResult<Value> {
        let num_pos_params = self
            .params
            .iter()
            .filter(|p| matches!(p, IntansiatedClosureParam::Pos(_)))
            .count();
        let num_pos_args = arguments.to_pos().len();
        let sink_size = num_pos_args.checked_sub(num_pos_params);

        let mut params = Vec::new();
        let mut sink = None;
        for arg in self.params.iter() {
            match arg {
                IntansiatedClosureParam::Pos(name) => {
                    params.push(arguments.expect::<Value>(name)?);
                }
                IntansiatedClosureParam::Named { name, default } => {
                    if let Some(value) = arguments.named::<Value>(name)? {
                        params.push(value);
                    } else if let Some(value) = default {
                        params.push(value.clone());
                    } else {
                        unreachable!(
                            "named arguments should always have a default value"
                        );
                    }
                }
                IntansiatedClosureParam::Sink(span, _) => {
                    sink = Some(params.len());
                    let mut args = Args::new(*span, std::iter::empty::<Value>());
                    if let Some(sink_size) = sink_size {
                        args.extend(arguments.consume(sink_size)?);
                    }

                    params.push(args.into_value());
                }
            }
        }

        if let Some(sink) = sink {
            let Value::Args(sink) = params.get_mut(sink).unwrap() else {
                unreachable!("sink should always be an args");
            };

            sink.items.extend(arguments.take().items);
        }

        // Ensure all arguments have been used.
        arguments.finish()?;

        // Call the function in a memoized context to re-use existing (cached)
        // closures.
        let out = memoized(
            self,
            engine.world,
            engine.introspector,
            engine.route.track(),
            engine.locator.track(),
            TrackedMut::reborrow_mut(&mut engine.tracer),
            &params,
        )?;

        Ok(out)
    }
}

#[comemo::memoize]
fn memoized(
    closure: &Closure,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    route: Tracked<Route>,
    locator: Tracked<Locator>,
    tracer: TrackedMut<Tracer>,
    params: &[Value],
) -> SourceResult<Value> {
    // Prepare the engine.
    let mut locator = Locator::chained(locator);
    let mut engine = Engine {
        world,
        introspector,
        route: Route::extend(route),
        locator: &mut locator,
        tracer,
    };

    // Create the executor
    let mut executor = Executor {
        state: ExecutorFlags::NONE,
        output: closure.output,
        registers: RegisterTable::default(),
        locals: smallvec![Value::None; closure.locals],
        scope_stack: SmallVec::new(),
        base: Some(world.library()),
        instructions: &closure.instructions,
        labels: &closure.labels,
        calls: &closure.calls,
        constants: &closure.constants,
        arguments: &params,
        closures: &closure.closures,
        strings: &closure.strings,
        captured: &closure.captures,
        content_labels: &closure.content_labels,
        patterns: &closure.patterns,
        join_contexts: SmallVec::new(),
        spans: &closure.spans,
        iterators: smallvec![],
        accesses: &closure.accesses,
    };

    // Define the closure inside of itself if needed.
    if let Some(this) = closure.this {
        executor.locals[this.0 as usize] = Value::Func(Func::from(closure.clone()));
    }

    executor.eval(&mut engine)
}
