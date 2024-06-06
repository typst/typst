use typst_syntax::ast::{self, AstNode};

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::lang::operands::Readable;

use super::{Compile, CompileAccess, Compiler, WritableGuard};

impl Compile for ast::Binary<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        // Due to short-circuiting, we compile them differently.
        if matches!(self.op(), ast::BinOp::Or | ast::BinOp::And) {
            return compile_and_or(self, compiler, engine, output);
        }

        // For assignments, we compile them separately and they produce no output.
        if matches!(
            self.op(),
            ast::BinOp::Assign
                | ast::BinOp::AddAssign
                | ast::BinOp::SubAssign
                | ast::BinOp::MulAssign
                | ast::BinOp::DivAssign
        ) {
            compiler.copy(self.span(), Readable::none(), output);

            return compile_assign(self, compiler, engine);
        }

        let lhs = self.lhs().compile_to_readable(compiler, engine)?;
        let rhs = self.rhs().compile_to_readable(compiler, engine)?;

        match self.op() {
            ast::BinOp::Add => compiler.add(self.span(), lhs, rhs, output),
            ast::BinOp::Sub => compiler.sub(self.span(), lhs, rhs, output),
            ast::BinOp::Mul => compiler.mul(self.span(), lhs, rhs, output),
            ast::BinOp::Div => compiler.div(self.span(), lhs, rhs, output),
            ast::BinOp::Eq => compiler.eq(self.span(), lhs, rhs, output),
            ast::BinOp::Neq => compiler.neq(self.span(), lhs, rhs, output),
            ast::BinOp::Lt => compiler.lt(self.span(), lhs, rhs, output),
            ast::BinOp::Leq => compiler.leq(self.span(), lhs, rhs, output),
            ast::BinOp::Gt => compiler.gt(self.span(), lhs, rhs, output),
            ast::BinOp::Geq => compiler.geq(self.span(), lhs, rhs, output),
            ast::BinOp::In => compiler.in_(self.span(), lhs, rhs, output),
            ast::BinOp::NotIn => compiler.not_in(self.span(), lhs, rhs, output),
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
}

impl Compile for ast::Unary<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let operand = self.expr().compile_to_readable(compiler, engine)?;

        match self.op() {
            ast::UnOp::Pos => compiler.pos(self.span(), operand, output),
            ast::UnOp::Not => compiler.not(self.span(), operand, output),
            ast::UnOp::Neg => compiler.neg(self.span(), operand, output),
        }

        Ok(())
    }
}

fn compile_and_or(
    binary: &ast::Binary<'_>,
    compiler: &mut Compiler,
    engine: &mut Engine,
    output: WritableGuard,
) -> SourceResult<()> {
    // First we run the lhs.
    let lhs = binary.lhs().compile_to_readable(compiler, engine)?;

    // Then we create a marker for the end of the operation.
    let marker = compiler.marker();

    // Then we conditionally jump to the end.
    match binary.op() {
        ast::BinOp::Or => compiler.jump_if(binary.span(), lhs.clone(), marker),
        ast::BinOp::And => compiler.jump_if_not(binary.span(), lhs.clone(), marker),
        _ => unreachable!(),
    }

    // Then we run the rhs.
    let rhs = binary.rhs().compile_to_readable(compiler, engine)?;

    // Add the marker.
    compiler.mark(binary.span(), marker);

    // Then, based on the result of the lhs, we either select the rhs to the output or the lhs.
    match binary.op() {
        ast::BinOp::Or => {
            compiler.select(binary.span(), lhs, Readable::bool(true), rhs, output)
        }
        ast::BinOp::And => {
            compiler.select(binary.span(), lhs, rhs, Readable::bool(false), output)
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn compile_assign(
    binary: &ast::Binary<'_>,
    compiler: &mut Compiler,
    engine: &mut Engine,
) -> SourceResult<()> {
    let lhs = binary.lhs().access(compiler, engine, true)?;
    let rhs = binary.rhs().compile_to_readable(compiler, engine)?;

    let access = compiler.access(lhs);
    if matches!(binary.op(), ast::BinOp::Assign) {
        compiler.assign(binary.lhs().span(), rhs, access);
        return Ok(());
    }

    let lhs_span = compiler.span(binary.lhs().span());
    match binary.op() {
        ast::BinOp::AddAssign => compiler.add_assign(binary.span(), rhs, lhs_span, access),
        ast::BinOp::SubAssign => compiler.sub_assign(binary.span(), rhs, lhs_span, access),
        ast::BinOp::MulAssign => compiler.mul_assign(binary.span(), rhs, lhs_span, access),
        ast::BinOp::DivAssign => compiler.div_assign(binary.span(), rhs, lhs_span, access),
        _ => unreachable!(),
    }

    Ok(())
}
