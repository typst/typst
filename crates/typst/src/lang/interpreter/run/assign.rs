use typst_syntax::Span;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::Value;
use crate::lang::compiled::CompiledAccess;
use crate::lang::interpreter::Vm;
use crate::lang::opcodes::{
    AddAssign, Assign, Destructure, DivAssign, MulAssign, SubAssign,
};
use crate::lang::operands::{AccessId, Readable};
use crate::lang::ops;

use super::SimpleRun;

impl SimpleRun for Assign {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        assign(span, vm, engine, self.value, self.out)
    }
}

impl SimpleRun for AddAssign {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        let lhs_span = vm.read(self.lhs_span);
        assign_op(lhs_span, vm, engine, self.value, self.out, |old, value| {
            ops::add(&old, &value).at(span)
        })
    }
}

impl SimpleRun for SubAssign {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        let lhs_span = vm.read(self.lhs_span);
        assign_op(lhs_span, vm, engine, self.value, self.out, |old, value| {
            ops::sub(&old, &value).at(span)
        })
    }
}

impl SimpleRun for MulAssign {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        let lhs_span = vm.read(self.lhs_span);
        assign_op(lhs_span, vm, engine, self.value, self.out, |old, value| {
            ops::mul(&old, &value).at(span)
        })
    }
}

impl SimpleRun for DivAssign {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        let lhs_span = vm.read(self.lhs_span);
        assign_op(lhs_span, vm, engine, self.value, self.out, |old, value| {
            ops::div(&old, &value).at(span)
        })
    }
}

impl SimpleRun for Destructure {
    fn run(&self, _: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the pattern.
        let pattern = vm.read(self.out);

        // Destructure the value.
        pattern.write(vm, engine, value)?;

        Ok(())
    }
}

fn assign(
    span: Span,
    vm: &mut Vm,
    engine: &mut Engine,
    value: Readable,
    out: AccessId,
) -> SourceResult<()> {
    // Get the value.
    let value = vm.read(value).clone();

    // Get the accessor.
    let access = vm.read(out);

    // Get the mutable reference to the target.
    if let CompiledAccess::Chained(_, dict, field, _) = access {
        let dict = vm.read(*dict);
        if let CompiledAccess::Register(dict) = dict {
            if let Some(Value::Dict(dict)) = vm.write(*dict) {
                dict.insert((*field).into(), value);
                return Ok(());
            }
        }
    }

    let out = access.write(span, vm, engine)?;

    // Write the value to the target.
    *out = value;

    Ok(())
}

fn assign_op(
    span: Span,
    vm: &mut Vm,
    engine: &mut Engine,
    value: Readable,
    out: AccessId,
    transformer: impl FnOnce(Value, Value) -> SourceResult<Value>,
) -> SourceResult<()> {
    // Get the value.
    let value = vm.read(value).clone();

    // Get the accessor.
    let access = vm.read(out);

    // Get the mutable reference to the target.
    if let CompiledAccess::Chained(_, dict, field, field_span) = access {
        let dict = vm.read(*dict);
        if let CompiledAccess::Register(dict) = dict {
            if let Some(Value::Dict(dict)) = vm.write(*dict) {
                let item = dict.at_mut(*field).at(*field_span)?;

                let old = std::mem::take(item);
                *item = transformer(old, value)?;

                return Ok(());
            }
        }
    }

    let out = access.write(span, vm, engine)?;

    // Write the value to the target.
    let old = std::mem::take(out);
    *out = transformer(old, value)?;

    Ok(())
}
