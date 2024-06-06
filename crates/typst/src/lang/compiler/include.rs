use typst_syntax::ast::{self, AstNode};
use typst_syntax::Span;

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{unknown_variable, Value};
use crate::lang::compiler::CompileAccess;

use super::{import_file, Compile, Compiler, ReadableGuard, WritableGuard};

impl Compile for ast::ModuleInclude<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        self.source().include(compiler, engine, output)
    }
}

trait ModuleInclude {
    fn include(
        &self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()>;
}

impl ModuleInclude for ast::Expr<'_> {
    fn include(
        &self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        match self {
            ast::Expr::Ident(ident) => ident.include(compiler, engine, output),
            ast::Expr::Str(str) => str.include(compiler, engine, output),
            ast::Expr::FieldAccess(field_access) => {
                field_access.include(compiler, engine, output)
            }
            other => {
                // For all other value (even invalid one), we defer to the dynamic implementation
                let path = other.compile_to_readable(compiler, engine)?;

                // We perform a dynamic include
                compiler.include(self.span(), path, output);

                Ok(())
            }
        }
    }
}

impl ModuleInclude for ast::Ident<'_> {
    fn include(
        &self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let Some(readable) = compiler.read(self.span(), self.as_str(), false) else {
            return Err(unknown_variable(self.as_str())).at(self.span());
        };

        match readable {
            ReadableGuard::Register(register) => {
                // If we are a constant alias, we can try and import it.
                if let Some(variable) = compiler.resolve_var(&register) {
                    if variable.constant {
                        let default = compiler.resolve_default(&register);
                        return include_value(
                            compiler,
                            engine,
                            default.unwrap(),
                            output,
                            self.span(),
                        );
                    }
                }
            }
            ReadableGuard::Constant(constant) => {
                // If we are a constant, we can try and import it.
                let value = compiler.get_constant(&constant).unwrap();

                return include_value(
                    compiler,
                    engine,
                    value.clone(),
                    output,
                    self.span(),
                );
            }
            ReadableGuard::String(string) => {
                // If we are a string, we can try and import it.
                let path = compiler.get_string(&string).unwrap();
                let Value::Str(path) = path.clone() else {
                    bail!(
                        self.span(),
                        "expected string, found {}",
                        path.ty().short_name()
                    );
                };

                return include_path(
                    compiler,
                    engine,
                    path.as_str(),
                    output,
                    self.span(),
                );
            }
            ReadableGuard::Global(value) => {
                // If we are a global, we can try and import it.
                let Some(lib) =
                    compiler.library().global.field_by_index(value.as_raw() as usize)
                else {
                    bail!(
                        self.span(),
                        "invalid global value {}, this is a compiler bug",
                        value.as_raw()
                    );
                };

                return include_value(compiler, engine, lib.clone(), output, self.span());
            }
            ReadableGuard::GlobalModule => {
                let value = &compiler.library().std;

                return include_value(
                    compiler,
                    engine,
                    value.clone(),
                    output,
                    self.span(),
                );
            }
            ReadableGuard::Captured(_) => {}
            ReadableGuard::Math(_) => {
                bail!(self.span(), "expected a path or a module, found an equation")
            }
            ReadableGuard::Bool(_) => {
                bail!(self.span(), "expected a path or a module, found a boolean")
            }
            ReadableGuard::Label(_) => {
                bail!(self.span(), "expected a path or a module, found a label")
            }
            ReadableGuard::None => {
                bail!(self.span(), "expected a path or a module, found none")
            }
            ReadableGuard::Auto => {
                bail!(self.span(), "expected a path or a module, found auto")
            }
        }

        // Otherwise, we include the value.
        let path = self.compile_to_readable(compiler, engine)?;
        compiler.include(self.span(), path, output);

        Ok(())
    }
}

impl ModuleInclude for ast::Str<'_> {
    fn include(
        &self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let path = self.get();

        include_path(compiler, engine, path.as_str(), output, self.span())
    }
}

impl ModuleInclude for ast::FieldAccess<'_> {
    fn include(
        &self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let access = self.access(compiler, engine, false)?;

        // If we can resolve it as a constant, we can try and import it.
        let Some(resolved) = access.resolve(compiler)? else {
            // Otherwise we default to the dynamic case.
            let access_id = compiler.access(access);
            compiler.access_isr(self.span(), access_id, output.clone());

            // We include the value.
            compiler.include(self.span(), output.clone(), output);

            return Ok(());
        };

        include_value(compiler, engine, resolved, output, self.span())
    }
}

fn include_value(
    compiler: &mut Compiler,
    engine: &mut Engine,
    value: Value,
    target: WritableGuard,
    span: Span,
) -> SourceResult<()> {
    match value {
        Value::Module(module) => {
            // Get the content of the module.
            let content = module.content();

            // Store it as a constant.
            let constant = compiler.const_(content);

            // Copy the constant to the target.
            compiler.copy(span, constant, target);

            Ok(())
        }
        Value::Str(path) => include_path(compiler, engine, path.as_str(), target, span),
        value => {
            bail!(span, "expected a path or a module, found {}", value.ty());
        }
    }
}

fn include_path(
    compiler: &mut Compiler,
    engine: &mut Engine,
    path: &str,
    target: WritableGuard,
    span: Span,
) -> SourceResult<()> {
    let module = import_file(engine, path, span)?;

    // Get the content of the module.
    let content = module.content();

    // Store it as a constant.
    let constant = compiler.const_(content);

    // Copy the constant to the target.
    compiler.copy(span, constant, target);

    Ok(())
}
