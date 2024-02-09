use std::sync::Arc;

use comemo::Prehashed;
use typst_syntax::ast::{self, AstNode};
use typst_syntax::Span;

use crate::engine::Engine;
use crate::vm::Closure;
use crate::{diag::SourceResult, util::PicoStr};

use super::{
    AccessPattern, Compile, CompileTopLevel, CompiledCode, CompiledParam, Compiler,
    PatternCompile, PatternItem, PatternKind, ReadableGuard, WritableGuard,
};

/// A closure that has been compiled but is not yet instantiated.
#[derive(Clone, Hash, PartialEq)]
pub enum CompiledClosure {
    /// A closure that has been compiled but is not yet instantiated.
    Closure(Arc<Prehashed<CompiledCode>>),
    /// A closure that has been instantiated statically.
    ///
    /// This is used for closures that do not capture any variables.
    /// The closure is already compiled and can be used directly.
    Instanciated(Closure),
}

impl CompiledClosure {
    pub fn new(resource: CompiledCode, compiler: &Compiler) -> Self {
        // Check whether we have any defaults that are resolved at runtime.
        let has_defaults = resource
            .params
            .iter()
            .flat_map(|param| param.iter())
            .filter_map(|param| param.default())
            .any(|default| default.is_reg());

        // Check if we have any captures.
        let has_captures = !resource.captures.as_ref().map_or(false, |c| c.is_empty());

        if has_defaults || has_captures {
            Self::Closure(Arc::new(Prehashed::new(resource)))
        } else {
            let scope = compiler.scope.borrow();
            Self::Instanciated(Closure::no_instance(
                resource,
                &compiler.common.constants,
                &scope.global,
                &compiler.common.strings,
            ))
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Self::Closure(resource) => resource.span,
            Self::Instanciated(closure) => closure.inner.compiled.span,
        }
    }
}

impl Compile for ast::Closure<'_> {
    type Output = WritableGuard;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        // Evaluate default values of named parameters.
        let mut defaults = Vec::new();
        for param in self.params().children() {
            if let ast::Param::Named(named) = param {
                let reg = named.expr().compile(engine, compiler)?;
                defaults.push(reg);
            }
        }

        let name = compiler.name.clone();
        let mut closure_compiler = Compiler::function(
            compiler,
            name.clone().unwrap_or_else(|| pico!("anonymous")),
        );

        // Create the local such that the closure can use itself.
        let closure_local = if let Some(name) = name {
            Some(closure_compiler.declare(self.span(), name))
        } else {
            None
        };

        // Build the parameter list of the closure.
        let mut params = Vec::with_capacity(self.params().children().count());
        let mut defaults_iter = defaults.iter();
        for param in self.params().children() {
            match param {
                ast::Param::Pos(pat) => {
                    // Compile the pattern.
                    let pattern = pat.compile(engine, &mut closure_compiler, true)?;

                    // We destructure the initializer using the pattern.
                    // Simple patterns can be directly stored.
                    if let PatternKind::Single(PatternItem::Simple(
                        _,
                        AccessPattern::Writable(WritableGuard::Register(reg)),
                        name,
                    )) = &pattern.kind
                    {
                        params.push(CompiledParam::Pos(
                            reg.as_register(),
                            PicoStr::new(name.as_str()),
                        ));
                    } else {
                        // Create a register for the pattern.
                        let reg = closure_compiler.register();
                        let pattern_id =
                            closure_compiler.pattern(pattern.as_vm_pattern());
                        params.push(CompiledParam::Pos(
                            reg.as_register(),
                            "anonymous".into(),
                        ));
                        closure_compiler.destructure(
                            pat.span(),
                            reg.as_readable(),
                            pattern_id,
                        );
                    }
                }
                ast::Param::Named(named) => {
                    // Create the local variable.
                    let name = named.name().get();
                    let target = closure_compiler.declare(named.name().span(), name);

                    // Add the parameter to the list.
                    params.push(CompiledParam::Named {
                        span: named.span(),
                        target: target.as_register(),
                        name: PicoStr::new(name.as_str()),
                        default: defaults_iter.next().map(|r| r.as_readable()),
                    });
                }
                ast::Param::Sink(sink) => {
                    let Some(name) = sink.name() else {
                        // Add the parameter to the list.
                        params.push(CompiledParam::Sink(sink.span(), None, pico!("..")));
                        continue;
                    };

                    // Create the local variable.
                    let target = closure_compiler.declare(name.span(), name.get());

                    params.push(CompiledParam::Sink(
                        sink.span(),
                        Some(target.as_register()),
                        pico!(".."),
                    ));
                }
            }
        }

        // Compile the body of the closure.
        match self.body() {
            ast::Expr::Code(code) => {
                code.body().compile_top_level(engine, &mut closure_compiler)?;
            }
            ast::Expr::Content(content) => {
                content.body().compile_top_level(engine, &mut closure_compiler)?;
            }
            other => other.compile_into(
                engine,
                &mut closure_compiler,
                WritableGuard::Joined,
            )?,
        }

        closure_compiler.flow();

        // Collect the compiled closure.
        let closure = closure_compiler.finish_closure(self.span(), params, closure_local);

        // Get the closure ID.
        let compiled = CompiledClosure::new(closure, &*compiler);
        let closure_id = compiler.closure(compiled);

        // Instantiate the closure.
        compiler.instantiate(self.span(), closure_id, &output);

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Get an output register.
        let reg = compiler.register();

        // Compile into the register.
        self.compile_into(engine, compiler, reg.clone().into())?;

        // Return the register.
        Ok(reg.into())
    }
}
