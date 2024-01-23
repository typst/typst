use typst_syntax::ast::{self, AstNode};

use crate::{
    compile::AssignOp,
    diag::{At, SourceResult},
};

use super::{Access, Compile, Compiler, Instruction, Register};

impl Compile for ast::Binary<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let mut target = Register::NONE;
        let isr = match self.op() {
            ast::BinOp::Add => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::Add { lhs, rhs, target }
            }
            ast::BinOp::Sub => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::Sub { lhs, rhs, target }
            }
            ast::BinOp::Mul => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::Mul { lhs, rhs, target }
            }
            ast::BinOp::Div => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::Div { lhs, rhs, target }
            }
            ast::BinOp::And => {
                target = compiler.reg().at(self.span())?;

                let label = compiler.label();

                let lhs = self.lhs().compile(compiler)?;

                // Bypass the rhs if the lhs is false.
                compiler.spans.push(self.span());
                compiler
                    .instructions
                    .push(Instruction::JumpIfNot { condition: lhs, label });

                let rhs = self.rhs().compile(compiler)?;

                // And the lhs and rhs.
                compiler.spans.push(self.span());
                compiler.instructions.push(Instruction::And { lhs, rhs, target });

                // Where to jump to if the lhs is false.
                compiler.spans.push(self.span());
                compiler.instructions.push(Instruction::Label { label });

                // In case we didn't run the And instruction, we
                // still need to set the target to false.
                compiler.spans.push(self.span());
                compiler.instructions.push(Instruction::Select {
                    condition: lhs,
                    lhs: target,
                    rhs: lhs,
                    target,
                });

                compiler.free(lhs);
                compiler.free(rhs);

                return Ok(target);
            }
            ast::BinOp::Or => {
                target = compiler.reg().at(self.span())?;

                let label = compiler.label();

                let lhs = self.lhs().compile(compiler)?;

                // Bypass the rhs if the lhs is false.
                compiler.spans.push(self.span());
                compiler
                    .instructions
                    .push(Instruction::JumpIf { condition: lhs, label });

                let rhs = self.rhs().compile(compiler)?;

                // And the lhs and rhs.
                compiler.spans.push(self.span());
                compiler.instructions.push(Instruction::Or { lhs, rhs, target });

                // Where to jump to if the lhs is false.
                compiler.spans.push(self.span());
                compiler.instructions.push(Instruction::Label { label });

                // In case we didn't run the And instruction, we
                // still need to set the target to false.
                compiler.spans.push(self.span());
                compiler.instructions.push(Instruction::Select {
                    condition: lhs,
                    lhs,
                    rhs: target,
                    target,
                });

                compiler.free(lhs);
                compiler.free(rhs);

                return Ok(target);
            }
            ast::BinOp::Eq => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::Eq { lhs, rhs, target }
            }
            ast::BinOp::Neq => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::Neq { lhs, rhs, target }
            }
            ast::BinOp::Lt => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::Lt { lhs, rhs, target }
            }
            ast::BinOp::Leq => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::Leq { lhs, rhs, target }
            }
            ast::BinOp::Gt => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::Gt { lhs, rhs, target }
            }
            ast::BinOp::Geq => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::Geq { lhs, rhs, target }
            }
            ast::BinOp::In => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::In { lhs, rhs, target }
            }
            ast::BinOp::NotIn => {
                target = compiler.reg().at(self.span())?;

                let lhs = self.lhs().compile(compiler)?;
                let rhs = self.rhs().compile(compiler)?;

                compiler.free(lhs);
                compiler.free(rhs);

                Instruction::NotIn { lhs, rhs, target }
            }
            ast::BinOp::Assign => {
                let lhs = self.lhs().access(compiler, true)?;
                let rhs = self.rhs().compile(compiler)?;
                lhs.free(compiler);
                compiler.free(rhs);
                let access = compiler.access(lhs);
                Instruction::Assign { access, value: rhs, op: AssignOp::None }
            }
            ast::BinOp::AddAssign => {
                let lhs = self.lhs().access(compiler, true)?;
                let rhs = self.rhs().compile(compiler)?;
                lhs.free(compiler);
                compiler.free(rhs);
                let access = compiler.access(lhs);
                Instruction::Assign { access, value: rhs, op: AssignOp::Add }
            }
            ast::BinOp::SubAssign => {
                let lhs = self.lhs().access(compiler, true)?;
                let rhs = self.rhs().compile(compiler)?;
                lhs.free(compiler);
                compiler.free(rhs);
                let access = compiler.access(lhs);
                Instruction::Assign { access, value: rhs, op: AssignOp::Sub }
            }
            ast::BinOp::MulAssign => {
                let lhs = self.lhs().access(compiler, true)?;
                let rhs = self.rhs().compile(compiler)?;
                lhs.free(compiler);
                compiler.free(rhs);
                let access = compiler.access(lhs);
                Instruction::Assign { access, value: rhs, op: AssignOp::Mul }
            }
            ast::BinOp::DivAssign => {
                let lhs = self.lhs().access(compiler, true)?;
                let rhs = self.rhs().compile(compiler)?;
                lhs.free(compiler);
                compiler.free(rhs);
                let access = compiler.access(lhs);
                Instruction::Assign { access, value: rhs, op: AssignOp::Div }
            }
        };

        compiler.spans.push(self.span());
        compiler.instructions.push(isr);

        Ok(target)
    }
}

impl Compile for ast::Unary<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let target = self.expr().compile(compiler)?;

        compiler.spans.push(self.span());
        compiler.instructions.push(match self.op() {
            ast::UnOp::Pos => Instruction::Pos { target },
            ast::UnOp::Neg => Instruction::Neg { target },
            ast::UnOp::Not => Instruction::Not { target },
        });

        Ok(target)
    }
}
