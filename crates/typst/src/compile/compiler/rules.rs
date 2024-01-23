use typst_syntax::ast::{self, AstNode};

use crate::compile::{Compile, Instruction, Register};
use crate::diag::{At, SourceResult};

impl Compile for ast::SetRule<'_> {
    fn compile(&self, compiler: &mut super::Compiler) -> SourceResult<Register> {
        let mut label = None;
        if let Some(expr) = self.condition() {
            let condition = expr.compile(compiler)?;
            let jmp_label = compiler.label();
            label = Some((condition, expr.span(), jmp_label));

            compiler.spans.push(expr.span());
            compiler
                .instructions
                .push(Instruction::JumpIfNot { condition, label: jmp_label });
        }

        let reg = compiler.reg().at(self.span())?;
        let target = self.target().compile(compiler)?;
        let args = self.args().compile(compiler)?;

        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::SetRule { target, args, result: reg });

        if let Some((condition, span, label)) = label {
            compiler.spans.push(span);
            compiler.instructions.push(Instruction::Label { label });

            // Add a select instruction to the end of the rule to ensure that the
            // result of the rule is always a value.
            compiler.spans.push(span);
            compiler.instructions.push(Instruction::Select {
                condition,
                lhs: reg,
                rhs: Register::NONE,
                target: reg,
            });
        }

        compiler.free(target);
        compiler.free(args);

        Ok(reg)
    }
}

impl Compile for ast::ShowRule<'_> {
    fn compile(&self, compiler: &mut super::Compiler) -> SourceResult<Register> {
        let selector = self.selector().map(|sel| sel.compile(compiler)).transpose()?;
        let transform = match self.transform() {
            ast::Expr::Set(set) => set.compile(compiler)?,
            other => other.compile(compiler)?,
        };

        let reg = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::ShowRule {
            selector,
            transform,
            result: reg,
        });

        compiler.free(transform);
        if let Some(selector) = selector {
            compiler.free(selector);
        }

        Ok(reg)
    }
}
