mod assign;
mod conditional;
mod flow;
mod markup;
mod math;
mod operators;
mod styling;
mod values;

use typst_syntax::Span;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::Value;
use crate::lang::opcodes::Opcode;

use super::Vm;

/// Runs an individual opcode, while giving it full access to the execution state.
pub trait Run {
    /// Runs an opcode.
    fn run(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut Vm,
        engine: &mut Engine,
        iterator: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()>;
}

/// Runs an individual opcode, while giving it limited access to the execution state.
pub trait SimpleRun {
    /// Runs an opcode.
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()>;
}

impl<T: SimpleRun> Run for T {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut Vm,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        <T as SimpleRun>::run(self, span, vm, engine)
    }
}
