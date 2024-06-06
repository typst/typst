use typst_syntax::{
    ast::{self, AstNode},
    Span,
};

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::lang::operands::Pointer;

use super::{call::ArgsCompile, Compile, Compiler, ReadableGuard, WritableGuard};

impl Compile for ast::SetRule<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        _: WritableGuard,
    ) -> SourceResult<()> {
        self.compile_to_readable(compiler, engine)?;

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let (target, args, else_) = compile_set(self, compiler, engine)?;
        compiler.set(self.span(), target, args);
        if let Some(else_) = else_ {
            compiler.mark(self.span(), else_);
        }

        Ok(ReadableGuard::None)
    }
}

impl Compile for ast::ShowRule<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        _: WritableGuard,
    ) -> SourceResult<()> {
        self.compile_to_readable(compiler, engine)?;

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let selector = self
            .selector()
            .map(|sel| sel.compile_to_readable(compiler, engine))
            .transpose()?;

        let selector_span =
            compiler.span(self.selector().map_or_else(Span::detached, ast::Expr::span));

        let transform = self.transform();
        match transform {
            ast::Expr::Set(set) => {
                let (target, args, else_) = compile_set(&set, compiler, engine)?;
                compiler.show_set(
                    transform.span(),
                    selector.map(|s| s.into()),
                    target,
                    args,
                    selector_span,
                );

                if let Some(else_) = else_ {
                    compiler.mark(transform.span(), else_);
                }
            }
            other => {
                let other = other.compile_to_readable(compiler, engine)?;
                compiler.show(
                    transform.span(),
                    selector.map(|s| s.into()),
                    other,
                    selector_span,
                );
            }
        }

        Ok(ReadableGuard::None)
    }
}

fn compile_set(
    set: &ast::SetRule<'_>,
    compiler: &mut super::Compiler,
    engine: &mut Engine,
) -> SourceResult<(ReadableGuard, ReadableGuard, Option<Pointer>)> {
    if let Some(expr) = set.condition() {
        // Compile the condition.
        let condition = expr.compile_to_readable(compiler, engine)?;

        // Create the jump marker.
        let else_ = compiler.marker();

        // Create the jump.
        compiler.jump_if_not(expr.span(), condition, else_);

        // Compile the set.
        let target = set.target().compile_to_readable(compiler, engine)?;
        let args = set.args().compile_args(compiler, engine, set.span())?;

        Ok((target, args, Some(else_)))
    } else {
        let target = set.target().compile_to_readable(compiler, engine)?;
        let args = set.args().compile_args(compiler, engine, set.span())?;

        Ok((target, args, None))
    }
}
