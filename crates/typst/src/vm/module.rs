use std::borrow::Cow;

use typst_syntax::{Source, Span};

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
    const NONE: Cow<Value> = Cow::Borrowed(&Value::None);
    let (output, scope) = if module.inner.registers <= 4 {
        let mut storage = [NONE; 4];
        run_module_internal(module, engine, &mut storage, true)?
    } else if module.inner.registers <= 8 {
        let mut storage = [NONE; 8];
        run_module_internal(module, engine, &mut storage, true)?
    } else if module.inner.registers <= 16 {
        let mut storage = [NONE; 16];
        run_module_internal(module, engine, &mut storage, true)?
    } else if module.inner.registers <= 32 {
        let mut storage = [NONE; 32];
        run_module_internal(module, engine, &mut storage, true)?
    } else {
        let mut storage = vec![NONE; module.inner.registers as usize];
        run_module_internal(module, engine, &mut storage, true)?
    };

    Ok(Module::new(module.inner.name.clone().unwrap(), scope.unwrap())
        .with_content(output.display()))
}

#[typst_macros::time(name = "eval", span = span)]
pub fn run_module_as_eval(
    module: &CompiledModule,
    engine: &mut Engine,
    span: Span,
) -> SourceResult<Value> {
    const NONE: Cow<Value> = Cow::Borrowed(&Value::None);
    let (output, _) = if module.inner.registers <= 4 {
        let mut storage = [NONE; 4];
        run_module_internal(module, engine, &mut storage, false)?
    } else if module.inner.registers <= 8 {
        let mut storage = [NONE; 8];
        run_module_internal(module, engine, &mut storage, false)?
    } else if module.inner.registers <= 16 {
        let mut storage = [NONE; 16];
        run_module_internal(module, engine, &mut storage, false)?
    } else if module.inner.registers <= 32 {
        let mut storage = [NONE; 32];
        run_module_internal(module, engine, &mut storage, false)?
    } else {
        let mut storage = vec![NONE; module.inner.registers as usize];
        run_module_internal(module, engine, &mut storage, false)?
    };

    Ok(output)
}

fn run_module_internal<'a>(
    module: &'a CompiledModule,
    engine: &mut Engine,
    registers: &mut [Cow<'a, Value>],
    scope: bool,
) -> SourceResult<(Value, Option<Scope>)> {
    let mut state = VMState {
        state: State::DISPLAY,
        output: None,
        instruction_pointer: 0,
        registers,
        joined: None,
        code: &**module.inner,
    };

    // Write all default values.
    for default in &module.inner.defaults {
        state
            .write_one(default.target, default.value.clone())
            .at(module.inner.span)?;
    }

    let output = match crate::vm::run(
        engine,
        &mut state,
        &module.inner.instructions,
        &module.inner.spans,
        None,
    )? {
        ControlFlow::Done(value) => value,
        other => bail!(module.inner.span, "module did not produce a value: {other:?}"),
    };

    let scope = scope.then(|| {
        let mut scope = Scope::new();
        for export in &module.inner.exports {
            scope.define(export.name.clone(), state.read(export.value).clone());
        }
        scope
    });

    Ok((output, scope))
}
