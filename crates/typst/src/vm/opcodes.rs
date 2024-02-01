use std::num::{NonZeroU32, NonZeroUsize};

use bytemuck::AnyBitPattern;
use typst_syntax::Span;
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{bail, error, At, SourceResult};
use crate::engine::Engine;
use crate::eval::ops;
use crate::foundations::{
    array, call_method_mut, is_mutating_method, Content, Func, IntoValue, NativeElement,
    Recipe, ShowableSelector, Style, Styles, Transformation, Value,
};
use crate::math::{AttachElem, EquationElem, FracElem, LrElem};
use crate::model::{EmphElem, HeadingElem, RefElem, StrongElem};
use crate::vm::{ControlFlow, State};

use super::{
    Access, AccessId, ClosureId, LabelId, OptionalReadable, OptionalWritable, PatternId,
    Pointer, Readable, ScopeId, VMState, Writable,
};

pub trait Run {
    fn run(
        &self,
        instructions: &[u8],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
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
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AnyBitPattern)]
        #[repr(C)]
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
    ($(
        $(#[$sattr:meta])*
        $name:ident: $snek:ident $(-> $out:ty)? $(=> {
            $(
                $(#[$attr:meta])*
                $arg:ident: $arg_ty:ty
            ),* $(,)?
        })? = $value:expr
    ),* $(,)?) => {
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
        pub enum Opcode {
            #[doc = "Indicates a flow event."]
            Flow,
            $(
                $(#[$sattr])*
                $name
            ),*
        }

        impl Opcode {
            pub fn from_u8(value: u8) -> Option<Self> {
                match value {
                    0x00 => Some(Self::Flow),
                    $($value => Some(Self::$name),)*
                    _ => None,
                }
            }

            pub fn to_u8(self) -> u8 {
                match self {
                    Self::Flow => 0x00,
                    $(Self::$name => $value,)*
                }
            }
        }

        impl Run for Opcode {
            fn run(
                &self,
                instructions: &[u8],
                span: Span,
                vm: &mut VMState,
                engine: &mut Engine,
            ) -> SourceResult<()> {
                const OFFSET: usize = std::mem::size_of::<u8>() + std::mem::size_of::<Span>();
                match self {
                    Self::Flow => {
                        // Move the instruction pointer and counter.
                        vm.instruction_pointer += 1;

                        Ok(())
                    }
                    $(Self::$name => {
                        // The constant that contains the length of one instruction.
                        const LEN: usize = {
                            let mut __i = 0;
                            $($(
                                // Add the size of the argument.
                                __i += std::mem::size_of::<$arg_ty>();
                            )*)?
                            $(
                                __i += std::mem::size_of::<$out>();
                            )?

                            __i
                        };

                        // Obtain the instruction's slice.
                        debug_assert!(vm.instruction_pointer + OFFSET + LEN <= instructions.len());
                        let instructions = &instructions[vm.instruction_pointer + OFFSET..];

                        // Copy the instruction into the stack.
                        let mut __i = 0;
                        $($(
                            let $arg = bytemuck::pod_read_unaligned(&instructions[__i..__i + std::mem::size_of::<$arg_ty>()]);
                            __i += std::mem::size_of::<$arg_ty>();
                        )*)?
                        let instruction = $name {
                            $($($arg,)*)?
                            $(
                                out: bytemuck::pod_read_unaligned(&instructions[__i..__i + std::mem::size_of::<$out>()]),
                            )?
                        };

                        eprintln!("{instruction:?}");

                        // Move the instruction pointer and counter.
                        vm.instruction_pointer += OFFSET + LEN;

                        // Run the instruction.
                        instruction.run(&instructions[LEN..], span, vm, engine)?;

                        Ok(())
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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Add the left-hand side to the right-hand side and write the result
        // to the output.
        vm.write_one(self.out, ops::add(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Sub {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Subtract the right-hand side from the left-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::sub(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Mul {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Multiply the left-hand side by the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::mul(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Div {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Divide the left-hand side by the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::div(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Neg {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?;

        // Negate the value and write the result to the output.
        vm.write_one(self.out, ops::neg(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Pos {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?;

        // Positivize the value and write the result to the output.
        vm.write_one(self.out, ops::pos(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Not {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?;

        // Negate the value and write the result to the output.
        vm.write_one(self.out, ops::not(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Gt {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::gt(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Geq {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::geq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Lt {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::lt(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Leq {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::leq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Eq {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::eq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Neq {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::neq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for In {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Check whether the left-hand side is in the right-hand side and write
        // the result to the output.
        vm.write_one(self.out, ops::in_(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for NotIn {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Check whether the left-hand side is not in the right-hand side and
        // write the result to the output.
        vm.write_one(self.out, ops::not_in(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for And {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Check whether the left-hand side is true and write the result to the
        // output.
        vm.write_one(self.out, ops::and(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Assign {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).at(span)?.clone();

        // Get the accessor.
        let access = vm.read(self.out).at(span)?.clone();

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).at(span)?.clone();

        // Get the accessor.
        let access = vm.read(self.out).at(span)?.clone();

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).at(span)?.clone();

        // Get the accessor.
        let access = vm.read(self.out).at(span)?.clone();

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).at(span)?.clone();

        // Get the accessor.
        let access = vm.read(self.out).at(span)?.clone();

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).at(span)?.clone();

        // Get the accessor.
        let access = vm.read(self.out).at(span)?.clone();

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).at(span)?.clone();

        // Get the pattern.
        let pattern = vm.read(self.out).at(span)?.clone();

        // Destructure the value.
        pattern.write(vm, value)?;

        Ok(())
    }
}

impl Run for Or {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left-hand side.
        let lhs = vm.read(self.lhs).at(span)?;
        let rhs = vm.read(self.rhs).at(span)?;

        // Check whether the left-hand side is true and write the result to the
        // output.
        vm.write_one(self.out, ops::or(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl Run for Copy {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).at(span)?.clone();

        // Write the value to the output.
        vm.write_one(self.out, value).at(span)?;

        Ok(())
    }
}

impl Run for None {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Write a `none` value to the output.
        vm.write_one(self.out, Value::None).at(span)?;

        Ok(())
    }
}

impl Run for Auto {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Write a `auto` value to the output.
        vm.write_one(self.out, Value::Auto).at(span)?;

        Ok(())
    }
}

impl Run for Set {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        // Load the target function.
        let target = vm
            .read(self.target)
            .at(span)?
            .clone()
            .cast::<Func>()
            .and_then(|func| {
                func.element().ok_or_else(|| {
                    error!("only element functions can be used in set rules")
                })
            })
            .at(span)?;

        // Load the arguments.
        let args = vm.read(self.args).at(span)?;
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
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Load the selector.
        let selector = self
            .selector
            .ok()
            .map(|selector| vm.read(selector)?.clone().cast::<ShowableSelector>())
            .transpose()
            .at(span)?;

        // Load the transform.
        let transform = vm
            .read(self.transform)
            .at(span)?
            .clone()
            .cast::<Transformation>()
            .at(span)?;

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
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Set that we are only displaying the remaining joined items.
        vm.state |= State::DISPLAY;

        // Load the style
        let style = vm.read(self.style).at(span)?.clone().cast::<Styles>().at(span)?;

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
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Get the closure.
        let closure = vm.read(self.closure).at(span)?;

        // Instantiate the closure. This involves:
        // - Capturing all necessary values.
        // - Capturing the default values of named arguments.
        let closure = vm.instantiate(closure)?;

        // Write the closure to the output.
        vm.write_one(self.out, closure).at(span)?;

        Ok(())
    }
}

impl Run for Call {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        // Get the function.
        let accessor = vm.read(self.closure).at(span)?.clone();

        // Get the arguments.
        let args = vm.read(self.args).at(span)?;
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
                let func = match &*func {
                    Value::Func(func) => func.clone(),
                    Value::Type(type_) => type_.constructor().at(span)?,
                    _ => {
                        bail!(span, "expected function, found {}", func.ty().long_name())
                    }
                };

                // Call the function.
                let value = func.call(engine, args)?;

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.access).at(span)?.read(span, vm)?;

        // Write the value to the output.
        vm.write_one(self.out, value.into_owned()).at(span)?;

        Ok(())
    }
}

impl Run for While {
    fn run(
        &self,
        instructions: &[u8],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        debug_assert!(self.len as usize <= instructions.len());

        // SAFETY: The instruction pointer is always within the bounds of the
        // instruction list.
        // JUSTIFICATION: This avoids a bounds check on every scope.
        let instructions = unsafe {
            std::slice::from_raw_parts(instructions.as_ptr(), self.len as usize)
        };

        let defaults = vm.read(self.scope).at(span)?.clone();

        let f = move || {
            let flow = vm.enter_scope(
                engine,
                &defaults,
                instructions,
                Some(Box::new(std::iter::empty())),
                None,
                true,
                false,
                span,
            )?;

            let joined = match flow {
                ControlFlow::Done(value) => value,
                ControlFlow::Break(_) | ControlFlow::Continue(_) => {
                    unreachable!("unexpected control flow")
                }
                ControlFlow::Return(value) => {
                    vm.state |= State::RETURNING;
                    value
                }
            };

            if let Some(out) = self.out.ok() {
                // Write the output to the output register.
                vm.write_one(out, joined).at(span)?;
            }

            vm.instruction_pointer += self.len as usize;

            Ok(())
        };

        // Stacker is broken on WASM.
        #[cfg(target_arch = "wasm32")]
        return f();

        #[cfg(not(target_arch = "wasm32"))]
        stacker::maybe_grow(32 * 1024, 2 * 1024 * 1024, f)
    }
}

impl Run for Iter {
    fn run(
        &self,
        instructions: &[u8],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        debug_assert!(self.len as usize <= instructions.len());

        // Get the iterable.
        let iterable = vm.read(self.iterable).at(span)?.clone();

        // SAFETY: The instruction pointer is always within the bounds of the
        // instruction list.
        // JUSTIFICATION: This avoids a bounds check on every scope.
        let instructions = unsafe {
            std::slice::from_raw_parts(instructions.as_ptr(), self.len as usize)
        };

        let defaults = vm.read(self.scope).at(span)?.clone();

        // Turn the iterable into an iterator.
        let iter: Box<dyn Iterator<Item = Value>> = match iterable {
            Value::Str(string) => {
                let iter = string
                    .graphemes(true)
                    .map(|s| Value::Str(s.into()))
                    .collect::<Vec<_>>();

                Box::new(iter.into_iter())
            }
            Value::Array(array) => Box::new(array.into_iter()),
            Value::Dict(dict) => Box::new(
                dict.into_iter()
                    .map(|(key, value)| array![key.into_value(), value].into_value()),
            ),
            _ => {
                bail!(
                    span,
                    "expected array or dictionary, found {}",
                    iterable.ty().long_name()
                );
            }
        };

        let f = move || {
            let flow = vm.enter_scope(
                engine,
                &defaults,
                instructions,
                Some(iter),
                None,
                true,
                false,
                span,
            )?;

            let joined = match flow {
                ControlFlow::Done(value) => value,
                ControlFlow::Break(_) | ControlFlow::Continue(_) => {
                    unreachable!("unexpected control flow")
                }
                ControlFlow::Return(value) => {
                    vm.state |= State::RETURNING;
                    value
                }
            };

            if let Some(out) = self.out.ok() {
                // Write the output to the output register.
                vm.write_one(out, joined).at(span)?;
            }

            vm.instruction_pointer += self.len as usize;

            Ok(())
        };

        // Stacker is broken on WASM.
        #[cfg(target_arch = "wasm32")]
        return f();

        #[cfg(not(target_arch = "wasm32"))]
        stacker::maybe_grow(32 * 1024, 2 * 1024 * 1024, f)
    }
}

impl Run for Next {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        let Some(iter) = &mut vm.iterator else {
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
        _: &[u8],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
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
        _: &[u8],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
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
        _: &[u8],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        vm.output = self.value.ok();
        if !vm.state.is_breaking() && !vm.state.is_continuing() {
            vm.state |= State::RETURNING;
        }

        Ok(())
    }
}

impl Run for Array {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?.clone();

        // Get a mutable reference to the array.
        let Value::Array(array) = vm.write(self.out).at(span)? else {
            bail!(span, "expected array, found {}", value.ty().long_name());
        };

        array.push(value);

        Ok(())
    }
}

impl Run for Dict {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?.clone();

        // Obtain the key.
        let Value::Str(key) = vm.read(self.key).at(span)?.clone() else {
            bail!(span, "expected string, found {}", value.ty().long_name());
        };

        // Get a mutable reference to the dictionary.
        let Value::Dict(dict) = vm.write(self.out).at(span)? else {
            bail!(span, "expected dictionary, found {}", value.ty().long_name());
        };

        dict.insert(key, value);

        Ok(())
    }
}

impl Run for Args {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?.clone();

        // Get a mutable reference to the argument set.
        let Value::Args(args) = vm.write(self.out).at(span)? else {
            bail!(span, "expected argument set, found {}", value.ty().long_name());
        };

        args.push(span, value);

        Ok(())
    }
}

impl Run for InsertArg {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?.clone();

        // Obtain the key.
        let Value::Str(key) = vm.read(self.key).at(span)?.clone() else {
            bail!(span, "expected string, found {}", value.ty().long_name());
        };

        // Get a mutable reference to the argument set.
        let Value::Args(args) = vm.write(self.out).at(span)? else {
            bail!(span, "expected argument set, found {}", value.ty().long_name());
        };

        args.insert(span, key, value);

        Ok(())
    }
}

impl Run for Spread {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?.clone();

        match vm.write(self.out).at(span)? {
            Value::Array(into) => match value {
                Value::Array(array) => {
                    into.extend(array.into_iter().map(|v| v.clone()));
                }
                Value::None => {}
                _ => {
                    bail!(
                        span,
                        "expected array or none, found {}",
                        value.ty().long_name()
                    );
                }
            },
            Value::Dict(into) => match value {
                Value::Dict(dict) => {
                    into.extend(dict.iter().map(|(k, v)| (k.clone(), v.clone())));
                }
                Value::None => {}
                _ => {
                    bail!(
                        span,
                        "expected dictionary or none, found {}",
                        value.ty().long_name()
                    );
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
                    bail!(
                        span,
                        "expected arguments, array, dictionary, or none, found {}",
                        value.ty().long_name()
                    );
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
        instructions: &[u8],
        span: Span,
        vm: &mut VMState,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        debug_assert!(self.len as usize <= instructions.len());

        // SAFETY: The instruction pointer is always within the bounds of the
        // instruction list.
        // JUSTIFICATION: This avoids a bounds check on every scope.
        let instructions = unsafe {
            std::slice::from_raw_parts(instructions.as_ptr(), self.len as usize)
        };

        let defaults = vm.read(self.scope).at(span)?.clone();

        // Enter the scope within the vm.
        let joins = self.flags & 0b010 != 0;
        let content = self.flags & 0b100 != 0;

        let f = move || {
            let flow = vm.enter_scope(
                engine,
                &defaults,
                instructions,
                None,
                None,
                joins,
                content,
                span,
            )?;

            let joined = match flow {
                ControlFlow::Done(value) => value,
                ControlFlow::Break(value) => {
                    vm.state |= State::BREAKING;
                    value
                }
                ControlFlow::Continue(value) => {
                    vm.state |= State::CONTINUING;
                    value
                }
                ControlFlow::Return(value) => {
                    vm.state |= State::RETURNING;
                    value
                }
            };

            if let Some(out) = self.out.ok() {
                // Write the output to the output register.
                vm.write_one(out, joined).at(span)?;
            }

            vm.instruction_pointer += self.len as usize;

            Ok(())
        };

        // Stacker is broken on WASM.
        #[cfg(target_arch = "wasm32")]
        return f();

        #[cfg(not(target_arch = "wasm32"))]
        stacker::maybe_grow(32 * 1024, 2 * 1024 * 1024, f)
    }
}

impl Run for Jump {
    fn run(
        &self,
        _: &[u8],
        _: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Jump to the instruction.
        vm.instruction_pointer = self.instruction.0 as usize;

        Ok(())
    }
}

impl Run for JumpIf {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the condition.
        let condition = vm.read(self.condition).at(span)?;

        // Get the condition as a boolean.
        let Value::Bool(condition) = condition else {
            bail!(span, "expected boolean, found {}", condition.ty().long_name());
        };

        // Jump to the instruction if the condition is true.
        if *condition {
            vm.instruction_pointer = self.instruction.0 as usize;
        }

        Ok(())
    }
}

impl Run for JumpIfNot {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the condition.
        let condition = vm.read(self.condition).at(span)?;

        // Get the condition as a boolean.
        let Value::Bool(condition) = condition else {
            bail!(span, "expected boolean, found {}", condition.ty().long_name());
        };

        // Jump to the instruction if the condition is false.
        if !*condition {
            vm.instruction_pointer = self.instruction.0 as usize;
        }

        Ok(())
    }
}

impl Run for Select {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the condition.
        let condition = vm.read(self.condition).at(span)?;

        // Get the condition as a boolean.
        let Value::Bool(condition) = condition else {
            bail!(span, "expected boolean, found {}", condition.ty().long_name());
        };

        // Select the true value if the condition is true, otherwise select the
        // false value.
        let value = if *condition {
            vm.read(self.true_).at(span)?
        } else {
            vm.read(self.false_).at(span)?
        };

        // Write the value to the output.
        vm.write_one(self.out, value.clone()).at(span)?;

        Ok(())
    }
}

impl Run for Delimited {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the left delimiter, body, and right delimiter.
        let left: Content = vm.read(self.left).at(span)?.clone().display();
        let body: Content = vm.read(self.body).at(span)?.clone().display();
        let right: Content = vm.read(self.right).at(span)?.clone().display();

        // Make the value into a delimited.
        let value = LrElem::new(left + body + right);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl Run for Attach {
    fn run(
        &self,
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the base, top, and bottom.
        let base = vm.read(self.base).at(span)?;
        let top = vm.read(self.top).at(span)?;
        let bottom = vm.read(self.bottom).at(span)?;

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the numerator and denominator.
        let numerator = vm.read(self.numerator).at(span)?;
        let denominator = vm.read(self.denominator).at(span)?;

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the degree and radicand.
        let degree = vm.read(self.degree).at(span)?;
        let radicand = vm.read(self.radicand).at(span)?;

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the supplement.
        let supplement = self
            .supplement
            .ok()
            .map(|supplement| vm.read(supplement).at(span))
            .transpose()?;

        // Create the reference.
        let mut reference = RefElem::new(*vm.read(self.label).at(span)?);

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?;

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?;

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value and level.
        let value = vm.read(self.value).at(span)?;
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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?;

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value and number.
        let value = vm.read(self.value).at(span)?;
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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value and description.
        let value = vm.read(self.term).at(span)?;
        let description = vm.read(self.description).at(span)?;

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
        _: &[u8],
        span: Span,
        vm: &mut VMState,
        _: &mut Engine,
    ) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).at(span)?;

        // Make the value into an equation.
        let value = EquationElem::new(value.clone().cast().at(span)?);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}
