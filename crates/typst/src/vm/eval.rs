use comemo::{Track, Tracked};
use typst_macros::Cast;
use typst_syntax::{ast, parse, parse_code, parse_math, Span};

use crate::compiler::{CompileTopLevel, CompiledModule, Compiler};
use crate::diag::SourceResult;
use crate::engine::{Engine, Route};
use crate::foundations::{NativeElement, Scope, Value};
use crate::introspection::{Introspector, Locator};
use crate::math::EquationElem;
use crate::vm::{run_module_as_eval, Tracer};
use crate::World;

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
    let mut engine = Engine {
        world,
        introspector: introspector.track(),
        route: Route::default(),
        locator: &mut locator,
        tracer: tracer.track_mut(),
    };

    let mut compiler = Compiler::module(engine.world.library().clone().into_inner());

    // Compile the code.
    match mode {
        EvalMode::Code => root
            .cast::<ast::Code>()
            .unwrap()
            .compile_top_level(&mut engine, &mut compiler)?,
        EvalMode::Markup => root
            .cast::<ast::Markup>()
            .unwrap()
            .compile_top_level(&mut engine, &mut compiler)?,
        EvalMode::Math => root
            .cast::<ast::Math>()
            .unwrap()
            .compile_top_level(&mut engine, &mut compiler)?,
    }

    let module = CompiledModule::new(compiler.finish_module(root.span(), "eval", vec![]));

    let output = run_module_as_eval(&module, &mut engine, root.span())?;

    Ok(match mode {
        EvalMode::Code => output,
        EvalMode::Markup => Value::Content(output.display()),
        EvalMode::Math => {
            Value::Content(EquationElem::new(output.display()).with_block(false).pack())
        }
    })
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
