use typst_syntax::ast::{self, AstNode};
use typst_utils::PicoStr;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::Value;
use crate::lang::compiled::{CompiledClosure, CompiledParam};
use crate::lang::compiler::{
    Access, CompileTopLevel, PatternCompile, PatternItem, PatternKind,
};

use super::{Compile, Compiler, WritableGuard};

impl Compile for ast::Closure<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        // Evaluate default values of named parameters.
        let mut defaults = Vec::new();
        for param in self.params().children() {
            if let ast::Param::Named(named) = param {
                let reg = named.expr().compile_to_readable(compiler, engine)?;
                defaults.push(reg);
            }
        }

        // Create a new compiler for the closure.
        let name = compiler.name.as_ref();
        let mut closure_compiler = Compiler::new_closure(compiler, name.cloned());

        // Create the local such that the closure can use itself.
        let closure_local =
            name.map(|name| closure_compiler.declare(self.span(), name.as_str()));

        // Build the parameter list of the closure.
        let mut params = Vec::with_capacity(self.params().children().count());
        let mut defaults_iter = defaults.iter();
        for param in self.params().children() {
            match param {
                ast::Param::Pos(pat) => {
                    // Compile the pattern.
                    let pattern =
                        pat.compile_pattern(&mut closure_compiler, engine, true)?;

                    if let PatternKind::Single(PatternItem::Simple(_, access, name)) =
                        &pattern.kind
                    {
                        let Some(Access::Register(reg)) =
                            closure_compiler.get_access(access)
                        else {
                            bail!(
                                pat.span(),
                                "expected a writable location for param";
                                hint: "this is a compiler bug"
                            );
                        };

                        let Some(Value::Str(name)) = closure_compiler.get_string(name)
                        else {
                            bail!(
                                pat.span(),
                                "expected a string for parameter name";
                                hint: "this is a compiler bug"
                            );
                        };

                        params.push(CompiledParam::Pos(
                            reg.clone().into(),
                            PicoStr::from(name.as_str()),
                        ));

                        continue;
                    }

                    let reg = closure_compiler.allocate();
                    let pattern_id = closure_compiler.pattern(pattern);

                    params.push(CompiledParam::Pos(
                        reg.clone().into(),
                        PicoStr::from("pattern parameter"),
                    ));

                    closure_compiler.destructure(pat.span(), reg, pattern_id);
                }
                ast::Param::Named(named) => {
                    // Create the local variable.
                    let name = named.name().get().as_str();
                    let target = closure_compiler.declare(named.name().span(), name);

                    // Add the parameter to the list.
                    params.push(CompiledParam::Named {
                        span: named.span(),
                        target: target.into(),
                        name: PicoStr::new(name),
                        default: defaults_iter.next().map(|r| r.clone().into()),
                    });
                }
                ast::Param::Spread(spread) => {
                    let Some(name) = spread.sink_ident() else {
                        // Add the parameter to the list.
                        params.push(CompiledParam::Sink(
                            spread.span(),
                            None,
                            PicoStr::from(".."),
                        ));
                        continue;
                    };

                    // Create the local variable.
                    let target = closure_compiler.declare(name.span(), name.as_str());

                    params.push(CompiledParam::Sink(
                        spread.span(),
                        Some(target.as_register()),
                        PicoStr::from(".."),
                    ));
                }
            }
        }

        // Compile the body of the closure.
        match self.body() {
            ast::Expr::Code(code) => {
                code.body().compile_top_level(&mut closure_compiler, engine)?;
            }
            ast::Expr::Content(content) => {
                content.body().compile_top_level(&mut closure_compiler, engine)?;
            }
            other => {
                other.compile(&mut closure_compiler, engine, WritableGuard::Joined)?
            }
        }

        // Ensure that a flow event is present.
        closure_compiler.flow();

        // Collect the compiled closure.
        let closure = closure_compiler.finish_closure(
            self.params().span(),
            params,
            closure_local,
        )?;

        // Get the closure ID.
        let compiled = CompiledClosure::new(closure, &*compiler);
        let closure_id = compiler.closure(compiled);

        // Instantiate the closure.
        compiler.instantiate(self.span(), closure_id, output);

        Ok(())
    }
}
