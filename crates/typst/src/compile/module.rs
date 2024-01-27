use comemo::{Track, Tracked, TrackedMut};
use ecow::{EcoString, EcoVec};
use smallvec::{smallvec, SmallVec};
use typst_syntax::{ast, Source, Span};

use crate::compile::{
    Call, Compile, CompiledClosure, Executor, ExecutorFlags, Instruction, Pattern, Register, RegisterTable
};
use crate::diag::SourceResult;
use crate::engine::{Engine, Route};
use crate::eval::Tracer;
use crate::foundations::{Label, Module, Scope, Value};
use crate::introspection::{Introspector, Locator};
use crate::World;

use super::AccessPattern;

#[derive(Clone, Hash)]
pub struct CompiledModule {
    /// The name of the module.
    pub name: EcoString,
    /// The top level scope (used to resolve names)
    pub scope: Scope,
    /// The output if there is no return statement.
    pub output: Register,
    /// The instructions that make up the module.
    pub instructions: EcoVec<Instruction>,
    /// The spans of the instructions.
    pub spans: EcoVec<Span>,
    /// The calls of the module.
    pub calls: EcoVec<Call>,
    /// The number of local variables.
    pub locals: usize,
    /// The constants of the module.
    pub constants: EcoVec<Value>,
    /// The strings of the module.
    pub strings: EcoVec<EcoString>,
    /// The patterns of the module.
    pub patterns: EcoVec<Pattern>,
    /// The closures of the module.
    pub closures: EcoVec<CompiledClosure>,
    /// The labels of the module.
    pub labels: EcoVec<usize>,
    /// The content labels of the module.
    pub content_labels: EcoVec<Label>,
    /// The access patterns in the module.
    pub accesses: EcoVec<AccessPattern>,
}

impl CompiledModule {
    #[typst_macros::time(name = "module eval", span = self.spans[0])]
    pub fn eval(&self, engine: &mut Engine) -> SourceResult<Module> {
        #[comemo::memoize]
        fn memoized(
            module: &CompiledModule,
            world: Tracked<dyn World + '_>,
            introspector: Tracked<Introspector>,
            route: Tracked<Route>,
            locator: Tracked<Locator>,
            tracer: TrackedMut<Tracer>,
        ) -> SourceResult<Module> {
            // Prepare the engine.
            let mut locator = Locator::chained(locator);
            let mut engine = Engine {
                world,
                introspector,
                route: Route::extend(route),
                locator: &mut locator,
                tracer,
            };

            // We instantiate the executor.
            let mut executor = Executor {
                state: ExecutorFlags::NONE,
                output: module.output,
                registers: RegisterTable::default(),
                locals: smallvec![Value::None; module.locals],
                scope_stack: SmallVec::new(),
                base: Some(world.library()),
                instructions: &module.instructions,
                labels: &module.labels,
                calls: &module.calls,
                constants: &module.constants,
                arguments: &[],
                closures: &module.closures,
                strings: &module.strings,
                captured: &[],
                content_labels: &module.content_labels,
                join_contexts: SmallVec::new(),
                spans: &module.spans,
                iterators: smallvec![],
                patterns: &module.patterns,
                accesses: &module.accesses,
            };

            // We eval the module.
            let output = executor.eval(&mut engine)?;

            // We build the scope by using the current scope and getting
            // all of the locals from the executor.
            let mut scope = module.scope.clone();
            for i in 0..module.locals {
                let value = executor.locals[i].clone();
                *scope.get_mut_by_id(i).unwrap() = value;
            }

            // Build the module.
            let mut module = Module::new(module.name.clone(), scope);
            if output != Value::None {
                module = module.with_content(output.display());
            }

            Ok(module)
        }

        memoized(
            self,
            engine.world,
            engine.introspector,
            engine.route.track(),
            engine.locator.track(),
            TrackedMut::reborrow_mut(&mut engine.tracer),
        )
    }
}

/// Evaluate a source file and return the resulting module.
#[comemo::memoize]
#[typst_macros::time(name = "eval", span = source.root().span())]
pub fn eval(
    world: Tracked<dyn World + '_>,
    route: Tracked<Route>,
    tracer: TrackedMut<Tracer>,
    source: &Source,
) -> SourceResult<Module> {
    // Prevent cyclic evaluation.
    let id = source.id();
    if route.contains(id) {
        panic!("Tried to cyclicly evaluate {:?}", id.vpath());
    }

    // Prepare the engine.
    let mut locator = Locator::new();
    let introspector = Introspector::default();
    let mut engine = Engine {
        world,
        route: Route::extend(route).with_id(id),
        introspector: introspector.track(),
        locator: &mut locator,
        tracer,
    };

    // Prepare VM.
    let root = source.root();

    // Check for well-formedness unless we are in trace mode.
    let errors = root.errors();
    // TODO: handle inspection.
    if !errors.is_empty() && true {
        return Err(errors.into_iter().map(Into::into).collect());
    }

    // Get the name.
    let name = id
        .vpath()
        .as_rootless_path()
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    // Evaluate the module.
    let markup = root.cast::<ast::Markup>().unwrap();
    let library = world.library().clone().into_inner();
    let output = markup.compile_all(&mut engine, name, library)?;
    // eprintln!("{:#?}", output.instructions);

    let module = output.eval(&mut engine)?;
    Ok(module)
}
