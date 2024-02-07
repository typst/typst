use typst_syntax::ast::{self, AstNode};

use crate::engine::Engine;
use crate::{diag::SourceResult, vm::Pointer};

use super::{Compile, ReadableGuard};

fn compile_set(
    set: &ast::SetRule<'_>,
    engine: &mut Engine,
    compiler: &mut super::Compiler,
) -> SourceResult<(ReadableGuard, ReadableGuard, Option<Pointer>)> {
    if let Some(expr) = set.condition() {
        // Compile the condition.
        let condition = expr.compile(engine, compiler)?;

        // Create the jump marker.
        let else_ = compiler.marker();

        // Create the jump.
        compiler.jump_if_not(expr.span(), condition.as_readable(), else_);

        // Compile the set.
        let target = set.target().compile(engine, compiler)?;
        let args = set.args().compile(engine, compiler)?;

        Ok((target, args, Some(else_)))
    } else {
        let target = set.target().compile(engine, compiler)?;
        let args = set.args().compile(engine, compiler)?;

        Ok((target, args, None))
    }
}

impl Compile for ast::SetRule<'_> {
    type Output = ();
    type IntoOutput = ();

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
        _: Self::Output,
    ) -> SourceResult<()> {
        let (target, args, else_) = compile_set(self, engine, compiler)?;
        compiler.set(self.span(), target.as_readable(), args.as_readable());
        if let Some(else_) = else_ {
            compiler.mark(self.span(), else_);
        }

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Compile into the register.
        self.compile_into(engine, compiler, ())
    }
}

impl Compile for ast::ShowRule<'_> {
    type Output = ();
    type IntoOutput = ();

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
        _: Self::Output,
    ) -> SourceResult<()> {
        let selector =
            self.selector().map(|sel| sel.compile(engine, compiler)).transpose()?;

        match self.transform() {
            ast::Expr::Set(set) => {
                let (target, args, else_) = compile_set(&set, engine, compiler)?;
                compiler.show_set(
                    self.span(),
                    selector.map(|s| s.as_readable()),
                    target.as_readable(),
                    args.as_readable(),
                );

                if let Some(else_) = else_ {
                    compiler.mark(self.span(), else_);
                }
            }
            other => {
                let transform = other.compile(engine, compiler)?;
                compiler.show(self.span(), selector.map(|s| s.as_readable()), &transform);
            }
        }

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        self.compile_into(engine, compiler, ())
    }
}
