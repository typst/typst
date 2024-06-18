//! Evaluation of markup and code.

pub(crate) mod ops;

mod access;
mod binding;
mod call;
mod code;
mod flow;
mod import;
mod markup;
mod math;
mod rules;
mod vm;

pub use self::call::*;
pub use self::import::*;
pub use self::vm::*;

pub(crate) use self::access::*;
pub(crate) use self::binding::*;
pub(crate) use self::flow::*;

use comemo::{Track, Tracked, TrackedMut};

use crate::diag::{bail, SourceResult};
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{Cast, Context, Module, NativeElement, Scope, Scopes, Value};
use crate::introspection::Introspector;
use crate::math::EquationElem;
use crate::syntax::{ast, parse, parse_code, parse_math, Source, Span};
use crate::World;

/// Evaluate a source file and return the resulting module.
#[comemo::memoize]
#[typst_macros::time(name = "eval", span = source.root().span())]
pub fn eval(
    world: Tracked<dyn World + '_>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    source: &Source,
) -> SourceResult<Module> {
    // Prevent cyclic evaluation.
    let id = source.id();
    if route.contains(id) {
        panic!("Tried to cyclicly evaluate {:?}", id.vpath());
    }

    // Prepare the engine.
    let introspector = Introspector::default();
    let engine = Engine {
        world,
        introspector: introspector.track(),
        traced,
        sink,
        route: Route::extend(route).with_id(id),
    };

    // Prepare VM.
    let context = Context::none();
    let scopes = Scopes::new(Some(world.library()));
    let root = source.root();
    let mut vm = Vm::new(engine, context.track(), scopes, root.span());

    // Check for well-formedness unless we are in trace mode.
    let errors = root.errors();
    if !errors.is_empty() && vm.inspected.is_none() {
        return Err(errors.into_iter().map(Into::into).collect());
    }

    // Evaluate the module.
    let markup = root.cast::<ast::Markup>().unwrap();
    let output = markup.eval(&mut vm)?;

    // Handle control flow.
    if let Some(flow) = vm.flow {
        bail!(flow.forbidden());
    }

    // Assemble the module.
    let name = id
        .vpath()
        .as_rootless_path()
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    Ok(Module::new(name, vm.scopes.top).with_content(output).with_file_id(id))
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
    let mut sink = Sink::new();
    let introspector = Introspector::default();
    let traced = Traced::default();
    let engine = Engine {
        world,
        introspector: introspector.track(),
        traced: traced.track(),
        sink: sink.track_mut(),
        route: Route::default(),
    };

    // Prepare VM.
    let context = Context::none();
    let scopes = Scopes::new(Some(world.library()));
    let mut vm = Vm::new(engine, context.track(), scopes, root.span());
    vm.scopes.scopes.push(scope);

    // Evaluate the code.
    let output = match mode {
        EvalMode::Code => root.cast::<ast::Code>().unwrap().eval(&mut vm)?,
        EvalMode::Markup => {
            Value::Content(root.cast::<ast::Markup>().unwrap().eval(&mut vm)?)
        }
        EvalMode::Math => Value::Content(
            EquationElem::new(root.cast::<ast::Math>().unwrap().eval(&mut vm)?)
                .with_block(false)
                .pack(),
        ),
    };

    // Handle control flow.
    if let Some(flow) = vm.flow {
        bail!(flow.forbidden());
    }

    Ok(output)
}

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

/// Evaluate an expression.
pub trait Eval {
    /// The output of evaluating the expression.
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output>;
}
