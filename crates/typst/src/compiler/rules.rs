use typst_syntax::ast::{self, AstNode};

use crate::{
    diag::{At, SourceResult},
    engine::Engine,
    vm::Readable,
};

use super::{Compile, Opcode, ReadableGuard, WritableGuard};

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
            let condition = expr.compile(engine, compiler)?;
            let jmp_label = compiler.jump();

            compiler.isr(Opcode::jump_if_not(
                expr.span(),
                condition.as_readable(),
                jmp_label,
            ));

            let reg = compiler.register().at(self.span())?;

            let target = self.target().compile(engine, compiler)?;
            let args = self.args().compile(engine, compiler)?;

            compiler.isr(Opcode::set(self.span(), &target, &args, reg.as_writeable()));

            compiler.isr(Opcode::jump_label(expr.span(), jmp_label));
            compiler.isr(Opcode::select(
                expr.span(),
                &condition,
                reg.as_readable(),
                Readable::none(),
                &output,
            ))
        } else {
            let target = self.target().compile(engine, compiler)?;
            let args = self.args().compile(engine, compiler)?;

            compiler.isr(Opcode::set(self.span(), &target, &args, &output));
        }

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Get an output register.
        let reg = compiler.register().at(self.span())?;

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

        compiler.isr(Opcode::show(self.span(), selector, &transform, &output));

        todo!()
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Get an output register.
        let reg = compiler.register().at(self.span())?;

        // Compile into the register.
        self.compile_into(engine, compiler, Some(reg.clone().into()))?;

        // Return the register.
        Ok(reg.into())
    }
}
