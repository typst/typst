use typst_syntax::Span;

use crate::diag::{bail, StrResult};
use crate::foundations::{IntoValue, Value};
use crate::lang::compiled::{
    CompiledAccess, CompiledClosure, CompiledDynamicModule, CompiledPattern,
};
use crate::lang::opcodes::{
    AccessId, ClosureId, LabelId, PatternId, Pointer, Readable, SpanId, Writable,
};
use crate::lang::operands::{Constant, Global, Math, ModuleId, Register, StringId};

use super::Vm;

/// Defines a value that can be read from the VM.
pub trait Read {
    type Output<'a, 'b>
    where
        'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b>;
}

/// Defines a value that can be written to the VM.
pub trait Write {
    fn write<'a>(&self, vm: &'a mut Vm) -> Option<&'a mut Value>;

    fn write_one(self, vm: &mut Vm, value: impl IntoValue) -> StrResult<()>
    where
        Self: Sized,
    {
        let Some(target) = self.write(vm) else {
            bail!("cannot write to a temporary value")
        };

        *target = value.into_value();

        Ok(())
    }
}

impl<T: Read> Read for Option<T> {
    type Output<'a, 'b> = Option<T::Output<'a, 'b>> where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        if let Some(this) = self {
            Some(this.read(vm))
        } else {
            None
        }
    }
}

impl Read for Readable {
    type Output<'a, 'b> = &'b Value where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        match self {
            Readable::Const(const_) => const_.read(vm),
            Readable::Reg(reg) => vm.read(*reg),
            Readable::Str(str_) => str_.read(vm),
            Readable::Global(global) => global.read(vm),
            Readable::Math(math) => math.read(vm),
            Readable::Bool(bool_) => {
                if *bool_ {
                    &Value::Bool(true)
                } else {
                    &Value::Bool(false)
                }
            }
            Readable::Label(label) => label.read(vm),
            Readable::None => &Value::None,
            Readable::Auto => &Value::Auto,
            Readable::GlobalModule => &vm.code.global.std,
        }
    }
}

impl Read for Writable {
    type Output<'a, 'b> = &'b Value where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        match self {
            Self::Reg(register) => register.read(vm),
            Self::Joiner => unreachable!("cannot read joined value"),
        }
    }
}

impl Write for Writable {
    fn write<'a, 'b>(&self, vm: &'b mut Vm<'a, '_>) -> Option<&'b mut Value> {
        match self {
            Self::Reg(register) => register.write(vm),
            Self::Joiner => unreachable!("cannot get mutable reference to joined value"),
        }
    }

    fn write_one<'a>(self, vm: &mut Vm<'a, '_>, value: impl IntoValue) -> StrResult<()>
    where
        Self: Sized,
    {
        match self {
            Self::Reg(register) => register.write_one(vm, value),
            Self::Joiner => vm.join(value),
        }
    }
}

impl Read for Constant {
    type Output<'a, 'b> = &'a Value where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        &vm.code.constants[self.0 as usize]
    }
}

impl Read for Register {
    type Output<'a, 'b> = &'b Value where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        &*vm.registers[self.0 as usize]
    }
}

impl Write for Register {
    fn write<'a, 'b>(&self, vm: &'b mut Vm<'a, '_>) -> Option<&'b mut Value> {
        Some(vm.registers[self.0 as usize].to_mut())
    }
}

impl Read for StringId {
    type Output<'a, 'b> = &'a Value where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        &vm.code.strings[self.0 as usize]
    }
}

impl Read for ClosureId {
    type Output<'a, 'b> = &'a CompiledClosure where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        &vm.code.closures[self.0 as usize]
    }
}

impl Read for Global {
    type Output<'a, 'b> = &'a Value where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        vm.code.global.global.field_by_index(self.0 as usize).unwrap()
    }
}

impl Read for Math {
    type Output<'a, 'b> = &'a Value where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        vm.code.global.math.field_by_index(self.0 as usize).unwrap()
    }
}

impl Read for ModuleId {
    type Output<'a, 'b> = &'a CompiledDynamicModule where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        &vm.code.modules[self.0 as usize]
    }
}

impl Read for LabelId {
    type Output<'a, 'b> = &'a Value where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        &vm.code.labels[self.0 as usize]
    }
}

impl Read for AccessId {
    type Output<'a, 'b> = &'a CompiledAccess where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        &vm.code.accesses[self.0 as usize]
    }
}

impl Read for PatternId {
    type Output<'a, 'b> = &'a CompiledPattern where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        &vm.code.patterns[self.0 as usize]
    }
}

impl Read for SpanId {
    type Output<'a, 'b> = Span where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        vm.code.isr_spans[self.0 as usize]
    }
}

impl Read for Pointer {
    type Output<'a, 'b> = usize where 'a: 'b;

    fn read<'a, 'b>(&self, vm: &'b Vm<'a, '_>) -> Self::Output<'a, 'b> {
        vm.code.jumps[self.0 as usize]
    }
}
