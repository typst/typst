use typst_syntax::ast::{self, AstNode};

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::vm::{OptionalReadable, Pointer, Readable};

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
        let if_body = compiler.section(engine, |compiler, engine| {
            self.if_body().compile_into(engine, compiler, output.clone())
        })?;

        let else_body = self
            .else_body()
            .map(|else_body| {
                compiler.section(engine, |compiler, engine| {
                    else_body.compile_into(engine, compiler, output.clone())
                })
            })
            .transpose()?
            .unwrap_or_else(|| {
                compiler
                    .section(engine, |compiler, _| {
                        if let Some(output) = &output {
                            compiler.copy(self.span(), Readable::none(), output);
                        }
                        Ok(())
                    })
                    .unwrap()
            });

        let condition = self.condition().compile(engine, compiler)?;

        // Compute the index of the true label:
        // + The current index in the bytecode.
        // + The length of the else body.
        // + The length of the jump opcode.
        let index = compiler.len() + else_body.len() + 1 + 1;
        let true_ = Pointer::new(index as u32);
        compiler.jump_if(self.span(), &condition, true_);
        compiler.extend(else_body);

        // Compute the index of the after label:
        // + The current index in the bytecode.
        // + The length of the jump opcode.
        // + The length of the if body.
        let index_after = compiler.len() + 1 + if_body.len();
        let after_ = Pointer::new(index_after as u32);
        compiler.jump_isr(self.span(), after_);
        compiler.extend(if_body);

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
        compiler.enter_indefinite(
            engine,
            true,
            output.as_ref().map(|w| w.as_writable()),
            false,
            |compiler, engine, _| {
                let reg = compiler.register().at(self.span())?;
                let condition = compiler.section(engine, |compiler, engine| {
                    self.condition().compile_into(
                        engine,
                        compiler,
                        Some(reg.clone().into()),
                    )
                })?;

                let body = compiler.section(engine, |compiler, engine| {
                    self.body().compile_into(
                        engine,
                        compiler,
                        if output.is_some() { Some(WritableGuard::Joined) } else { None },
                    )?;
                    compiler.flow();
                    Ok(())
                })?;

                // The index of the top label.
                let top_index = compiler.len();
                let top = Pointer::new(top_index as u32);

                // The index of the after label.
                // + The index of the top label.
                // + The length of the condition.
                // + The length of the jump-if-not opcode.
                // + The length of the body.
                // + The length of the jump opcode.
                let after_index = top_index + condition.len() + 1 + body.len() + 1;
                let after = Pointer::new(after_index as u32);
                compiler.extend(condition);
                compiler.jump_if_not(self.span(), reg.as_readable(), after);
                compiler.extend(body);
                compiler.jump_isr(self.span(), top);

                Ok(())
            },
            |compiler, _, len, out, scope| {
                compiler.while_(self.span(), scope, len as u32, 0b101, out);
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
                let top_index = compiler.len();
                let top = Pointer::new(top_index as u32);

                let pattern = self.pattern().compile(engine, compiler, true)?;
                if let PatternKind::Single(PatternItem::Simple(
                    span,
                    AccessPattern::Writable(writable),
                    _,
                )) = &pattern.kind
                {
                    compiler.next(*span, writable);
                } else {
                    let i = compiler.register().at(self.span())?;
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
                compiler.jump_isr(self.span(), top);

                Ok(())
            },
            |compiler, engine, len, out, scope| {
                let iterable = self.iter().compile(engine, compiler)?;
                compiler.iter(
                    self.iter().span(),
                    scope,
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
        let flow = value.is_some();
        compiler.return_(
            self.span(),
            value.map_or_else(OptionalReadable::none, |v| v.as_readable().into()),
        );

        if flow {
            // Force a flow after a return with a value.
            compiler.flow();
        }

        Ok(())
    }
}
