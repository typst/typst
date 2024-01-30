use typst_syntax::ast::{self, AstNode};

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::vm::Readable;

use super::{Access, Compile, Compiler, Opcode, ReadableGuard, WritableGuard};

impl Compile for ast::Binary<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = Option<ReadableGuard>;

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

        if matches!(self.op(), ast::BinOp::Or | ast::BinOp::And) {
            return compile_and_or(engine, compiler, self, output);
        }

        if matches!(
            self.op(),
            ast::BinOp::Assign
                | ast::BinOp::AddAssign
                | ast::BinOp::SubAssign
                | ast::BinOp::MulAssign
                | ast::BinOp::DivAssign
        ) {
            compiler.isr(Opcode::copy(self.span(), Readable::none(), &output));

            return compile_assign(engine, compiler, self);
        }

        let lhs = self.lhs().compile(engine, compiler)?;
        let rhs = self.rhs().compile(engine, compiler)?;

        match self.op() {
            ast::BinOp::Add => {
                compiler.isr(Opcode::add(self.span(), &lhs, &rhs, &output))
            }
            ast::BinOp::Sub => {
                compiler.isr(Opcode::sub(self.span(), &lhs, &rhs, &output))
            }
            ast::BinOp::Mul => {
                compiler.isr(Opcode::mul(self.span(), &lhs, &rhs, &output))
            }
            ast::BinOp::Div => {
                compiler.isr(Opcode::div(self.span(), &lhs, &rhs, &output))
            }
            ast::BinOp::Eq => compiler.isr(Opcode::eq(self.span(), &lhs, &rhs, &output)),
            ast::BinOp::Neq => {
                compiler.isr(Opcode::neq(self.span(), &lhs, &rhs, &output))
            }
            ast::BinOp::Lt => compiler.isr(Opcode::lt(self.span(), &lhs, &rhs, &output)),
            ast::BinOp::Leq => {
                compiler.isr(Opcode::leq(self.span(), &lhs, &rhs, &output))
            }
            ast::BinOp::Gt => compiler.isr(Opcode::gt(self.span(), &lhs, &rhs, &output)),
            ast::BinOp::Geq => {
                compiler.isr(Opcode::geq(self.span(), &lhs, &rhs, &output))
            }
            ast::BinOp::In => compiler.isr(Opcode::in_(self.span(), &lhs, &rhs, &output)),
            ast::BinOp::NotIn => {
                compiler.isr(Opcode::not_in(self.span(), &lhs, &rhs, &output))
            }
            ast::BinOp::And
            | ast::BinOp::Or
            | ast::BinOp::Assign
            | ast::BinOp::AddAssign
            | ast::BinOp::SubAssign
            | ast::BinOp::MulAssign
            | ast::BinOp::DivAssign => unreachable!(),
        }

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Assignments don't return anything.
        if matches!(
            self.op(),
            ast::BinOp::Assign
                | ast::BinOp::AddAssign
                | ast::BinOp::SubAssign
                | ast::BinOp::MulAssign
                | ast::BinOp::DivAssign
        ) {
            compile_assign(engine, compiler, self)?;
            return Ok(None);
        }

        let output = compiler.register().at(self.span())?;
        self.compile_into(engine, compiler, Some(output.clone().into()))?;
        Ok(Some(output.into()))
    }
}

fn compile_and_or(
    engine: &mut Engine,
    compiler: &mut Compiler,
    binary: &ast::Binary<'_>,
    output: WritableGuard,
) -> SourceResult<()> {
    let label = compiler.jump();

    // First we run the lhs.
    let lhs = binary.lhs().compile(engine, compiler)?;

    // Then we conditionally jump to the end.
    match binary.op() {
        ast::BinOp::Or => compiler.isr(Opcode::jump_if(binary.span(), &lhs, label)),
        ast::BinOp::And => compiler.isr(Opcode::jump_if_not(binary.span(), &lhs, label)),
        _ => unreachable!(),
    }

    // Then we run the rhs.
    let rhs = binary.rhs().compile(engine, compiler)?;

    // We add the jump label.
    compiler.isr(Opcode::jump_label(binary.span(), compiler.scope_id(), label));

    // Then, based on the result of the lhs, we either select the rhs to the output or the lhs.
    match binary.op() {
        ast::BinOp::Or => compiler.isr(Opcode::select(
            binary.span(),
            &lhs,
            Readable::bool(true),
            &rhs,
            &output,
        )),
        ast::BinOp::And => compiler.isr(Opcode::select(
            binary.span(),
            &lhs,
            &rhs,
            Readable::bool(false),
            &output,
        )),
        _ => unreachable!(),
    }

    Ok(())
}

fn compile_assign(
    engine: &mut Engine,
    compiler: &mut Compiler,
    binary: &ast::Binary<'_>,
) -> SourceResult<()> {
    let lhs = binary.lhs().access(engine, compiler, true)?;
    let rhs = binary.rhs().compile(engine, compiler)?;

    let access = compiler.access(lhs.as_vm_access());

    match binary.op() {
        ast::BinOp::Assign => compiler.isr(Opcode::assign(binary.span(), &rhs, access)),
        ast::BinOp::AddAssign => {
            compiler.isr(Opcode::add_assign(binary.span(), &rhs, access))
        }
        ast::BinOp::SubAssign => {
            compiler.isr(Opcode::sub_assign(binary.span(), &rhs, access))
        }
        ast::BinOp::MulAssign => {
            compiler.isr(Opcode::mul_assign(binary.span(), &rhs, access))
        }
        ast::BinOp::DivAssign => {
            compiler.isr(Opcode::div_assign(binary.span(), &rhs, access))
        }
        _ => unreachable!(),
    }

    Ok(())
}

impl Compile for ast::Unary<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        let Some(output) = output else {
            return Ok(());
        };

        let value = self.expr().compile(engine, compiler)?;

        match self.op() {
            ast::UnOp::Pos => compiler.isr(Opcode::pos(self.span(), &value, &output)),
            ast::UnOp::Neg => compiler.isr(Opcode::neg(self.span(), &value, &output)),
            ast::UnOp::Not => compiler.isr(Opcode::not(self.span(), &value, &output)),
        }

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
