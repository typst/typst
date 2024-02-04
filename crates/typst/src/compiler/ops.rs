use typst_syntax::ast::{self, AstNode};

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::vm::Readable;

use super::{Access, Compile, Compiler, ReadableGuard, WritableGuard};

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
            compiler.copy(self.span(), Readable::none(), &output);

            return compile_assign(engine, compiler, self);
        }

        let lhs = self.lhs().compile(engine, compiler)?;
        let rhs = self.rhs().compile(engine, compiler)?;

        match self.op() {
            ast::BinOp::Add => compiler.add(self.span(), &lhs, &rhs, &output),
            ast::BinOp::Sub => compiler.sub(self.span(), &lhs, &rhs, &output),
            ast::BinOp::Mul => compiler.mul(self.span(), &lhs, &rhs, &output),
            ast::BinOp::Div => compiler.div(self.span(), &lhs, &rhs, &output),
            ast::BinOp::Eq => compiler.eq(self.span(), &lhs, &rhs, &output),
            ast::BinOp::Neq => compiler.neq(self.span(), &lhs, &rhs, &output),
            ast::BinOp::Lt => compiler.lt(self.span(), &lhs, &rhs, &output),
            ast::BinOp::Leq => compiler.leq(self.span(), &lhs, &rhs, &output),
            ast::BinOp::Gt => compiler.gt(self.span(), &lhs, &rhs, &output),
            ast::BinOp::Geq => compiler.geq(self.span(), &lhs, &rhs, &output),
            ast::BinOp::In => compiler.in_(self.span(), &lhs, &rhs, &output),
            ast::BinOp::NotIn => compiler.not_in(self.span(), &lhs, &rhs, &output),
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

        let output = compiler.register();
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
    // First we run the lhs.
    let lhs = binary.lhs().compile(engine, compiler)?;

    // Then we create a marker for the end of the operation.
    let marker = compiler.marker();

    // Then we conditionally jump to the end.
    match binary.op() {
        ast::BinOp::Or => compiler.jump_if(binary.span(), &lhs, marker),
        ast::BinOp::And => compiler.jump_if_not(binary.span(), &lhs, marker),
        _ => unreachable!(),
    }

    // Then we run the rhs.
    let rhs = binary.rhs().compile(engine, compiler)?;

    // Add the marker.
    compiler.mark(binary.span(), marker);

    // Then, based on the result of the lhs, we either select the rhs to the output or the lhs.
    match binary.op() {
        ast::BinOp::Or => {
            compiler.select(binary.span(), &lhs, Readable::bool(true), &rhs, &output)
        }
        ast::BinOp::And => {
            compiler.select(binary.span(), &lhs, &rhs, Readable::bool(false), &output)
        }
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
        ast::BinOp::Assign => compiler.assign(binary.span(), &rhs, access),
        ast::BinOp::AddAssign => compiler.add_assign(binary.span(), &rhs, access),
        ast::BinOp::SubAssign => compiler.sub_assign(binary.span(), &rhs, access),
        ast::BinOp::MulAssign => compiler.mul_assign(binary.span(), &rhs, access),
        ast::BinOp::DivAssign => compiler.div_assign(binary.span(), &rhs, access),
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
            ast::UnOp::Pos => compiler.pos(self.span(), &value, &output),
            ast::UnOp::Neg => compiler.neg(self.span(), &value, &output),
            ast::UnOp::Not => compiler.not(self.span(), &value, &output),
        }

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let output = compiler.register();
        self.compile_into(engine, compiler, Some(output.clone().into()))?;
        Ok(output.into())
    }
}
