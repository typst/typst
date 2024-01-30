use typst_syntax::ast::{self, AstNode};

use crate::compiler::Access;
use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::is_mutating_method;
use crate::vm::Readable;

use super::{AccessPattern, Compile, Compiler, Opcode, ReadableGuard, WritableGuard};

impl Compile for ast::FuncCall<'_> {
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

        let callee = self.callee();
        let in_math = in_math(callee);
        let args = self.args();
        let trailing_comma = args.trailing_comma();

        let args = args.compile(engine, compiler)?;

        // Try to compile an associated function.
        let callee = if let ast::Expr::FieldAccess(access) = callee {
            let field = access.field();

            // If this is a mutating method, we need to access the target instead
            // of the usual copy.
            if is_mutating_method(&field) {
                access.access(engine, compiler, true)?
            } else {
                let c = self.callee().compile(engine, compiler)?;
                AccessPattern::Readable(c)
            }
        } else {
            let c = self.callee().compile(engine, compiler)?;
            AccessPattern::Readable(c)
        };

        let closure = compiler.access(callee.as_vm_access());
        compiler.isr(Opcode::call(
            self.span(),
            closure,
            &args,
            if in_math { 0b01 } else { 0b00 } | if trailing_comma { 0b10 } else { 0b00 },
            &output,
        ));

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

impl Compile for ast::Args<'_> {
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

        let capacity = self.items().count();
        let mut args = self.items();
        let Some(first) = args.next() else {
            compiler.isr(Opcode::copy(self.span(), Readable::none(), &output));

            return Ok(());
        };

        // Allocate the arguments
        compiler.isr(Opcode::args(self.span(), capacity as u32, &output));

        // Compile the first argument
        first.compile_into(engine, compiler, output.clone())?;

        // Compile the rest of the arguments
        for arg in args {
            arg.compile_into(engine, compiler, output.clone())?;
        }

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

impl Compile for ast::Arg<'_> {
    type Output = WritableGuard;
    type IntoOutput = ();

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        match self {
            ast::Arg::Pos(pos) => {
                let pos = pos.compile(engine, compiler)?;
                compiler.isr(Opcode::push_arg(self.span(), &pos, &output));
            }
            ast::Arg::Named(named) => {
                let name = named.name().get().clone();
                let name_id = compiler.string(name.clone());
                let value = named.expr().compile(engine, compiler)?;
                compiler.isr(Opcode::insert_arg(self.span(), name_id, &value, &output));
            }
            ast::Arg::Spread(spread) => {
                let spread = spread.compile(engine, compiler)?;
                compiler.isr(Opcode::spread(self.span(), &spread, &output));
            }
        }

        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        _: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        bail!(self.span(), "cannot compile individual arguments")
    }
}

fn in_math(expr: ast::Expr) -> bool {
    match expr {
        ast::Expr::MathIdent(_) => true,
        ast::Expr::FieldAccess(access) => in_math(access.target()),
        _ => false,
    }
}
