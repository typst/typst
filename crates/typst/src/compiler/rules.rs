use typst_syntax::ast::{self, AstNode};

use crate::{
    diag::{At, SourceResult},
    engine::Engine,
    vm::{Pointer, Readable},
};

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
            let condition = expr.compile(engine, compiler)?;

            let target = compiler.register().at(self.span())?;
            let args = compiler.register().at(self.span())?;
            let set = compiler.section(engine, |compiler, engine| {
                self.target().compile_into(
                    engine,
                    compiler,
                    Some(target.clone().into()),
                )?;
                self.args()
                    .compile_into(engine, compiler, Some(args.clone().into()))?;
                Ok(())
            })?;

            let jump_index = compiler.len() + set.len() + 2;
            let jump_label = Pointer::new(jump_index as u32);
            compiler.jump_if_not(expr.span(), condition.as_readable(), jump_label);

            let reg = compiler.register().at(self.span())?;
            compiler.set(
                self.span(),
                target.as_readable(),
                args.as_readable(),
                reg.as_writeable(),
            );

            compiler.select(
                expr.span(),
                &condition,
                reg.as_readable(),
                Readable::none(),
                &output,
            );
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

        compiler.show(self.span(), selector, &transform, &output);

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
