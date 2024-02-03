use typst_syntax::ast::{self, AstNode};

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::vm::{OptionalReadable, Readable};

use super::{
    AccessPattern, Compile, Compiler, PatternCompile, PatternItem, PatternKind,
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
        // Compile the condition
        let condition = self.condition().compile(engine, compiler)?;

        // Create the jump labels
        let if_ = compiler.marker();
        let end = compiler.marker();

        // Create the conditonal jump
        compiler.jump_if(self.span(), &condition, if_);

        // Compile the else body
        if let Some(else_body) = self.else_body() {
            else_body.compile_into(engine, compiler, output.clone())?;
        } else if let Some(output) = &output {
            compiler.copy(self.span(), Readable::none(), output);
        }

        // Jump to the end
        compiler.jump(self.span(), end);

        // Compile the if body
        compiler.mark(self.span(), if_);
        self.if_body().compile_into(engine, compiler, output)?;

        // Mark the end
        compiler.mark(self.span(), end);

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Get an output register.
        let reg = compiler.register();

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
        compiler.enter_indefinite(
            engine,
            true,
            output.as_ref().map(|w| w.as_writable()),
            false,
            |compiler, engine, _| {
                // Create the jump labels
                let top = compiler.marker();
                let end = compiler.marker();

                // Mark the top
                compiler.mark(self.span(), top);

                // Compile the condition
                let condition = self.condition().compile(engine, compiler)?;

                // Create the conditonal jump
                compiler.jump_if_not(self.span(), &condition, end);

                // Compile the while body
                self.body().compile_into(
                    engine,
                    compiler,
                    if output.is_some() { Some(WritableGuard::Joined) } else { None },
                )?;
                compiler.flow();

                // Jump to the top
                compiler.jump(self.span(), top);

                // Mark the end
                compiler.mark(self.span(), end);

                Ok(())
            },
            |compiler, _, len, out| {
                compiler.while_(self.span(), len as u32, 0b101, out);
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
        let reg = compiler.register();

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
                let pattern = self.pattern().compile(engine, compiler, true)?;
                if let PatternKind::Single(PatternItem::Simple(
                    span,
                    AccessPattern::Writable(writable),
                    _,
                )) = &pattern.kind
                {
                    compiler.next(*span, writable);
                } else {
                    let i = compiler.register();
                    compiler.next(self.iter().span(), i.as_writeable());

                    let pattern_id = compiler.pattern(pattern.as_vm_pattern());
                    compiler.destructure(
                        self.pattern().span(),
                        i.as_readable(),
                        pattern_id,
                    );
                }

                self.body().compile_into(
                    engine,
                    compiler,
                    Some(WritableGuard::Joined),
                )?;
                compiler.flow();
                compiler.jump_top(self.span());

                Ok(())
            },
            |compiler, engine, len, out| {
                let iterable = self.iter().compile(engine, compiler)?;
                compiler.iter(
                    self.iter().span(),
                    len as u32,
                    &iterable,
                    0b101,
                    out,
                );
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
        let reg = compiler.register();

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
            bail!(self.span(), "cannot break outside of loop");
        }

        compiler.break_(self.span());

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
            bail!(self.span(), "cannot continue outside of loop");
        }

        compiler.continue_(self.span());

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
            bail!(self.span(), "cannot return outside of function");
        }

        let value = self.body().map(|body| body.compile(engine, compiler)).transpose()?;
        compiler.return_(
            self.span(),
            value.map_or_else(OptionalReadable::none, |v| v.as_readable().into()),
        );

        Ok(())
    }
}
