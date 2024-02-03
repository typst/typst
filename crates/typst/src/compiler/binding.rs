use typst_syntax::ast::{self, AstNode};

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::vm::Readable;

use super::{
    AccessPattern, Compile, Compiler, PatternCompile, PatternItem, PatternKind,
    ReadableGuard, WritableGuard,
};

impl Compile for ast::LetBinding<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ();

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        if let Some(output) = output {
            compiler.copy(self.span(), Readable::none(), &output);
        }

        self.compile(engine, compiler)?;
        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        match self.kind() {
            ast::LetBindingKind::Normal(pattern) => {
                compile_normal(engine, compiler, self, &pattern)
            }
            ast::LetBindingKind::Closure(closure) => {
                compile_closure(engine, compiler, self, &closure)
            }
        }
    }
}

fn compile_normal(
    engine: &mut Engine,
    compiler: &mut Compiler,
    binding: &ast::LetBinding<'_>,
    pattern: &ast::Pattern<'_>,
) -> SourceResult<()> {
    // Simple patterns can be directly stored.
    if let ast::Pattern::Normal(ast::Expr::Ident(ident)) = pattern {
        let guard = compiler.register();
        if let Some(init) = binding.init() {
            init.compile_into(
                engine,
                compiler,
                Some(WritableGuard::from(guard.clone())),
            )?;
        }
        compiler
            .declare_into(ident.span(), ident.get().clone(), guard);
    } else {
        // We destructure the initializer using the pattern.
        let value = if let Some(init) = binding.init() {
            init.compile(engine, compiler)?
        } else {
            ReadableGuard::None
        };

        // We compile the pattern.
        let pattern = pattern.compile(engine, compiler, true)?;
        let pattern_id = compiler.pattern(pattern.as_vm_pattern());

        // We destructure the initializer using the pattern.
        compiler.flow();
        compiler.destructure(binding.span(), &value, pattern_id);
    }

    Ok(())
}

fn compile_closure(
    engine: &mut Engine,
    compiler: &mut Compiler,
    binding: &ast::LetBinding<'_>,
    closure_name: &ast::Ident<'_>,
) -> SourceResult<()> {
    // We create the local.
    let local = compiler
        .declare(closure_name.span(), closure_name.get().clone());

    let Some(init) = binding.init() else {
        bail!(binding.span(), "closure declaration requires an initializer");
    };

    // We swap the names
    let mut name = Some(closure_name.get().clone());
    std::mem::swap(&mut name, &mut compiler.name);

    // We compile the initializer.
    init.compile_into(engine, compiler, Some(local.into()))?;

    // We swap the names back.
    std::mem::swap(&mut name, &mut compiler.name);

    Ok(())
}

impl Compile for ast::DestructAssignment<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ();

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        self.compile(engine, compiler)?;
        if let Some(output) = output {
            compiler.copy(self.span(), Readable::none(), &output);
        }

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // We compile the pattern and add it to the local scope.
        let pattern = self.pattern().compile(engine, compiler, false)?;

        // We destructure the initializer using the pattern.
        // Simple patterns can be directly stored.
        if let PatternKind::Single(PatternItem::Simple(
            _,
            AccessPattern::Writable(guard),
            _,
        )) = &pattern.kind
        {
            self.value().compile_into(engine, compiler, Some(guard.clone()))?;
        } else {
            let value = self.value().compile(engine, compiler)?;
            let pattern_id = compiler.pattern(pattern.as_vm_pattern());

            compiler.destructure(self.span(), &value, pattern_id);
        }

        Ok(())
    }
}
