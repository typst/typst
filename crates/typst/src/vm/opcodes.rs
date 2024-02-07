use std::num::{NonZeroU32, NonZeroUsize};

use typst_syntax::{Span, Spanned};
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{bail, At, SourceResult, Trace, Tracepoint};
use crate::engine::Engine;
use crate::foundations::{
    array, call_method_mut, is_mutating_method, Arg, Content, Func, IntoValue,
    NativeElement, Recipe, SequenceElem, ShowableSelector, Transformation, Value,
};
use crate::math::{AttachElem, EquationElem, FracElem, LrElem};
use crate::model::{EmphElem, HeadingElem, RefElem, StrongElem};
use crate::util::PicoStr;
use crate::vm::{ops, ControlFlow, Register, State};

use super::{
    Access, AccessId, ClosureId, LabelId, PatternId, Pointer, Readable, SpanId, VMState,
    Writable,
};

pub trait Run {
    fn run(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        iterator: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()>;
}

macro_rules! opcode_struct {
    (
        $(#[$sattr:meta])*
        $name:ident $(-> $out:ty)? $(=> {
            $(
                $(#[$attr:meta])*
                $arg:ident: $arg_ty:ty
            ),* $(,)?
        })?
    ) => {
        $(#[$sattr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(packed)]
        pub struct $name {
            $(
                $(
                    $(#[$attr])*
                    pub $arg: $arg_ty,
                )*
            )?
            $(
                #[doc = "The output of the instruction."]
                pub out: $out,
            )?
        }
    };
}

macro_rules! opcodes {
    (
        $(
            $(#[$sattr:meta])*
            $name:ident: $snek:ident $(-> $out:ty)? $(=> {
                $(
                    $(#[$attr:meta])*
                    $arg:ident: $arg_ty:ty
                ),* $(,)?
            })?
        ),* $(,)?
    ) => {
        $(
            opcode_struct! {
                $(#[$sattr])*
                $name $(-> $out)? $(=> {
                    $(
                        $(#[$attr])*
                        $arg: $arg_ty
                    ),*
                })?
            }
        )*

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Opcode {
            #[doc = "Indicates a flow event."]
            Flow = 0,
            $(
                $(#[$sattr])*
                $name($name)
            ),*
        }

        impl Run for Opcode {
            fn run(
                &self,
                instructions: &[Opcode],
                spans: &[Span],
                span: Span,
                vm: &mut VMState,
                engine: &mut Engine,
                iterator: Option<&mut dyn Iterator<Item = Value>>
            ) -> SourceResult<()> {
                vm.instruction_pointer += 1;

                match self {
                    Self::Flow => Ok(()),
                    $(Self::$name($snek) => {
                        $snek.run(
                            &instructions[vm.instruction_pointer..],
                            &spans[vm.instruction_pointer..],
                            span,
                            vm,
                            engine,
                            iterator
                        )
                    })*
                }
            }
        }
    };
}

include!("opcodes_raw.rs");

impl Run for Add {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Add the left-hand side to the right-hand side and write the result
        // to the output.
        vm.write_one(self.out, ops::add(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Sub {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Subtract the right-hand side from the left-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::sub(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Mul {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Multiply the left-hand side by the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::mul(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Div {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Divide the left-hand side by the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::div(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Neg {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Negate the value and write the result to the output.
        vm.write_one(self.out, ops::neg(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Pos {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Positivize the value and write the result to the output.
        vm.write_one(self.out, ops::pos(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Not {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Negate the value and write the result to the output.
        vm.write_one(self.out, ops::not(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Gt {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::gt(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Geq {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::geq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Lt {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::lt(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Leq {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::leq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Eq {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::eq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Neq {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::neq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for In {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Check whether the left-hand side is in the right-hand side and write
        // the result to the output.
        vm.write_one(self.out, ops::in_(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for NotIn {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Check whether the left-hand side is not in the right-hand side and
        // write the result to the output.
        vm.write_one(self.out, ops::not_in(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for And {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Check whether the left-hand side is true and write the result to the
        // output.
        vm.write_one(self.out, ops::and(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Assign {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the accessor.
        let access = vm.read(self.out);

        // Get the mutable reference to the target.
        let out = access.write(span, vm)?;

        // Write the value to the target.
        *out = value;

        Ok(())
    }
}

impl Run for AddAssign {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the accessor.
        let access = vm.read(self.out);

        // Get the mutable reference to the target.
        let out = access.write(span, vm)?;

        // Take the p: Transformationrevious value. (non-allocating)
        let pre = std::mem::take(out);

        // Add the value to the target.
        *out = ops::add(&pre, &value).at(span)?;

        Ok(())
    }
}

impl Run for SubAssign {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the accessor.
        let access = vm.read(self.out);

        // Get the mutable reference to the target.
        let out = access.write(span, vm)?;

        // Take the previous value. (non-allocating)
        let pre = std::mem::take(out);

        // Sub the value to the target.
        *out = ops::sub(&pre, &value).at(span)?;

        Ok(())
    }
}

impl Run for MulAssign {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the accessor.
        let access = vm.read(self.out);

        // Get the mutable reference to the target.
        let out = access.write(span, vm)?;

        // Take the previous value. (non-allocating)
        let pre = std::mem::take(out);

        // Multiply the value and the target.
        *out = ops::mul(&pre, &value).at(span)?;

        Ok(())
    }
}

impl Run for DivAssign {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the accessor.
        let access = vm.read(self.out);

        // Get the mutable reference to the target.
        let out = access.write(span, vm)?;

        // Take the previous value. (non-allocating)
        let pre = std::mem::take(out);

        // Divide the value by the target.
        *out = ops::div(&pre, &value).at(span)?;

        Ok(())
    }
}

impl Run for Destructure {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the pattern.
        let pattern = vm.read(self.out);

        // Destructure the value.
        pattern.write(vm, value)?;

        Ok(())
    }
}

impl Run for Or {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Check whether the left-hand side is true and write the result to the
        // output.
        vm.write_one(self.out, ops::or(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for CopyIsr {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        const NONE: Value = Value::None;
        const AUTO: Value = Value::Auto;
        const TRUE: Value = Value::Bool(true);
        const FALSE: Value = Value::Bool(false);

        // Get the value.
        let value = match self.value {
            Readable::Reg(reg) => {
                let value = vm.read(reg);

                // Write the value to the output.
                vm.write_one(self.out, value.clone()).at(span)?;

                return Ok(());
            }
            Readable::Const(const_) => vm.read(const_),
            Readable::Str(string) => vm.read(string),
            Readable::Global(global) => vm.read(global),
            Readable::Math(math) => vm.read(math),
            Readable::None => &NONE,
            Readable::Auto => &AUTO,
            Readable::Bool(bool_) => {
                if bool_ {
                    &TRUE
                } else {
                    &FALSE
                }
            }
        };

        // Write the value to the output.
        match self.out {
            Writable::Reg(reg) => vm.write_borrowed(reg, value).at(span)?,
            Writable::Joined => vm.join(value.clone()).at(span)?,
        }

        Ok(())
    }
}

impl Run for None {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Write a `none` value to the output.
        vm.write_one(self.out, Value::None).at(span)?;

        Ok(())
    }
}

impl Run for Auto {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Write a `auto` value to the output.
        vm.write_one(self.out, Value::Auto).at(span)?;

        Ok(())
    }
}

impl Run for Set {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Load the arguments.
        let args = match self.args {
            Readable::Reg(reg) => vm.take(reg).into_owned(),
            other => vm.read(other).clone(),
        };

        let args = match args {
            Value::None => crate::foundations::Args::new::<Value>(span, []),
            Value::Args(args) => args,
            _ => {
                bail!(
                    span,
                    "expected arguments or none, found {}",
                    args.ty().long_name()
                );
            }
        };

        // Load the target function.
        let target = match vm.read(self.target) {
            Value::Func(func) => {
                if let Some(elem) = func.element() {
                    elem
                } else {
                    bail!(span, "only element functions can be used in set rules")
                }
            }
            Value::Type(ty) => {
                if let Some(elem) = ty.constructor().at(span)?.element() {
                    elem
                } else {
                    bail!(span, "only element functions can be used in set rules")
                }
            }
            other => bail!(span, "expected function, found {}", other.ty()),
        };

        // Build the rule and apply it.
        let set_rule = target.set(engine, args)?.spanned(span);
        vm.styled(set_rule).at(span)?;

        Ok(())
    }
}

impl Run for Show {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Load the selector.
        let selector = self
            .selector
            .map(|selector| vm.read(selector).clone().cast::<ShowableSelector>())
            .transpose()
            .at(span)?;

        // Load the transform.
        let transform =
            vm.read(self.transform).clone().cast::<Transformation>().at(span)?;

        // Create the show rule.
        let show_rule = Recipe {
            span,
            selector: selector.map(|selector| selector.0),
            transform,
        };

        // Write the value to the output.
        vm.recipe(show_rule).at(span)?;

        Ok(())
    }
}

impl Run for ShowSet {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Load the selector.
        let selector = self
            .selector
            .map(|selector| vm.read(selector).clone().cast::<ShowableSelector>())
            .transpose()
            .at(span)?;
        // Load the arguments.
        let args = match self.args {
            Readable::Reg(reg) => vm.take(reg).into_owned(),
            other => vm.read(other).clone(),
        };

        let args = match args {
            Value::None => crate::foundations::Args::new::<Value>(span, []),
            Value::Args(args) => args,
            _ => {
                bail!(
                    span,
                    "expected arguments or none, found {}",
                    args.ty().long_name()
                );
            }
        };

        // Load the target function.
        let target = match vm.read(self.target) {
            Value::Func(func) => {
                if let Some(elem) = func.element() {
                    elem
                } else {
                    bail!(span, "only element functions can be used in set rules")
                }
            }
            Value::Type(ty) => {
                if let Some(elem) = ty.constructor().at(span)?.element() {
                    elem
                } else {
                    bail!(span, "only element functions can be used in set rules")
                }
            }
            other => bail!(span, "expected function, found {}", other.ty()),
        };

        // Create the show rule.
        let set_rule = target.set(engine, args)?.spanned(span);
        let show_rule = Recipe {
            span,
            selector: selector.map(|selector| selector.0),
            transform: Transformation::Style(set_rule),
        };

        // Write the value to the output.
        vm.recipe(show_rule).at(span)?;

        Ok(())
    }
}

impl Run for Instantiate {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Get the closure.
        let closure = vm.read(self.closure);
        let closure_span = closure.inner.span;

        // Instantiate the closure. This involves:
        // - Capturing all necessary values.
        // - Capturing the default values of named arguments.
        let closure = vm.instantiate(closure)?;

        // Write the closure to the output.
        vm.write_one(self.out, Func::from(closure).spanned(closure_span))
            .at(span)?;

        Ok(())
    }
}

impl Run for Call {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Get the function.
        let accessor = vm.read(self.closure);

        // Get the arguments.
        let args = match self.args {
            Readable::Reg(reg) => vm.take(reg).into_owned(),
            other => vm.read(other).clone(),
        };

        let args = match args {
            Value::None => crate::foundations::Args::new::<Value>(span, []),
            Value::Args(args) => args,
            _ => {
                bail!(
                    span,
                    "expected arguments or none, found {}",
                    args.ty().long_name()
                );
            }
        };

        match accessor {
            Access::Chained(rest, last) if is_mutating_method(&last) => {
                // Obtain the value.
                let mut value = rest.write(span, vm)?;

                // Call the method.
                let value = call_method_mut(&mut value, &last, args, span)?;

                // Write the value to the output.
                vm.write_one(self.out, value).at(span)?;
            }
            other => {
                // Obtain the value.
                let func = other.read(span, vm)?;

                // Call the method.
                let value = match &*func {
                    Value::Func(func) => {
                        let point = || Tracepoint::Call(func.name().map(Into::into));
                        func.call(engine, args).trace(engine.world, point, span)?
                    }
                    Value::Type(type_) => {
                        let point = || Tracepoint::Call(func.name().map(Into::into));
                        type_.constructor().at(span)?.call(engine, args).trace(
                            engine.world,
                            point,
                            span,
                        )?
                    }
                    _ => {
                        bail!(span, "expected function, found {}", func.ty().long_name())
                    }
                };

                // Write the value to the output.
                vm.write_one(self.out, value).at(span)?;
            }
        }

        Ok(())
    }
}

impl Run for Field {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.access).read(span, vm)?;

        // Write the value to the output.
        vm.write_one(self.out, value.into_owned()).at(span)?;

        Ok(())
    }
}

impl Run for While {
    fn run(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        let instructions = &instructions[..self.len as usize];

        let flow =
            vm.enter_scope(engine, instructions, spans, None, None, true, false, true)?;

        let mut forced_return = false;
        let output = match flow {
            ControlFlow::Done(value) => value,
            ControlFlow::Break(_) | ControlFlow::Continue(_) => {
                unreachable!("unexpected control flow")
            }
            ControlFlow::Return(value, forced) => {
                vm.state |=
                    if forced { State::FORCE_RETURNING } else { State::RETURNING };
                forced_return = forced;
                value
            }
        };

        if forced_return {
            let reg = Register(0);
            vm.write_one(reg, output).at(span)?;
            vm.output = Some(Readable::reg(reg));
        } else if let Some(out) = self.out {
            // Write the output to the output register.
            vm.write_one(out, output).at(span)?;
        }

        vm.instruction_pointer += self.len as usize;

        Ok(())
    }
}

impl Run for Iter {
    #[typst_macros::time(name = "for loop", span = span)]
    fn run(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        debug_assert!(self.len as usize <= instructions.len());

        // Get the iterable.
        let iterable = vm.read(self.iterable).clone();
        let instructions = &instructions[..self.len as usize];

        // Turn the iterable into an iterator.
        let flow = match iterable {
            Value::Str(string) => {
                let mut iter = string.graphemes(true).map(|s| Value::Str(s.into()));

                vm.enter_scope(
                    engine,
                    instructions,
                    spans,
                    Some(&mut iter),
                    None,
                    true,
                    false,
                    true,
                )?
            }
            Value::Array(array) => {
                let mut iter = array.iter().cloned();
                vm.enter_scope(
                    engine,
                    instructions,
                    spans,
                    Some(&mut iter),
                    None,
                    true,
                    false,
                    true,
                )?
            }
            Value::Dict(dict) => {
                let mut iter = dict
                    .into_iter()
                    .map(|(key, value)| array![key.into_value(), value].into_value());
                vm.enter_scope(
                    engine,
                    instructions,
                    spans,
                    Some(&mut iter),
                    None,
                    true,
                    false,
                    true,
                )?
            }
            _ => {
                bail!(
                    span,
                    "expected array, string, or dictionary, found {}",
                    iterable.ty().long_name()
                );
            }
        };

        let mut forced_return = false;
        let output = match flow {
            ControlFlow::Done(value) => value,
            ControlFlow::Break(_) | ControlFlow::Continue(_) => {
                unreachable!("unexpected control flow")
            }
            ControlFlow::Return(value, forced) => {
                vm.state |=
                    if forced { State::FORCE_RETURNING } else { State::RETURNING };
                forced_return = forced;
                value
            }
        };

        if forced_return {
            let reg = Register(0);
            vm.write_one(reg, output).at(span)?;
            vm.output = Some(Readable::reg(reg));
        } else if let Some(out) = self.out {
            // Write the output to the output register.
            vm.write_one(out, output).at(span)?;
        }

        vm.instruction_pointer += self.len as usize;

        Ok(())
    }
}

impl Run for Next {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        iterator: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        let Some(iter) = iterator else {
            bail!(span, "not in an iterable scope");
        };

        // Get the next value.
        let Some(value) = iter.next() else {
            vm.state |= State::DONE;
            return Ok(());
        };

        // Write the value to the output.
        vm.write_one(self.out, value).at(span)?;

        Ok(())
    }
}

impl Run for Continue {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        if !vm.state.is_breaking() && !vm.state.is_returning() {
            vm.state |= State::CONTINUING;
        }

        Ok(())
    }
}

impl Run for Break {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        if !vm.state.is_continuing() && !vm.state.is_returning() {
            vm.state |= State::BREAKING;
        }

        Ok(())
    }
}

impl Run for Return {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        vm.output = self.value;
        if !vm.state.is_breaking() && !vm.state.is_continuing() {
            if vm.output.is_some() {
                vm.state |= State::FORCE_RETURNING;
            } else {
                vm.state |= State::RETURNING;
            }
        }

        Ok(())
    }
}

impl Run for Array {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Create a new array.
        let array = Value::Array(crate::foundations::Array::with_capacity(
            self.capacity as usize,
        ));

        // Write the array to the output.
        vm.write_one(self.out, array).at(span)?;

        Ok(())
    }
}

impl Run for Push {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).clone();

        // Get a mutable reference to the array.
        let Value::Array(array) = vm.write(self.out) else {
            bail!(span, "expected array, found {}", value.ty().long_name());
        };

        array.push(value);

        Ok(())
    }
}

impl Run for Dict {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Create a new dictionary.
        let dict =
            Value::Dict(crate::foundations::Dict::with_capacity(self.capacity as usize));

        // Write the dictionary to the output.
        vm.write_one(self.out, dict).at(span)?;

        Ok(())
    }
}

impl Run for Insert {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).clone();

        // Obtain the key.
        let Value::Str(key) = vm.read(self.key).clone() else {
            bail!(span, "expected string, found {}", value.ty().long_name());
        };

        // Get a mutable reference to the dictionary.
        let Value::Dict(dict) = vm.write(self.out) else {
            bail!(span, "expected dictionary, found {}", value.ty().long_name());
        };

        dict.insert(key, value);

        Ok(())
    }
}

impl Run for Args {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Create a new argument set.
        let args = Value::Args(crate::foundations::Args::with_capacity(
            span,
            self.capacity as usize,
        ));

        // Write the argument set to the output.
        vm.write_one(self.out, args).at(span)?;

        Ok(())
    }
}

impl Run for PushArg {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value's span.
        let value_span = vm.read(self.value_span);

        // Obtain the value.
        let value = vm.read(self.value).clone();

        // Get a mutable reference to the argument set.
        let Value::Args(args) = vm.write(self.out) else {
            bail!(span, "expected argument set, found {}", value.ty().long_name());
        };

        args.push(value_span, value);

        Ok(())
    }
}

impl Run for InsertArg {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value's span.
        let value_span = vm.read(self.value_span);

        // Obtain the value.
        let value = vm.read(self.value).clone();

        // Get a mutable reference to the argument set.
        let Value::Args(args) = vm.write(self.out) else {
            bail!(span, "expected argument set, found {}", value.ty());
        };

        args.insert(value_span, self.key, value);

        Ok(())
    }
}

impl Run for SpreadArg {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value's span.
        let value_span = vm.read(self.value_span);

        // Obtain the value.
        let value = vm.read(self.value).clone();

        // Get a mutable reference to the argument set.
        let Value::Args(into) = vm.write(self.out) else {
            bail!(span, "expected argument set, found {}", value.ty().long_name());
        };

        match value {
            Value::Args(args_) => {
                into.chain(args_);
            }
            Value::Dict(dict) => {
                into.extend(dict.into_iter().map(|(name, value)| Arg {
                    span,
                    name: Some(PicoStr::new(name.as_str())),
                    value: Spanned::new(value, value_span),
                }));
            }
            Value::Array(array) => {
                into.extend(array.into_iter().map(|value| Arg {
                    span,
                    name: None,
                    value: Spanned::new(value, value_span),
                }));
            }
            Value::None => {}
            _ => {
                bail!(span, "cannot spread {}", value.ty());
            }
        }

        Ok(())
    }
}

impl Run for Spread {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).clone();

        match vm.write(self.out) {
            Value::Array(into) => match value {
                Value::Array(array) => {
                    into.extend(array.into_iter());
                }
                Value::None => {}
                _ => {
                    bail!(span, "cannot spread {} into array", value.ty());
                }
            },
            Value::Dict(into) => match value {
                Value::Dict(dict) => {
                    into.extend(dict.into_iter());
                }
                Value::None => {}
                _ => {
                    bail!(span, "cannot spread {} into dictionary", value.ty());
                }
            },
            Value::Args(into) => match value {
                Value::Args(args_) => {
                    into.chain(args_);
                }
                Value::Dict(dict) => {
                    into.extend(dict.into_iter());
                }
                Value::Array(array) => {
                    into.extend(array.into_iter());
                }
                Value::None => {}
                _ => {
                    bail!(span, "cannot spread {}", value.ty());
                }
            },
            _ => {
                bail!(
                    span,
                    "expected array, dictionary, or arguments, found {}",
                    value.ty().long_name()
                );
            }
        }

        Ok(())
    }
}

impl Run for Enter {
    fn run(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        let instructions = &instructions[..self.len as usize];

        // Enter the scope within the vm.
        let joins = self.flags & 0b010 != 0;
        let content = self.flags & 0b100 != 0;

        let flow = vm.enter_scope(
            engine,
            instructions,
            spans,
            None,
            None,
            joins,
            content,
            false,
        )?;

        let mut forced_return = false;
        let output = match flow {
            ControlFlow::Done(value) => value,
            ControlFlow::Break(value) => {
                vm.state |= State::BREAKING;
                value
            }
            ControlFlow::Continue(value) => {
                vm.state |= State::CONTINUING;
                value
            }
            ControlFlow::Return(value, forced) => {
                vm.state |=
                    if forced { State::FORCE_RETURNING } else { State::RETURNING };
                forced_return = forced;
                value
            }
        };

        if forced_return {
            let reg = Register(0);
            vm.write_one(reg, output).at(span)?;
            vm.output = Some(Readable::reg(reg));
        } else if let Some(out) = self.out {
            // Write the output to the output register.
            vm.write_one(out, output).at(span)?;
        }

        vm.instruction_pointer += self.len as usize;

        Ok(())
    }
}

impl Run for PointerMarker {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        _: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        Ok(())
    }
}

impl Run for JumpTop {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Jump to the instruction.
        vm.instruction_pointer = 0;

        Ok(())
    }
}

impl Run for Jump {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Jump to the instruction.
        vm.instruction_pointer = vm.read(self.instruction);

        Ok(())
    }
}

impl Run for JumpIf {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the condition.
        let condition = vm.read(self.condition);

        // Get the condition as a boolean.
        let Value::Bool(condition) = condition else {
            bail!(span, "expected boolean, found {}", condition.ty().long_name());
        };

        // Jump to the instruction if the condition is true.
        if *condition {
            vm.instruction_pointer = vm.read(self.instruction);
        }

        Ok(())
    }
}

impl Run for JumpIfNot {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the condition.
        let condition = vm.read(self.condition);

        // Get the condition as a boolean.
        let Value::Bool(condition) = condition else {
            bail!(span, "expected boolean, found {}", condition.ty().long_name());
        };

        // Jump to the instruction if the condition is false.
        if !*condition {
            vm.instruction_pointer = vm.read(self.instruction);
        }

        Ok(())
    }
}

impl Run for Select {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
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

impl Run for Delimited {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the left delimiter, body, and right delimiter.
        let left: Content = vm.read(self.left).clone().display();
        let body: Content = vm.read(self.body).clone().display();
        let right: Content = vm.read(self.right).clone().display();

        // Make the value into a delimited.
        let value = LrElem::new(
            SequenceElem::new(vec![left.into(), body.into(), right.into()])
                .pack()
                .spanned(span),
        );

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for Attach {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the base, top, and bottom.
        let base = vm.read(self.base);
        let top = vm.read(self.top);
        let bottom = vm.read(self.bottom);

        // Make the value into an attach.
        let mut value = AttachElem::new(base.clone().display());

        if let Some(top) = top {
            value.push_t(Some(top.clone().display()));
        }

        if let Some(bottom) = bottom {
            value.push_b(Some(bottom.clone().display()));
        }

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for Frac {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the numerator and denominator.
        let numerator = vm.read(self.numerator);
        let denominator = vm.read(self.denominator);

        // Make the value into a fraction.
        let value =
            FracElem::new(numerator.clone().display(), denominator.clone().display());

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for Root {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the degree and radicand.
        let degree = vm.read(self.degree);
        let radicand = vm.read(self.radicand);

        // Make the value into a root.
        let mut value = crate::math::RootElem::new(radicand.clone().display());

        if let Some(degree) = degree {
            value.push_index(Some(degree.clone().display()));
        }

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for Ref {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the supplement.
        let supplement = self.supplement.map(|supplement| vm.read(supplement));

        // Create the reference.
        let mut reference = RefElem::new(vm.read(self.label));

        if let Some(supplement) = supplement {
            reference.push_supplement(supplement.clone().cast().at(span)?);
        }

        // Write the reference to the output.
        vm.write_one(self.out, reference.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for Strong {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value strong.
        let value = StrongElem::new(value.clone().cast().at(span)?);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for Emph {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value emphasized.
        let value = EmphElem::new(value.clone().cast().at(span)?);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for Heading {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value and level.
        let value = vm.read(self.value);
        let level = self.level;

        // Make the value into a heading.
        let mut value = HeadingElem::new(value.clone().cast().at(span)?);

        // Set the level of the heading.
        let Some(level) = NonZeroUsize::new(level as usize) else {
            bail!(span, "heading level must be greater than zero, instruction malformed");
        };
        value.push_level(level);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for ListItem {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value into a list item.
        let value = crate::model::ListItem::new(value.clone().cast().at(span)?);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for EnumItem {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value and number.
        let value = vm.read(self.value);
        let number = self.number.map(|number| number.get() as usize - 1);

        // Make the value into an enum item.
        let value = crate::model::EnumItem::new(value.clone().cast().at(span)?)
            .with_number(number);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for TermItem {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value and description.
        let value = vm.read(self.term);
        let description = vm.read(self.description);

        // Make the value into a term.
        let value = crate::model::TermItem::new(
            value.clone().cast().at(span)?,
            description.clone().cast().at(span)?,
        );

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for Equation {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value into an equation.
        let value = EquationElem::new(value.clone().cast().at(span)?);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}
