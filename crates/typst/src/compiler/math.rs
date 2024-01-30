use ecow::eco_format;
use typst_syntax::ast::{self, AstNode};

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::NativeElement;
use crate::math::{AlignPointElem, PrimesElem};
use crate::text::TextElem;
use crate::vm::Constant;

use super::{Compile, Compiler, Opcode, ReadableGuard, WritableGuard};

impl Compile for ast::Math<'_> {
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
            false,
            output.as_ref().map(|w| w.as_writable()),
            true,
            |compiler, _| {
                let join = Some(WritableGuard::Joined);

                for expr in self.exprs() {
                    expr.compile_into(engine, compiler, join.clone())?;
                }

                Ok(())
            },
        )
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let output = compiler.register().at(self.span())?;
        self.compile_into(engine, compiler, Some(output.clone().into()))?;
        Ok(output.into())
    }
}

impl Compile for ast::MathIdent<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        // If we don't have an output, we do nothing.
        let Some(output) = output else {
            return Ok(());
        };

        let read = self.compile(engine, compiler)?;

        compiler.isr(Opcode::copy(self.span(), &read, &output));

        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let Some(value) = compiler.read(self.span(), self.get()).at(self.span())? else {
            bail!(self.span(), "unknown variable: {}", self.get())
        };

        Ok(value)
    }
}

impl Compile for ast::MathAlignPoint<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = Constant;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        // If we don't have an output, we do nothing.
        let Some(output) = output else {
            return Ok(());
        };

        let read = self.compile(engine, compiler)?;

        compiler.isr(Opcode::copy(self.span(), read, &output));

        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let value = AlignPointElem::new().pack().spanned(self.span());
        Ok(compiler.const_(value))
    }
}

impl Compile for ast::MathDelimited<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        // If we don't have an output, we do nothing.
        let Some(output) = output else {
            return Ok(());
        };

        let left = self.open().compile(engine, compiler)?;
        let body = self.body().compile(engine, compiler)?;
        let right = self.close().compile(engine, compiler)?;

        compiler.isr(Opcode::delimited(self.span(), &left, &body, &right, &output));

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let output = compiler.register().at(self.span())?;
        self.compile_into(engine, compiler, Some(output.clone().into()))?;
        Ok(output.into())
    }
}

impl Compile for ast::MathAttach<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        // If we don't have an output, we do nothing.
        let Some(output) = output else {
            return Ok(());
        };
        let base = self.base().compile(engine, compiler)?;

        let top = if let Some(top) =
            self.top().map(|value| value.compile(engine, compiler)).transpose()?
        {
            Some(top)
        } else if let Some(primes) = self
            .primes()
            .map(|value| value.compile(engine, compiler))
            .transpose()?
        {
            Some(ReadableGuard::Constant(primes))
        } else {
            None
        };

        let bottom = self
            .bottom()
            .map_or(Ok(None), |value| value.compile(engine, compiler).map(Some))?;

        compiler.isr(Opcode::attach(self.span(), &base, top, bottom, &output));

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let output = compiler.register().at(self.span())?;
        self.compile_into(engine, compiler, Some(output.clone().into()))?;
        Ok(output.into())
    }
}

impl Compile for ast::MathPrimes<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = Constant;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        // If we don't have an output, we do nothing.
        let Some(output) = output else {
            return Ok(());
        };

        let value = self.compile(engine, compiler)?;

        compiler.isr(Opcode::copy(self.span(), value, &output));

        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let value = PrimesElem::new(self.count()).pack().spanned(self.span());
        Ok(compiler.const_(value))
    }
}

impl Compile for ast::MathFrac<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        // If we don't have an output, we do nothing.
        let Some(output) = output else {
            return Ok(());
        };

        let num = self.num().compile(engine, compiler)?;
        let denom = self.denom().compile(engine, compiler)?;

        compiler.isr(Opcode::frac(self.span(), &num, &denom, &output));

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let output = compiler.register().at(self.span())?;
        self.compile_into(engine, compiler, Some(output.clone().into()))?;
        Ok(output.into())
    }
}

impl Compile for ast::MathRoot<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        // If we don't have an output, we do nothing.
        let Some(output) = output else {
            return Ok(());
        };

        let radicand = self.radicand().compile(engine, compiler)?;
        let degree = self
            .index()
            .map(|i| TextElem::packed(eco_format!("{i}")).spanned(self.span()))
            .map(|value| compiler.const_(value));

        compiler.isr(Opcode::root(self.span(), degree, &radicand, &output));

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let output = compiler.register().at(self.span())?;
        self.compile_into(engine, compiler, Some(output.clone().into()))?;
        Ok(output.into())
    }
}
