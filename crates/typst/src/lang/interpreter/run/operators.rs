use typst_syntax::Span;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::Value;
use crate::lang::interpreter::Vm;
use crate::lang::opcodes::{
    Add, And, Auto, CopyIsr, Div, Eq, Geq, Gt, In, Leq, Lt, Mul, Neg, Neq, None, Not,
    NotIn, Or, Pos, ReadAccess, Sub,
};
use crate::lang::ops;

use super::SimpleRun;

impl SimpleRun for Add {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Add the right-hand side from the left-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::add(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Sub {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Subtract the right-hand side from the left-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::sub(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Mul {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Multiply the left-hand side by the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::mul(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Div {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Divide the left-hand side by the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::div(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Neg {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Negate the value and write the result to the output.
        vm.write_one(self.out, ops::neg(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Pos {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Positive the value and write the result to the output.
        vm.write_one(self.out, ops::pos(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Not {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Negate the value and write the result to the output.
        vm.write_one(self.out, ops::not(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Gt {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::gt(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Geq {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::geq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Lt {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::lt(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Leq {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::leq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Eq {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::eq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Neq {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::neq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for In {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value and the iterator.
        let value = vm.read(self.lhs);
        let iterator = vm.read(self.rhs);

        // Check if the value is in the iterator and write the result to the
        // output.
        vm.write_one(self.out, ops::in_(value, iterator).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for NotIn {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value and the iterator.
        let value = vm.read(self.lhs);
        let iterator = vm.read(self.rhs);

        // Check if the value is not in the iterator and write the result to the
        // output.
        vm.write_one(self.out, ops::not_in(value, iterator).at(span)?)
            .at(span)?;

        Ok(())
    }
}

impl SimpleRun for And {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // And the left-hand side with the right-hand side and write the result
        // to the output.
        vm.write_one(self.out, ops::and(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Or {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Or the left-hand side with the right-hand side and write the result
        // to the output.
        vm.write_one(self.out, ops::or(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for CopyIsr {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Write the value to the output.
        vm.write_one(self.out, value).at(span)?;

        Ok(())
    }
}

impl SimpleRun for ReadAccess {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Get the access.
        let access = vm.read(self.access);

        // Get the value.
        let value = access.read(span, vm)?.into_owned();

        // Write the value to the output.
        vm.write_one(self.out, value).at(span)?;

        Ok(())
    }
}

impl SimpleRun for None {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Write `None` to the output.
        vm.write_one(self.out, Value::None).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Auto {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Write the value to the output.
        vm.write_one(self.out, Value::Auto).at(span)?;

        Ok(())
    }
}
