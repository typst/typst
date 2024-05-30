use comemo::Tracked;
use typst_syntax::Span;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{Context, Value};
use crate::lang::{opcodes::*, ops};

use super::Vm;

pub trait Run {
    /// Runs an opcode.
    fn run(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut Vm,
        engine: &mut Engine,
        context: Tracked<Context>,
        iterator: Option<&mut dyn Iterator<Item = Value>>
    ) -> SourceResult<()>;
}

impl Run for Add {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut Vm,
        _: &mut Engine,
        _: Tracked<Context>,
        _: Option<&mut dyn Iterator<Item = Value>>
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
