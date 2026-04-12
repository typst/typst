//! Typst's code interpreter.

pub(crate) mod ops;

mod access;
mod binding;
mod call;
mod code;
mod flow;
mod import;
mod markup;
mod math;
mod methods;
mod rules;
mod vm;

pub use self::call::{CapturesVisitor, eval_closure};
pub use self::flow::FlowEvent;
pub use self::import::import;
pub use self::vm::{Vm, hint_if_shadowed_std};

use self::access::*;
use self::binding::*;
use self::methods::*;

use comemo::{Track, Tracked, TrackedMut};
use typst_library::World;
use typst_library::diag::{SourceResult, bail};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Context, Module, NativeElement, Scope, Scopes, Value};
use typst_library::introspection::{EmptyIntrospector, Introspector};
use typst_library::math::EquationElem;
use typst_library::routines::Routines;
use typst_syntax::{Source, Span, SyntaxMode, ast, parse, parse_code, parse_math};
use typst_utils::Protected;

/// Evaluate a source file and return the resulting module.
#[comemo::memoize]
#[typst_macros::time(name = "eval", span = source.root().span())]
pub fn eval(
    routines: &Routines,
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
    let introspector = EmptyIntrospector;
    let engine = Engine {
        routines,
        world,
        introspector: Protected::new(introspector.track()),
        traced,
        sink,
        route: Route::extend(route).with_id(id),
    };

    // Prepare VM.
    let context = Context::none();
    let scopes = Scopes::new(Some(world.library()));
    let root = source.root();
    let mut vm = Vm::new(engine, context.track(), scopes, root.span());

    // Check for errors or warnings in the syntax tree before evaluating it.
    // However, if we're inspecting a span, we keep going with evaluation
    // regardless of syntax errors (although we will still add warnings).
    let errors_and_warnings = root.errors_and_warnings().into_iter();
    if root.erroneous().errors && vm.inspected.is_none() {
        // Returning errors and warnings together should preserve ordering.
        return Err(errors_and_warnings.map(Into::into).collect());
    }
    // Filtering is necessary since we may have syntax errors that we're
    // ignoring.
    for warning in errors_and_warnings.filter(|diag| !diag.is_error) {
        vm.engine.sink.warn(warning.into());
    }

    // Evaluate the module.
    let markup = root.cast::<ast::Markup>().unwrap();
    let output = markup.eval(&mut vm)?;

    // Handle control flow.
    if let Some(flow) = vm.flow {
        bail!(flow.forbidden());
    }

    // Assemble the module.
    let name = id.vpath().file_stem().unwrap_or_default();

    Ok(Module::new(name, vm.scopes.top).with_content(output).with_file_id(id))
}

/// Evaluate a string as code and return the resulting value.
///
/// Everything in the output is associated with the given `span`.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
pub fn eval_string(
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    mut sink: TrackedMut<Sink>,
    introspector: Tracked<dyn Introspector + '_>,
    context: Tracked<Context>,
    string: &str,
    span: Span,
    mode: SyntaxMode,
    scope: Scope,
) -> SourceResult<Value> {
    let mut root = match mode {
        SyntaxMode::Code => parse_code(string),
        SyntaxMode::Markup => parse(string),
        SyntaxMode::Math => parse_math(string),
    };

    root.synthesize(span);

    // Check for errors or warnings in the syntax tree before evaluating it.
    let errors_and_warnings = root.errors_and_warnings();
    if root.erroneous().errors {
        // Returning errors and warnings together should preserve ordering.
        return Err(errors_and_warnings.into_iter().map(Into::into).collect());
    }
    for diag in errors_and_warnings {
        assert!(!diag.is_error, "we just checked for syntax errors");
        sink.warn(diag.into());
    }

    // Prepare the engine.
    let traced = Traced::default();
    let engine = Engine {
        routines,
        world,
        introspector: Protected::new(introspector),
        traced: traced.track(),
        sink,
        route: Route::default(),
    };

    // Prepare VM.
    let scopes = Scopes::new(Some(world.library()));
    let mut vm = Vm::new(engine, context, scopes, span);
    vm.scopes.scopes.push(scope);

    // Evaluate the code.
    let output = match mode {
        SyntaxMode::Code => root.cast::<ast::Code>().unwrap().eval(&mut vm)?,
        SyntaxMode::Markup => {
            Value::Content(root.cast::<ast::Markup>().unwrap().eval(&mut vm)?)
        }
        SyntaxMode::Math => Value::Content(
            EquationElem::new(root.cast::<ast::Math>().unwrap().eval(&mut vm)?)
                .with_block(false)
                .pack()
                .spanned(span),
        ),
    };

    // Handle control flow.
    if let Some(flow) = vm.flow {
        bail!(flow.forbidden());
    }

    Ok(output)
}

/// Evaluate an expression.
pub trait Eval {
    /// The output of evaluating the expression.
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output>;
}
