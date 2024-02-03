use typst_syntax::ast::{self, AstNode};

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::vm::Readable;

use super::{Compile, ReadableGuard, WritableGuard};

impl Compile for ast::SetRule<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        let Some(output) = output else {
            return Ok(());
        };

        if let Some(expr) = self.condition() {
            // Compile the condition.
            let condition = expr.compile(engine, compiler)?;

            // Create the jump marker.
            let else_ = compiler.marker();
            let end = compiler.marker();

            // Create the jump.
            compiler.jump_if_not(expr.span(), condition.as_readable(), else_);

            // Compile the set.
            let target = self.target().compile(engine, compiler)?;
            let args = self.args().compile(engine, compiler)?;
            compiler.set(self.span(), &target, &args, &output);

            // Jump to the end.
            compiler.jump(expr.span(), end);

            // Compile the else body.
            compiler.mark(expr.span(), else_);
            compiler.copy(expr.span(), Readable::none(), &output);

            // Mark the end.
            compiler.mark(expr.span(), end);
        } else {
            let target = self.target().compile(engine, compiler)?;
            let args = self.args().compile(engine, compiler)?;

            compiler.set(self.span(), &target, &args, &output);
        }

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Get an output register.
        let reg = compiler.register();

        // Compile into the register.
        self.compile_into(engine, compiler, Some(reg.clone().into()))?;

        // Return the register.
        Ok(reg.into())
    }
}

impl Compile for ast::ShowRule<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        let Some(output) = output else {
            return Ok(());
        };

        let selector =
            self.selector().map(|sel| sel.compile(engine, compiler)).transpose()?;
        let transform = match self.transform() {
            ast::Expr::Set(set) => set.compile(engine, compiler)?,
            other => other.compile(engine, compiler)?,
        };

        compiler.show(self.span(), selector, &transform, &output);

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Get an output register.
        let reg = compiler.register();

        // Compile into the register.
        self.compile_into(engine, compiler, Some(reg.clone().into()))?;

        // Return the register.
        Ok(reg.into())
    }
}
