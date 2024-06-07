use typst_syntax::Span;

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::Value;
use crate::lang::interpreter::{ControlFlow, Vm};
use crate::lang::opcodes::{
    BeginIter, Enter, Jump, JumpIf, JumpIfNot, JumpTop, Opcode, PointerMarker, Select,
};
use crate::lang::operands::{Readable, Register};

use super::{Run, SimpleRun};

impl Run for Enter {
    fn run(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut Vm,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        let instructions = &instructions[..self.len as usize];
        let spans = &spans[..self.len as usize];

        // Enter the scope within the vm.
        let flow =
            vm.enter_scope(engine, instructions, spans, None, None, self.content, false)?;

        let mut forced_return = false;
        let output = match flow {
            ControlFlow::Done(value) => value,
            ControlFlow::Break(value) => {
                vm.state.set_breaking();
                value
            }
            ControlFlow::Continue(value) => {
                vm.state.set_continuing();
                value
            }
            ControlFlow::Return(value, forced) => {
                vm.state.set_returning(forced);
                forced_return = forced;
                value
            }
        };

        if forced_return {
            let reg = Register(0);
            vm.write_one(reg, output).at(span)?;
            vm.output = Some(Readable::reg(reg));
        } else {
            // Write the output to the output register.
            vm.write_one(self.out, output).at(span)?;
        }

        vm.bump(self.len as usize);

        Ok(())
    }
}

impl SimpleRun for PointerMarker {
    fn run(&self, _: Span, _: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        Ok(())
    }
}

impl SimpleRun for JumpTop {
    fn run(&self, _: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        vm.jump(0);
        Ok(())
    }
}

impl SimpleRun for Jump {
    fn run(&self, _: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Jump to the instruction.
        vm.jump(vm.read(self.instruction));

        Ok(())
    }
}

impl SimpleRun for JumpIf {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the condition.
        let condition = vm.read(self.condition);

        // Get the condition as a boolean.
        let Value::Bool(condition) = condition else {
            bail!(span, "expected boolean, found {}", condition.ty().long_name());
        };

        // Jump to the instruction if the condition is true.
        if *condition {
            vm.jump(vm.read(self.instruction));
        }

        Ok(())
    }
}

impl SimpleRun for JumpIfNot {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the condition.
        let condition = vm.read(self.condition);

        // Get the condition as a boolean.
        let Value::Bool(condition) = condition else {
            bail!(span, "expected boolean, found {}", condition.ty().long_name());
        };

        // Jump to the instruction if the condition is true.
        if !*condition {
            vm.jump(vm.read(self.instruction));
        }

        Ok(())
    }
}

impl SimpleRun for BeginIter {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        if vm.iter() > 100_000 {
            bail!(span, "loop seems to be infinite");
        }

        Ok(())
    }
}

impl SimpleRun for Select {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the condition.
        let condition = vm.read(self.condition);

        // Get the condition as a boolean.
        let Value::Bool(condition) = condition else {
            bail!(span, "expected boolean, found {}", condition.ty().long_name());
        };

        // Select the true value if the condition is true, otherwise select the
        // false value.
        let value = if *condition { vm.read(self.true_) } else { vm.read(self.false_) };

        // Write the value to the output.
        vm.write_one(self.out, value.clone()).at(span)?;

        Ok(())
    }
}
