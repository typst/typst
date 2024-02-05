use std::num::{NonZeroU32, NonZeroUsize};

use typst_syntax::{Span, Spanned};
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{bail, error, At, SourceResult, Trace, Tracepoint};
use crate::engine::Engine;
use crate::foundations::{
    array, call_method_mut, is_mutating_method, Arg, Content, Func, IntoValue,
    NativeElement, Recipe, ShowableSelector, Style, Styles, Transformation, Value,
};
use crate::math::{AttachElem, EquationElem, FracElem, LrElem};
use crate::model::{EmphElem, HeadingElem, RefElem, StrongElem};
use crate::vm::{ops, ControlFlow, Register, State};

use super::{
    Access, AccessId, ClosureId, LabelId, PatternId, Pointer, Readable, SpanId, VMState,
    Writable,
};

pub trait Run {
    fn run<I: Iterator<Item = Value>>(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        iterator: Option<&mut I>,
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
            fn run<I: Iterator<Item = Value>>(
                &self,
                instructions: &[Opcode],
                spans: &[Span],
                span: Span,
                vm: &mut VMState,
                engine: &mut Engine,
                iterator: Option<&mut I>
            ) -> SourceResult<()> {
                match self {
                    Self::Flow => {
                        // Move the instruction pointer and counter.
                        vm.instruction_pointer += 1;

                        Ok(())
                    }
                    $(Self::$name($snek) => {
                        vm.instruction_pointer += 1;
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Negate the value and write the result to the output.
        vm.write_one(self.out, ops::neg(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Pos {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Positivize the value and write the result to the output.
        vm.write_one(self.out, ops::pos(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Not {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Negate the value and write the result to the output.
        vm.write_one(self.out, ops::not(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Gt {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the accessor.
        let access = vm.read(self.out).clone();

        // Get the mutable reference to the target.
        let out = access.write(span, vm)?;

        // Write the value to the target.
        *out = value;

        Ok(())
    }
}

impl Run for AddAssign {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the accessor.
        let access = vm.read(self.out).clone();

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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the accessor.
        let access = vm.read(self.out).clone();

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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the accessor.
        let access = vm.read(self.out).clone();

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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the accessor.
        let access = vm.read(self.out).clone();

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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the pattern.
        let pattern = vm.read(self.out).clone();

        // Destructure the value.
        pattern.write(vm, value)?;

        Ok(())
    }
}

impl Run for Or {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Write the value to the output.
        vm.write_one(self.out, value).at(span)?;

        Ok(())
    }
}

impl Run for None {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Write a `none` value to the output.
        vm.write_one(self.out, Value::None).at(span)?;

        Ok(())
    }
}

impl Run for Auto {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Write a `auto` value to the output.
        vm.write_one(self.out, Value::Auto).at(span)?;

        Ok(())
    }
}

impl Run for Set {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Load the target function.
        let target = vm
            .read(self.target)
            .clone()
            .cast::<Func>()
            .and_then(|func| {
                func.element().ok_or_else(|| {
                    error!("only element functions can be used in set rules")
                })
            })
            .at(span)?;

        // Load the arguments.
        let args = vm.read(self.args);
        let args = match args {
            Value::None => crate::foundations::Args::new::<Value>(span, []),
            Value::Args(args) => args.clone(),
            _ => {
                bail!(
                    span,
                    "expected arguments or none, found {}",
                    args.ty().long_name()
                );
            }
        };

        // Create the set rule and store it in the target.
        vm.write_one(self.out, target.set(engine, args)?.spanned(span))
            .at(span)?;

        Ok(())
    }
}

impl Run for Show {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
        let value = Styles::from(Style::Recipe(Recipe {
            span,
            selector: selector.map(|selector| selector.0),
            transform,
        }));

        // Write the value to the output.
        vm.write_one(self.out, value).at(span)?;

        Ok(())
    }
}

impl Run for Styled {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Set that we are only displaying the remaining joined items.
        vm.state |= State::DISPLAY;

        // Load the content.
        let styles = vm.read(self.style).clone();
        if styles.is_none() {
            return Ok(());
        }

        // Load the style
        let style = styles.clone().cast::<Styles>().at(span)?;

        if style.len() == 1 {
            // If it is a single style, without a selector, we must style it using `recipe`
            if let Style::Recipe(r @ Recipe { span: _, selector: None, transform: _ }) =
                &*style.as_slice()[0]
            {
                vm.recipe(r.clone()).at(span)?;
                return Ok(());
            }
        }

        if style.len() == 1 {
            // If it is a single style, without a selector, we must style it using `recipe`
            if let Style::Recipe(r @ Recipe { span: _, selector: None, transform: _ }) =
                &*style.as_slice()[0]
            {
                vm.recipe(r.clone()).at(span)?;
                return Ok(());
            }
        }

        // Style the remaining content.
        vm.styled(style).at(span)?;

        Ok(())
    }
}

impl Run for Instantiate {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Get the function.
        let accessor = vm.read(self.closure).clone();

        // Get the arguments.
        let args = vm.read(self.args);
        let args = match args {
            Value::None => crate::foundations::Args::new::<Value>(span, []),
            Value::Args(args) => args.clone(),
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.access).read(span, vm)?;

        // Write the value to the output.
        vm.write_one(self.out, value.into_owned()).at(span)?;

        Ok(())
    }
}

impl Run for While {
    fn run<I: Iterator<Item = Value>>(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        debug_assert!(self.len as usize <= instructions.len());

        // SAFETY: The instruction pointer is always within the bounds of the
        // instruction list.
        // JUSTIFICATION: This avoids a bounds check on every scope.
        let instructions = unsafe {
            std::slice::from_raw_parts(instructions.as_ptr(), self.len as usize)
        };

        let flow = vm.enter_scope::<std::iter::Empty<Value>>(
            engine,
            instructions,
            spans,
            None,
            None,
            true,
            false,
            true,
        )?;

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
    fn run<I: Iterator<Item = Value>>(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        debug_assert!(self.len as usize <= instructions.len());

        // Get the iterable.
        let iterable = vm.read(self.iterable).clone();

        // SAFETY: The instruction pointer is always within the bounds of the
        // instruction list.
        // JUSTIFICATION: This avoids a bounds check on every scope.
        let instructions = unsafe {
            std::slice::from_raw_parts(instructions.as_ptr(), self.len as usize)
        };

        // Turn the iterable into an iterator.
        let flow = match iterable {
            Value::Str(string) => {
                let mut iter = string.graphemes(true).map(|s| Value::Str(s.into()));

                vm.enter_scope::<&mut dyn Iterator<Item = Value>>(
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
                vm.enter_scope::<&mut dyn Iterator<Item = Value>>(
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
                vm.enter_scope::<&mut dyn Iterator<Item = Value>>(
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        iterator: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        if !vm.state.is_breaking() && !vm.state.is_returning() {
            vm.state |= State::CONTINUING;
        }

        Ok(())
    }
}

impl Run for Break {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        if !vm.state.is_continuing() && !vm.state.is_returning() {
            vm.state |= State::BREAKING;
        }

        Ok(())
    }
}

impl Run for Return {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Obtain the value's span.
        let value_span = vm.read(self.value_span);

        // Obtain the value.
        let value = vm.read(self.value).clone();

        // Obtain the key.
        let Value::Str(key) = vm.read(self.key).clone() else {
            bail!(span, "expected string, found {}", value.ty());
        };

        // Get a mutable reference to the argument set.
        let Value::Args(args) = vm.write(self.out) else {
            bail!(span, "expected argument set, found {}", value.ty());
        };

        args.insert(value_span, key, value);

        Ok(())
    }
}

impl Run for SpreadArg {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
                    name: Some(name),
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).clone();

        match vm.write(self.out) {
            Value::Array(into) => match value {
                Value::Array(array) => {
                    into.extend(array.into_iter().map(|v| v.clone()));
                }
                Value::None => {}
                _ => {
                    bail!(span, "cannot spread {} into array", value.ty());
                }
            },
            Value::Dict(into) => match value {
                Value::Dict(dict) => {
                    into.extend(dict.iter().map(|(k, v)| (k.clone(), v.clone())));
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
                    into.extend(dict.iter().map(|(k, v)| (k.clone(), v.clone())));
                }
                Value::Array(array) => {
                    into.extend(array.into_iter().map(|v| v.clone()));
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        debug_assert!(self.len as usize <= instructions.len());

        // SAFETY: The instruction pointer is always within the bounds of the
        // instruction list.
        // JUSTIFICATION: This avoids a bounds check on every scope.
        let instructions = unsafe {
            std::slice::from_raw_parts(instructions.as_ptr(), self.len as usize)
        };

        // Enter the scope within the vm.
        let joins = self.flags & 0b010 != 0;
        let content = self.flags & 0b100 != 0;

        let flow = vm.enter_scope::<std::iter::Empty<Value>>(
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        _: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        Ok(())
    }
}

impl Run for JumpTop {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Jump to the instruction.
        vm.instruction_pointer = 0;

        Ok(())
    }
}

impl Run for Jump {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Jump to the instruction.
        vm.instruction_pointer = vm.read(self.instruction);

        Ok(())
    }
}

impl Run for JumpIf {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
    ) -> SourceResult<()> {
        // Obtain the left delimiter, body, and right delimiter.
        let left: Content = vm.read(self.left).clone().display();
        let body: Content = vm.read(self.body).clone().display();
        let right: Content = vm.read(self.right).clone().display();

        // Make the value into a delimited.
        let value = LrElem::new(left + body + right);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for Attach {
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
    fn run<I: Iterator<Item = Value>>(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
        _: Option<&mut I>,
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
