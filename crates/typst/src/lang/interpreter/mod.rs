mod access;
mod joiner;
mod pattern;
mod read;
mod run;
mod state;

use std::borrow::Cow;
use std::sync::Arc;

use comemo::Tracked;
use typst_syntax::Span;

use crate::diag::SourceResult;
use crate::diag::StrResult;
use crate::engine::Engine;
use crate::foundations::Context;
use crate::foundations::IntoValue;
use crate::foundations::Recipe;
use crate::foundations::SequenceElem;
use crate::foundations::Styles;
use crate::foundations::Value;
use crate::lang::closure::Param;
use crate::lang::compiled::CompiledParam;

use super::closure::Closure;
use super::compiled::CompiledClosure;
use super::compiled::CompiledCode;
use super::opcodes::Opcode;
use super::opcodes::Readable;
use super::operands::Register;

pub use self::joiner::*;
pub use self::read::*;
pub use self::run::*;
pub use self::state::*;

pub struct Vm<'a, 'b> {
    /// The current state of the VM.
    state: State,
    /// The output of the VM.
    output: Option<Readable>,
    /// The current instruction pointer.
    instruction_pointer: usize,
    /// The joined values.
    joined: Option<Joiner>,
    /// The registers.
    registers: &'b mut [Cow<'a, Value>],
    /// The code being executed.
    code: &'a CompiledCode,
}

impl<'a, 'b> Vm<'a, 'b> {
    /// Creates a new VM that displays the output.
    pub fn display(registers: &'a mut [Cow<'b, Value>], code: &'a CompiledCode) -> Self {
        Self {
            state: State::empty().display(true),
            output: None,
            instruction_pointer: 0,
            joined: None,
            registers,
            code,
        }
    }

    /// Creates a new VM that does not display the output.
    pub fn new(registers: &'a mut [Cow<'b, Value>], code: &'a CompiledCode) -> Self {
        Self {
            state: State::empty(),
            output: None,
            instruction_pointer: 0,
            joined: None,
            registers,
            code,
        }
    }
}

impl<'a> Vm<'a, '_> {
    /// Read a value from the VM.
    pub fn read<'b, T: Read>(&'b self, readable: T) -> T::Output<'a, 'b> {
        readable.read(self)
    }

    /// Read a register from the VM.
    pub fn read_register<'b>(&'b self, register: Register) -> &'b Cow<'a, Value> {
        &self.registers[register.0 as usize]
    }

    /// Take a register from the VM.
    pub fn take(&mut self, register: Register) -> Cow<'a, Value> {
        std::mem::take(&mut self.registers[register.0 as usize])
    }

    /// Write a value to the VM, returning a mutable reference to the value.
    pub fn write(&mut self, writable: impl Write) -> &mut Value {
        writable.write(self)
    }

    /// Write a value to the VM.
    pub fn write_one(
        &mut self,
        writable: impl Write,
        value: impl IntoValue,
    ) -> StrResult<()> {
        writable.write_one(self, value)
    }

    pub fn write_borrowed(&mut self, reg: Register, value: &'a Value) {
        self.registers[reg.0 as usize] = Cow::Borrowed(value);
    }

    /// Advance the instruction pointer.
    pub fn next(&mut self) {
        self.instruction_pointer += 1;
    }

    /// Jump to a specific instruction.
    pub fn jump(&mut self, instruction_pointer: usize) {
        self.instruction_pointer = instruction_pointer;
    }

    /// Bump the instruction pointer by a specific amount.
    pub fn bump(&mut self, amount: usize) {
        self.instruction_pointer += amount;
    }

    /// Get the current instruction pointer.
    pub fn instruction_pointer(&self) -> usize {
        self.instruction_pointer
    }

    /// Join a value to the current joining state.
    pub fn join(&mut self, value: impl IntoValue) -> StrResult<()> {
        // Convert the value to a display value if we are in display mode.
        let value = value.into_value();

        // We don't join `None`.
        if value.is_none() {
            return Ok(());
        }

        if let Some(joiner) = self.joined.take() {
            self.joined = Some(joiner.join(value)?);
        } else if self.state.is_display() {
            self.joined =
                Some(Joiner::Display(SequenceElem::new(vec![value.display().into()])));
        } else {
            self.joined = Some(Joiner::Value(value));
        }

        Ok(())
    }

    /// Applies a styling to the current joining state.
    pub fn styled(&mut self, styles: Styles) -> StrResult<()> {
        if let Some(joiner) = self.joined.take() {
            self.joined = Some(joiner.styled(styles));
        } else {
            self.joined = Some(Joiner::Styled {
                parent: None,
                content: SequenceElem::new(vec![]),
                styles,
            });
        }

        Ok(())
    }

    /// Applies a recipe to the current joining state.
    pub fn recipe(&mut self, recipe: Recipe) -> StrResult<()> {
        if let Some(joiner) = self.joined.take() {
            self.joined = Some(joiner.recipe(recipe));
        } else {
            self.joined = Some(Joiner::Recipe {
                parent: None,
                content: SequenceElem::new(vec![]),
                recipe: Box::new(recipe),
            });
        }

        Ok(())
    }

    /// Instantiate a closure.
    #[typst_macros::time(name = "instantiate closure", span = closure.span())]
    pub fn instantiate(
        &self,
        closure: &'a CompiledClosure,
    ) -> SourceResult<Cow<'a, Closure>> {
        let closure = match closure {
            CompiledClosure::Closure(closure) => closure,
            CompiledClosure::Instanciated(closure) => {
                return Ok(Cow::Borrowed(closure));
            }
        };

        // Load the default values for the parameters.
        let arg_count = closure.params.as_ref().map_or(0, |p| p.len());
        let mut params = Vec::with_capacity(arg_count);
        for param in closure.params.iter().flat_map(|p| p.iter()) {
            match param {
                CompiledParam::Pos(target, pos) => {
                    params.push((Some(*target), Param::Pos(pos.resolve())))
                }
                CompiledParam::Named { target, name, default, .. } => {
                    params.push((
                        Some(*target),
                        Param::Named {
                            name: name.resolve(),
                            default: self.read(*default).cloned(),
                        },
                    ));
                }
                CompiledParam::Sink(span, target, name) => {
                    params.push((*target, Param::Sink(*span, name.resolve())));
                }
            }
        }

        // Load the captured values.
        let capture_count = closure.captures.as_ref().map_or(0, |c| c.len());
        let mut captures = Vec::with_capacity(capture_count);
        for capture in closure.captures.iter().flat_map(|c| c.iter()) {
            captures.push((capture.register, self.read(capture.readable).clone()));
        }

        Ok(Cow::Owned(Closure::new(Arc::clone(closure), params, captures)))
    }

    /// Enter a new scope.
    #[typst_macros::time(name = "enter scope", span = spans[0])]
    pub fn enter_scope<'b>(
        &'b mut self,
        engine: &mut Engine,
        instructions: &'b [Opcode],
        spans: &'b [Span],
        context: Tracked<Context>,
        iterator: Option<&mut dyn Iterator<Item = Value>>,
        mut output: Option<Readable>,
        content: bool,
        looping: bool,
    ) -> SourceResult<ControlFlow> {
        let mut state =
            State::empty().loop_(looping || iterator.is_some()).display(content);

        let mut joiner = None;
        let mut instruction_pointer = 0;

        std::mem::swap(&mut self.state, &mut state);
        std::mem::swap(&mut self.output, &mut output);
        std::mem::swap(&mut self.joined, &mut joiner);
        std::mem::swap(&mut self.instruction_pointer, &mut instruction_pointer);

        let out = run(engine, self, instructions, spans, context, iterator)?;

        std::mem::swap(&mut self.state, &mut state);
        std::mem::swap(&mut self.output, &mut output);
        std::mem::swap(&mut self.joined, &mut joiner);
        std::mem::swap(&mut self.instruction_pointer, &mut instruction_pointer);

        Ok(out)
    }
}

pub fn run<'a: 'b, 'b, 'c>(
    engine: &mut Engine,
    vm: &mut Vm<'a, 'c>,
    instructions: &'b [Opcode],
    spans: &'b [Span],
    context: Tracked<Context>,
    mut iterator: Option<&mut dyn Iterator<Item = Value>>,
) -> SourceResult<ControlFlow> {
    fn next<'a: 'b, 'b, 'c>(
        vm: &mut Vm<'a, 'c>,
        instructions: &'b [Opcode],
    ) -> Option<&'b Opcode> {
        if vm.instruction_pointer == instructions.len() {
            vm.state.done();
            return None;
        }

        debug_assert!(vm.instruction_pointer + 1 <= instructions.len());
        Some(&instructions[vm.instruction_pointer])
    }

    while vm.state.is_running() {
        let Some(opcode) = next(vm, instructions) else {
            vm.state.done();
            break;
        };

        let idx = vm.instruction_pointer;

        opcode.run(
            &instructions,
            &spans,
            spans[idx],
            vm,
            engine,
            context,
            iterator.as_mut().map_or(None, |p| Some(&mut **p)),
        )?;

        if matches!(opcode, Opcode::Flow) && !matches!(vm.state.flow, Flow::None) {
            if vm.state.is_looping() {
                match vm.state.flow {
                    Flow::None => {}
                    Flow::Continue => {
                        vm.instruction_pointer = 0;
                        vm.state.flow = Flow::None;
                        continue;
                    }
                    Flow::Break | Flow::Return(_) | Flow::Done => {
                        vm.state.set_done();
                        break;
                    }
                }
            } else {
                match vm.state.flow {
                    Flow::None => {}
                    Flow::Continue | Flow::Break | Flow::Return(_) | Flow::Done => {
                        break;
                    }
                }
            }
        }
    }

    let output = if let Some(readable) = vm.output {
        match readable {
            Readable::Reg(reg) => Some(vm.take(reg).into_owned()),
            Readable::None => Some(Value::None),
            Readable::Bool(b) => Some(Value::Bool(b)),
            _ => Some(vm.read(readable).clone()),
        }
    } else if let Some(joined) = vm.joined.take() {
        Some(joined.collect(engine, context)?)
    } else {
        None
    };

    Ok(match vm.state.flow {
        Flow::Break => ControlFlow::Break(output.unwrap_or(Value::None)),
        Flow::Continue => ControlFlow::Continue(output.unwrap_or(Value::None)),
        Flow::Return(forced) => {
            ControlFlow::Return(output.unwrap_or(Value::None), forced)
        }
        _ => ControlFlow::Done(output.unwrap_or(Value::None)),
    })
}
