use comemo::{Track, Tracked};
use typst_macros::Cast;
use typst_syntax::{ast, parse, parse_code, parse_math, Span};

use crate::diag::SourceResult;
use crate::engine::{Engine, Route};
use crate::foundations::{Scope, Value};
use crate::introspection::{Introspector, Locator};
use crate::vm::Tracer;
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
    let engine = Engine {
        world,
        introspector: introspector.track(),
        route: Route::default(),
        locator: &mut locator,
        tracer: tracer.track_mut(),
    };

    // Prepare VM.
    /*let scopes = Scopes::new(Some(world.library()));
    let mut vm = Vm::new(engine, scopes, root.span());
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

    Ok(output)*/

    todo!()
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
