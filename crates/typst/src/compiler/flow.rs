use typst_syntax::ast::{self, AstNode};

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::vm::{OptionalReadable, Readable, Writable};

use super::{
    AccessPattern, Compile, Compiler, Opcode, PatternCompile, PatternItem, PatternKind,
    ReadableGuard, WritableGuard,
};

impl Compile for ast::Conditional<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        let true_ = compiler.jump();
        let after = compiler.jump();

        let condition = self.condition().compile(engine, compiler)?;
        if let Some(else_body) = self.else_body() {
            compiler.isr(Opcode::jump_if(self.span(), &condition, true_));

            else_body.compile_into(engine, compiler, output.clone())?;

            compiler.isr(Opcode::jump(else_body.span(), after));

            compiler.isr(Opcode::jump_label(self.span(), compiler.scope_id(), true_));

            self.if_body().compile_into(engine, compiler, output)?;
        } else {
            compiler.isr(Opcode::jump_if_not(self.span(), &condition, after));

            if let Some(output) = output {
                let reg = self.if_body().compile(engine, compiler)?;
                compiler.isr(Opcode::select(
                    self.condition().span(),
                    &condition,
                    &reg,
                    Readable::none(),
                    &output,
                ));
            } else {
                self.if_body().compile_into(engine, compiler, None)?;
            }
        }

        compiler.isr(Opcode::jump_label(self.span(), compiler.scope_id(), after));

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Get an output register.
        let reg = compiler.register().at(self.span())?;

        // Compile into the register.
        self.compile_into(engine, compiler, Some(reg.clone().into()))?;

        // Return the register.
        Ok(reg.into())
    }
}

impl Compile for ast::WhileLoop<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        compiler.enter(
            self.span(),
            true,
            output.as_ref().map(|w| w.as_writable()),
            false,
            |compiler, _| {
                let top = compiler.jump();
                let after = compiler.jump();

                compiler.isr(Opcode::jump_label(self.span(), compiler.scope_id(), top));

                let condition = self.condition().compile(engine, compiler)?;
                compiler.isr(Opcode::jump_if_not(self.span(), &condition, after));

                self.body().compile_into(
                    engine,
                    compiler,
                    if output.is_some() { Some(WritableGuard::Joined) } else { None },
                )?;

                compiler.isr(Opcode::jump(self.span(), top));
                compiler.isr(Opcode::jump_label(self.span(), compiler.scope_id(), after));

                Ok(())
            },
        )
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Get an output register.
        let reg = compiler.register().at(self.span())?;

        // Compile into the register.
        self.compile_into(engine, compiler, Some(reg.clone().into()))?;

        // Return the register.
        Ok(reg.into())
    }
}

impl Compile for ast::ForLoop<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        compiler.enter_indefinite(
            engine,
            true,
            output.as_ref().map(|w| w.as_writable()),
            false,
            |compiler, engine, _| {
                let top = compiler.jump();
                compiler.isr(Opcode::jump_label(self.span(), compiler.scope_id(), top));

                let pattern = self.pattern().compile(engine, compiler, true)?;
                if let PatternKind::Single(PatternItem::Simple(
                    span,
                    AccessPattern::Writable(writable),
                    _,
                )) = &pattern.kind
                {
                    compiler.isr(Opcode::next(*span, writable));
                } else {
                    let i = compiler.register().at(self.span())?;
                    compiler.isr(Opcode::next(self.iter().span(), i.as_writeable()));

                    let pattern_id = compiler.pattern(pattern.as_vm_pattern());
                    compiler.isr(Opcode::destructure(
                        self.pattern().span(),
                        i.as_readable(),
                        pattern_id,
                    ));
                }

                self.body().compile_into(
                    engine,
                    compiler,
                    Some(WritableGuard::Joined),
                )?;
                compiler.isr(Opcode::jump(self.span(), top));

                Ok(())
            },
            |compiler, engine, len, _, scope| {
                let iterable = self.iter().compile(engine, compiler)?;
                compiler.isr(Opcode::iter(
                    self.iter().span(),
                    scope,
                    len as u32,
                    &iterable,
                    0b01,
                    Writable::joined(),
                ));
                Ok(())
            },
        )
    }
    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Get an output register.
        let reg = compiler.register().at(self.span())?;

        // Compile into the register.
        self.compile_into(engine, compiler, Some(reg.clone().into()))?;

        // Return the register.
        Ok(reg.into())
    }
}

impl Compile for ast::LoopBreak<'_> {
    type Output = ();
    type IntoOutput = ();

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        _: Self::Output,
    ) -> SourceResult<()> {
        self.compile(engine, compiler)
    }

    fn compile(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        if !compiler.in_loop() {
            bail!(self.span(), "cannot break outside of a loop");
        }

        compiler.isr(Opcode::break_(self.span()));

        Ok(())
    }
}

impl Compile for ast::LoopContinue<'_> {
    type Output = ();
    type IntoOutput = ();

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        _: Self::Output,
    ) -> SourceResult<()> {
        self.compile(engine, compiler)
    }

    fn compile(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        if !compiler.in_loop() {
            bail!(self.span(), "cannot continue outside of a loop");
        }

        compiler.isr(Opcode::continue_(self.span()));

        Ok(())
    }
}

impl Compile for ast::FuncReturn<'_> {
    type Output = ();
    type IntoOutput = ();

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        _: Self::Output,
    ) -> SourceResult<()> {
        self.compile(engine, compiler)
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        if !compiler.in_function() {
            bail!(self.span(), "cannot return outside of a function");
        }

        let value = self.body().map(|body| body.compile(engine, compiler)).transpose()?;
        compiler.isr(Opcode::return_(
            self.span(),
            value.map_or_else(OptionalReadable::none, |v| v.as_readable().into()),
        ));

        Ok(())
    }
}
