pub mod closure;
pub mod compiled;
pub mod compiler;
pub mod interpreter;
pub mod module;
pub mod opcodes;
pub mod operands;
pub mod ops;
mod tracer;

use std::borrow::Cow;

use closure::Param;
use comemo::{Track, Tracked, TrackedMut};
use compiled::{CompiledModule, Export};
use compiler::{CompileTopLevel, Compiler};
use ecow::EcoString;
use interpreter::{run, ControlFlow, Vm};
use typst_macros::Cast;
use typst_syntax::{ast, parse, parse_code, parse_math, Source, Span};
use typst_utils::LazyHash;

use crate::diag::{bail, At, SourceResult};
use crate::engine::{Engine, Route};
use crate::foundations::{Args, Context, Func, Module, NativeElement, Scope, Value};
use crate::introspection::{Introspector, Locator};
use crate::math::EquationElem;
use crate::{Library, World};

use self::closure::Closure;
pub use self::tracer::Tracer;

/// In which mode to evaluate a string.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum EvalMode {
    /// Evaluate as code, as after a hash.
    Code,
    /// Evaluate as markup, like in a Typst file.
    Markup,
    /// Evaluate as math, as in an equation.
    Math,
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
    let library = world.library();
    let introspector = Introspector::default();
    let mut engine = Engine {
        world,
        route: Route::extend(route).with_id(id),
        introspector: introspector.track(),
        locator: &mut locator,
        tracer,
    };

    // Compile the module
    let compiled = compile_module(source, library, &mut engine)?;

    // Evaluate the module
    let context = Context::none();
    run_module(source, &compiled, &mut engine, context.track(), true)
}

/// Evaluate a string as code and return the resulting value.
///
/// Everything in the output is associated with the given `span`.
#[comemo::memoize]
pub fn eval_string(
    world: Tracked<dyn World + '_>,
    string: &str,
    span: Span,
    mode: EvalMode,
    scope: Scope,
) -> SourceResult<Value> {
    let mut root = match mode {
        EvalMode::Code => parse_code(string),
        EvalMode::Markup => parse(string),
        EvalMode::Math => parse_math(string),
    };

    root.synthesize(span);

    // Check for well-formedness.
    let errors = root.errors();
    if !errors.is_empty() {
        return Err(errors.into_iter().map(Into::into).collect());
    }

    // Prepare the engine.
    let mut tracer = Tracer::new();
    let mut locator = Locator::new();
    let introspector = Introspector::default();
    let library = world.library();
    let mut engine = Engine {
        world,
        introspector: introspector.track(),
        route: Route::default(),
        locator: &mut locator,
        tracer: tracer.track_mut(),
    };

    let mut compiler = Compiler::new_module(library);

    for (name, value) in scope.iter() {
        compiler.declare_default(root.span(), name.as_str(), value.clone());
    }

    // Compile the code.
    match mode {
        EvalMode::Code => root
            .cast::<ast::Code>()
            .unwrap()
            .compile_top_level(&mut compiler, &mut engine)?,
        EvalMode::Markup => root
            .cast::<ast::Markup>()
            .unwrap()
            .compile_top_level(&mut compiler, &mut engine)?,
        EvalMode::Math => root
            .cast::<ast::Math>()
            .unwrap()
            .compile_top_level(&mut compiler, &mut engine)?,
    }

    let module = CompiledModule::new(compiler.finish_module(root.span(), "eval", vec![]));

    let context = Context::none();
    let output = run_module_as_eval(&module, &mut engine, context.track(), root.span(), matches!(mode, EvalMode::Markup | EvalMode::Math))?;

    Ok(match mode {
        EvalMode::Code => output,
        EvalMode::Markup => Value::Content(output.display()),
        EvalMode::Math => {
            Value::Content(EquationElem::new(output.display()).with_block(false).pack())
        }
    })
}

/// Call the function in the context with the arguments.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
pub(crate) fn call_closure(
    func: &Func,
    closure: &LazyHash<Closure>,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    route: Tracked<Route>,
    locator: Tracked<Locator>,
    tracer: TrackedMut<Tracer>,
    context: Tracked<Context>,
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

    run_closure(func, closure, &mut engine, context, args)
}

#[typst_macros::time(name = "module eval", span = source.root().span())]
pub fn run_module(
    source: &Source,
    module: &CompiledModule,
    engine: &mut Engine,
    context: Tracked<Context>,
    display: bool,
) -> SourceResult<Module> {
    const NONE: Cow<Value> = Cow::Borrowed(&Value::None);
    let (output, scope) = if module.inner.registers <= 4 {
        let mut storage = [NONE; 4];
        run_module_internal(module, engine, context, &mut storage, true, display)?
    } else if module.inner.registers <= 8 {
        let mut storage = [NONE; 8];
        run_module_internal(module, engine, context, &mut storage, true, display)?
    } else if module.inner.registers <= 16 {
        let mut storage = [NONE; 16];
        run_module_internal(module, engine, context, &mut storage, true, display)?
    } else if module.inner.registers <= 32 {
        let mut storage = [NONE; 32];
        run_module_internal(module, engine, context, &mut storage, true, display)?
    } else {
        let mut storage = vec![NONE; module.inner.registers as usize];
        run_module_internal(module, engine, context, &mut storage, true, display)?
    };

    let name = module.inner.name.clone().unwrap_or(EcoString::inline("anonymous"));
    Ok(Module::new(name, scope.unwrap()).with_content(output.display()))
}

#[typst_macros::time(name = "eval", span = span)]
pub fn run_module_as_eval(
    module: &CompiledModule,
    engine: &mut Engine,
    context: Tracked<Context>,
    span: Span,
    display: bool,
) -> SourceResult<Value> {
    const NONE: Cow<Value> = Cow::Borrowed(&Value::None);
    let (output, _) = if module.inner.registers <= 4 {
        let mut storage = [NONE; 4];
        run_module_internal(module, engine, context, &mut storage, false, display)?
    } else if module.inner.registers <= 8 {
        let mut storage = [NONE; 8];
        run_module_internal(module, engine, context, &mut storage, false, display)?
    } else if module.inner.registers <= 16 {
        let mut storage = [NONE; 16];
        run_module_internal(module, engine, context, &mut storage, false, display)?
    } else if module.inner.registers <= 32 {
        let mut storage = [NONE; 32];
        run_module_internal(module, engine, context, &mut storage, false, display)?
    } else {
        let mut storage = vec![NONE; module.inner.registers as usize];
        run_module_internal(module, engine, context, &mut storage, false, display)?
    };

    Ok(output)
}

fn run_module_internal<'a, 'b>(
    module: &'a CompiledModule,
    engine: &mut Engine,
    context: Tracked<'a, Context<'a>>,
    registers: &'a mut [Cow<'b, Value>],
    scope: bool,
    display: bool,
) -> SourceResult<(Value, Option<Scope>)>
where
    'a: 'b,
{
    let mut vm = Vm::new(registers, &**module.inner, context).with_display(display);

    // Write all default values.
    for default in &*module.inner.defaults {
        vm.write_borrowed(default.target, &default.value);
    }

    let output = match run(
        engine,
        &mut vm,
        &module.inner.instructions,
        &module.inner.spans,
        None,
    )? {
        ControlFlow::Done(value) => value,
        other => bail!(module.inner.span, "module did not produce a value: {other:?}"),
    };

    let scope = scope.then(|| {
        let mut scope = Scope::new();
        for export in module.inner.exports.iter().flat_map(|e| e.iter()) {
            scope.define(export.name.resolve(), vm.read(export.value).clone());
        }
        scope
    });

    Ok((output, scope))
}

#[typst_macros::time(name = "module compile", span = source.root().span())]
pub fn compile_module(
    source: &Source,
    library: &Library,
    engine: &mut Engine,
) -> SourceResult<CompiledModule> {
    // Parse the source.
    let root = source.root();

    // Check for well-formedness unless we are in trace mode.
    let errors = root.errors();
    if !errors.is_empty() {
        return Err(errors.into_iter().map(Into::into).collect());
    }

    // Evaluate the module.
    let markup = root.cast::<ast::Markup>().unwrap();

    // Assemble the module.
    let name = source
        .id()
        .vpath()
        .as_rootless_path()
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    // Prepare Compiler.
    let mut compiler = Compiler::new_module(library);

    // Compile the module.
    markup.compile_top_level(&mut compiler, engine)?;

    let scopes = compiler.scope.borrow();
    let exports = scopes
        .variables
        .iter()
        .map(|(name, var)| Export {
            name: name.as_str().into(),
            value: var.register.as_readable(),
            span: var.span,
        })
        .collect();

    drop(scopes);
    Ok(CompiledModule::new(compiler.finish_module(root.span(), &*name, exports)))
}

fn run_closure(
    func: &Func,
    closure: &Closure,
    engine: &mut Engine,
    context: Tracked<Context>,
    args: Args,
) -> SourceResult<Value> {
    const NONE: Cow<Value> = Cow::Borrowed(&Value::None);
    if closure.inner.compiled.registers <= 4 {
        let mut storage = [NONE; 4];
        run_closure_internal(func, closure, engine, context, args, &mut storage)
    } else if closure.inner.compiled.registers <= 8 {
        let mut storage = [NONE; 8];
        run_closure_internal(func, closure, engine, context, args, &mut storage)
    } else if closure.inner.compiled.registers <= 16 {
        let mut storage = [NONE; 16];
        run_closure_internal(func, closure, engine, context, args, &mut storage)
    } else if closure.inner.compiled.registers <= 32 {
        let mut storage = [NONE; 32];
        run_closure_internal(func, closure, engine, context, args, &mut storage)
    } else {
        let mut storage = vec![NONE; closure.inner.compiled.registers as usize];
        run_closure_internal(func, closure, engine, context, args, &mut storage)
    }
}

fn run_closure_internal<'a, 'b>(
    func: &Func,
    closure: &'b Closure,
    engine: &mut Engine,
    context: Tracked<'a, Context<'a>>,
    mut args: Args,
    registers: &'b mut [Cow<'a, Value>],
) -> SourceResult<Value>
where
    'b: 'a,
{
    let num_pos_params = closure
        .inner
        .params
        .iter()
        .filter(|(_, p)| matches!(p, Param::Pos(_)))
        .count();

    let inner = &**closure.inner;
    let compiled = &**closure.inner.compiled;
    let num_pos_args = args.to_pos().len();
    let sink_size = num_pos_args.checked_sub(num_pos_params);

    let mut vm = Vm::new(registers, compiled, context);

    // Write all default values.
    for default in compiled.defaults.iter() {
        vm.write_borrowed(default.target, &default.value);
    }

    // Write all of the captured values to the registers.
    for (target, value) in &*inner.captures {
        vm.write_borrowed(*target, &value);
    }

    // Write the self reference to the registers.
    if let Some(self_storage) = compiled.self_storage {
        vm.write_one(self_storage, Value::Func(func.clone()))
            .at(compiled.span)?;
    }

    // Write all of the arguments to the registers.
    let mut sink = None;
    for (target, arg) in &inner.params {
        match arg {
            Param::Pos(name) => {
                if let Some(target) = target {
                    vm.write_one(*target, args.expect::<Value>(name)?)
                        .at(compiled.span)?;
                }
            }
            Param::Named { name, default } => {
                if let Some(target) = target {
                    if let Some(value) = args.named::<Value>(name)? {
                        vm.write_one(*target, value).at(compiled.span)?;
                    } else if let Some(default) = default {
                        vm.write_borrowed(*target, default);
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

                    vm.write_one(*target, arguments).at(compiled.span)?;
                } else if let Some(sink_size) = sink_size {
                    args.consume(sink_size)?;
                }
            }
        }
    }

    if let Some(sink) = sink {
        if let Some(sink) = sink {
            let Some(Value::Args(sink)) = vm.write(sink) else {
                unreachable!("sink should always be an args");
            };

            sink.items.extend(args.take().items);
        } else {
            args.take();
        }
    }

    // Ensure all arguments have been used.
    args.finish()?;

    match run(engine, &mut vm, &compiled.instructions, &compiled.spans, None)? {
        ControlFlow::Return(value, _) | ControlFlow::Done(value) => Ok(value),
        _ => bail!(compiled.span, "closure did not return a value"),
    }
}
