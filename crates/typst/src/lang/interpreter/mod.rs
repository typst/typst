mod joiner;
mod state;
mod read;
mod run;

use std::borrow::Cow;

use crate::diag::StrResult;
use crate::foundations::IntoValue;
use crate::foundations::Recipe;
use crate::foundations::SequenceElem;
use crate::foundations::Styles;
use crate::foundations::Value;

use super::compiled::CompiledCode;
use super::opcodes::Readable;
use super::operands::Register;

pub use self::state::*;
pub use self::joiner::*;
pub use self::read::*;
pub use self::run::*;

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

    /// Advance the instruction pointer.
    pub fn next(&mut self) {
        self.instruction_pointer += 1;
    }

    /// Jump to a specific instruction.
    pub fn jump(&mut self, instruction_pointer: usize) {
        self.instruction_pointer = instruction_pointer;
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
}
