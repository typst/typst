use typst_syntax::ast::{self, AstNode};

use crate::engine::Engine;
use crate::foundations::is_mutating_method;
use crate::{diag::SourceResult, lang::operands::Readable};

use super::{Access, Compile, CompileAccess, Compiler, WritableGuard};

impl Compile for ast::FuncCall<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let callee = self.callee();
        let in_math = in_math(callee);
        let args = self.args();
        let trailing_comma = args.trailing_comma();

        let args = args.compile_to_readable(compiler, engine)?;

        // Try to compile an associated function.
        let callee = if let ast::Expr::FieldAccess(access) = callee {
            let field = access.field();

            // If this is a mutating method, we need to access the target instead
            // of the usual copy.
            if is_mutating_method(&field) {
                access.access(compiler, engine, true)?
            } else {
                let c = self.callee().compile_to_readable(compiler, engine)?;
                Access::Readable(c)
            }
        } else {
            let c = self.callee().compile_to_readable(compiler, engine)?;
            Access::Readable(c)
        };

        let closure = compiler.access(callee);
        compiler.call(self.span(), closure, args, in_math, trailing_comma, output);

        Ok(())
    }
}

impl Compile for ast::Args<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let mut args = self.items();
        let Some(first) = args.next() else {
            compiler.copy(self.span(), Readable::none(), output);

            return Ok(());
        };

        // Allocate the arguments
        let capacity = self.items().count();
        compiler.args(self.span(), capacity as u32, output.clone());

        // Compile the first argument
        first.compile(compiler, engine, output.clone())?;

        // Compile the rest of the arguments
        for arg in args {
            arg.compile(compiler, engine, output.clone())?;
        }

        Ok(())
    }
}

impl Compile for ast::Arg<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        match self {
            ast::Arg::Pos(pos) => {
                let guard = pos.compile_to_readable(compiler, engine)?;
                let span_id = compiler.span(pos.span());
                compiler.push_arg(self.span(), guard, span_id, output);
            }
            ast::Arg::Named(named) => {
                let name = compiler.string(named.name().as_str());
                let value = named.expr().compile_to_readable(compiler, engine)?;
                let span_id = compiler.span(named.expr().span());
                compiler.insert_arg(self.span(), name, value, span_id, output);
            }
            ast::Arg::Spread(spread) => {
                let guard = spread.expr().compile_to_readable(compiler, engine)?;
                let span_id = compiler.span(spread.span());
                compiler.spread_arg(self.span(), guard, span_id, output);
            }
        }

        Ok(())
    }

    fn compile_to_readable(
        &self,
        _: &mut Compiler<'_>,
        _: &mut Engine,
    ) -> SourceResult<super::ReadableGuard> {
        unreachable!("`Arg` should be compiled through `Compile::compile")
    }
}

/// Checks whether the given expression is in a math context.
fn in_math(expr: ast::Expr) -> bool {
    match expr {
        ast::Expr::MathIdent(_) => true,
        ast::Expr::FieldAccess(access) => in_math(access.target()),
        _ => false,
    }
}
