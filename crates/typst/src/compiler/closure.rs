use ecow::{EcoString, EcoVec};
use typst_syntax::ast::{self, AstNode};

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::vm::{CompiledParam, OptionalWritable};

use super::{
    AccessPattern, Compile, Compiler, PatternCompile, PatternItem, PatternKind,
    ReadableGuard, WritableGuard,
};

impl Compile for ast::Closure<'_> {
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
            name.clone().unwrap_or_else(|| EcoString::inline("anonymous")),
        );

        // Create the local such that the closure can use itself.
        let closure_local = if let Some(name) = name.clone() {
            Some(closure_compiler.declare(self.span(), name))
        } else {
            None
        };

        // Build the parameter list of the closure.
        let mut params = EcoVec::new();
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
                        AccessPattern::Writable(reg),
                        name,
                    )) = &pattern.kind
                    {
                        params.push(CompiledParam::Pos(reg.into(), name.clone()));
                    } else {
                        // Create a register for the pattern.
                        let reg = closure_compiler.register();
                        let pattern_id =
                            closure_compiler.pattern(pattern.as_vm_pattern());
                        params.push(CompiledParam::Pos(
                            reg.as_writeable(),
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
                    let target =
                        closure_compiler.declare(named.name().span(), name.clone());

                    // Add the parameter to the list.
                    params.push(CompiledParam::Named {
                        span: named.span(),
                        target: target.as_writeable(),
                        name: name.clone(),
                        default: defaults_iter.next().map(|r| r.as_readable()),
                    });
                }
                ast::Param::Sink(sink) => {
                    let Some(name) = sink.name() else {
                        // Add the parameter to the list.
                        params.push(CompiledParam::Sink(
                            sink.span(),
                            OptionalWritable::none(),
                            EcoString::new(),
                        ));
                        continue;
                    };

                    // Create the local variable.
                    let target =
                        closure_compiler.declare(name.span(), name.get().clone());

                    params.push(CompiledParam::Sink(
                        sink.span(),
                        OptionalWritable::some(target.as_writeable()),
                        EcoString::new(),
                    ));
                }
            }
        }

        // Compile the body of the closure.
        self.body().compile_into(
            engine,
            &mut closure_compiler,
            Some(WritableGuard::Joined),
        )?;
        closure_compiler.flow();

        // Collect the compiled closure.
        let closure = closure_compiler.into_compiled_closure(
            self.span(),
            params,
            closure_local.map(WritableGuard::Register),
        );

        // Get the closure ID.
        let closure_id = compiler.closure(closure);

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
        self.compile_into(engine, compiler, Some(reg.clone().into()))?;

        // Return the register.
        Ok(reg.into())
    }
}
