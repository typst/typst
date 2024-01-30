use comemo::{Tracked, TrackedMut};

use crate::compiler::CompiledModule;
use crate::diag::{bail, At, SourceResult};
use crate::engine::{Engine, Route};
use crate::eval::Tracer;
use crate::foundations::{Module, Scope, Value};
use crate::introspection::{Introspector, Locator};
use crate::vm::{ControlFlow, VM};
use crate::World;

use super::{State, VMState};

#[comemo::memoize]
#[typst_macros::time(name = "module eval")]
pub fn run_module(
    module: &CompiledModule,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    route: Tracked<Route>,
    locator: Tracked<Locator>,
    tracer: TrackedMut<Tracer>,
) -> SourceResult<Module> {
    // These are required to prove that the registers can be created
    // at compile time safely.
    const SIZE: usize = 256;
    const NONE: Value = Value::None;

    let mut locator = Locator::chained(locator);
    let mut engine = Engine {
        world,
        introspector,
        route: Route::extend(route),
        locator: &mut locator,
        tracer,
    };

    let mut state = VMState {
        state: State::JOINING,
        output: module.inner.output,
        global: &module.inner.global,
        instruction_pointer: 0,
        registers: [NONE; SIZE],
        joined: None,
        constants: &module.inner.constants,
        strings: &module.inner.strings,
        labels: &module.inner.labels,
        closures: &module.inner.closures,
        accesses: &module.inner.accesses,
        patterns: &module.inner.patterns,
        defaults: &module.inner.defaults,
        parent: None,
        iterator: None,
    };

    // Write all default values.
    if let Some(defaults) = module.inner.defaults.get(0) {
        for default in defaults {
            state
                .write_one(default.target, default.value.clone())
                .at(module.inner.span)?;
        }
    }

    let mut vm = VM {
        state,
        span: module.inner.span,
        instructions: &module.inner.instructions,
    };

    let output = match vm.run(&mut engine)? {
        ControlFlow::Done(value) => value,
        _ => bail!(module.inner.span, "module did not produce a value"),
    };

    let mut scope = Scope::new();
    for export in &module.inner.exports {
        scope.define(
            export.name.clone(),
            vm.state.read(export.value).at(export.span)?.clone(),
        );
    }

    Ok(Module::new(module.inner.name.clone(), scope).with_content(output.display()))
}
