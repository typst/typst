use typst_syntax::Source;

use crate::compiler::CompiledModule;
use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Module, Scope, Value};
use crate::vm::ControlFlow;

use super::{State, VMState};

#[typst_macros::time(name = "module eval", span = source.root().span())]
pub fn run_module(
    source: &Source,
    module: &CompiledModule,
    engine: &mut Engine,
) -> SourceResult<Module> {
    // These are required to prove that the registers can be created
    // at compile time safely.
    const NONE: Value = Value::None;

    let mut state = VMState {
        state: State::JOINING | State::DISPLAY,
        output: None,
        global: &module.inner.global,
        instruction_pointer: 0,
        registers: smallvec::smallvec![NONE; module.inner.registers],
        joined: None,
        constants: &module.inner.constants,
        strings: &module.inner.strings,
        labels: &module.inner.labels,
        closures: &module.inner.closures,
        accesses: &module.inner.accesses,
        patterns: &module.inner.patterns,
        spans: &module.inner.isr_spans,
        jumps: &module.inner.jumps,
    };

    // Write all default values.
    for default in &module.inner.defaults {
        state
            .write_one(default.target, default.value.clone())
            .at(module.inner.span)?;
    }

    let output = match crate::vm::run::<std::iter::Empty<Value>>(
        engine,
        &mut state,
        &module.inner.instructions,
        &module.inner.spans,
        None,
    )? {
        ControlFlow::Done(value) => value,
        other => bail!(module.inner.span, "module did not produce a value: {other:?}"),
    };

    let mut scope = Scope::new();
    for export in &module.inner.exports {
        scope.define(export.name.clone(), state.read(export.value).clone());
    }

    Ok(Module::new(module.inner.name.clone(), scope).with_content(output.display()))
}
