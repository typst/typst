use ecow::{eco_format, eco_vec};
use typst_syntax::ast::{self, AstNode};

use crate::diag::{error, At, SourceResult};
use crate::foundations::{IntoValue, NativeElement};
use crate::math::{AlignPointElem, PrimesElem};
use crate::text::TextElem;

use super::{Compile, Compiler, Instruction, Register};

impl Compile for ast::Math<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        // The expressions we will compile.
        let exprs = self.exprs();

        // We push a join group.
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::JoinGroup {
            capacity: exprs.size_hint().1.unwrap_or_else(|| exprs.size_hint().0) as u16,
        });

        for expr in exprs {
            let expr = expr.compile_display(compiler)?;
            if !expr.is_none() {
                compiler.spans.push(self.span());
                compiler.instructions.push(Instruction::Join { value: expr });
                compiler.free(expr);
            }
        }

        // We allocate a new register for the result of the math expression.
        let res = compiler.reg().at(self.span())?;

        // We pop the join group.
        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::PopGroup { target: res, content: true });

        Ok(res)
    }
}

impl Compile for ast::MathIdent<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let reg = compiler.reg().at(self.span())?;
        let isr = compiler.local_ref_in_math(self.get(), reg).ok_or_else(|| {
            eco_vec![error!(self.span(), "unknown identifier: `{}`", self.get())]
        })?;

        compiler.instructions.push(isr);
        compiler.spans.push(self.span());

        Ok(reg)
    }
}

impl Compile for ast::MathAlignPoint<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = AlignPointElem::new().pack().spanned(self.span());
        let value = compiler.const_(value.into_value());
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::MathDelimited<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let left = self.open().compile_display(compiler)?;
        let body = self.body().compile_display(compiler)?;
        let right = self.close().compile_display(compiler)?;

        let res = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Delimited {
            left,
            body,
            right,
            target: res,
        });

        compiler.free(left);
        compiler.free(body);
        compiler.free(right);

        Ok(res)
    }
}

impl Compile for ast::MathAttach<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let base = self.base().compile_display(compiler)?;

        let top = if let Some(top) =
            self.top().map(|value| value.compile_display(compiler))
        {
            top?
        } else if let Some(primes) = self.primes().map(|value| value.compile(compiler)) {
            primes?
        } else {
            Register::NONE
        };

        let bottom = self
            .bottom()
            .map_or(Ok(Register::NONE), |value| value.compile_display(compiler))?;

        let res = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Attach {
            base,
            top,
            bottom,
            target: res,
        });

        compiler.free(base);
        compiler.free(top);
        compiler.free(bottom);

        Ok(res)
    }
}

impl Compile for ast::MathPrimes<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = compiler.const_(
            PrimesElem::new(self.count()).pack().spanned(self.span()).into_value(),
        );
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::MathFrac<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let numerator = self.num().compile_display(compiler)?;
        let denominator = self.denom().compile_display(compiler)?;

        let res = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Frac {
            numerator,
            denominator,
            target: res,
        });

        compiler.free(numerator);
        compiler.free(denominator);

        Ok(res)
    }
}

impl Compile for ast::MathRoot<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let degree = self
            .index()
            .map(|i| {
                TextElem::packed(eco_format!("{i}")).spanned(self.span()).into_value()
            })
            .map(|i| compiler.const_(i));
        let radicand = self.radicand().compile_display(compiler)?;

        let res = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::Root { degree, radicand, target: res });

        compiler.free(radicand);

        Ok(res)
    }
}
