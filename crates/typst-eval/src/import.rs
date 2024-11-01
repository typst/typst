use comemo::TrackedMut;
use ecow::{eco_format, eco_vec, EcoString};
use typst_library::diag::{
    bail, error, warning, At, FileError, SourceResult, Trace, Tracepoint,
};
use typst_library::foundations::{Content, Module, Value};
use typst_library::World;
use typst_syntax::ast::{self, AstNode};
use typst_syntax::package::{PackageManifest, PackageSpec};
use typst_syntax::{FileId, Span, VirtualPath};

use crate::{eval, Eval, Vm};

impl Eval for ast::ModuleImport<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let source = self.source();
        let source_span = source.span();
        let mut source = source.eval(vm)?;
        let new_name = self.new_name();
        let imports = self.imports();

        match &source {
            Value::Func(func) => {
                if func.scope().is_none() {
                    bail!(source_span, "cannot import from user-defined functions");
                }
            }
            Value::Type(_) => {}
            other => {
                source = Value::Module(import(vm, other.clone(), source_span, true)?);
            }
        }

        if let Some(new_name) = new_name {
            if let ast::Expr::Ident(ident) = self.source() {
                if ident.as_str() == new_name.as_str() {
                    // Warn on `import x as x`
                    vm.engine.sink.warn(warning!(
                        new_name.span(),
                        "unnecessary import rename to same name",
                    ));
                }
            }

            // Define renamed module on the scope.
            vm.scopes.top.define_ident(new_name, source.clone());
        }

        let scope = source.scope().unwrap();
        match imports {
            None => {
                // Only import here if there is no rename.
                if new_name.is_none() {
                    let name: EcoString = source.name().unwrap().into();
                    vm.scopes.top.define(name, source);
                }
            }
            Some(ast::Imports::Wildcard) => {
                for (var, value, span) in scope.iter() {
                    vm.scopes.top.define_spanned(var.clone(), value.clone(), span);
                }
            }
            Some(ast::Imports::Items(items)) => {
                let mut errors = eco_vec![];
                for item in items.iter() {
                    let mut path = item.path().iter().peekable();
                    let mut scope = scope;

                    while let Some(component) = &path.next() {
                        let Some(value) = scope.get(component) else {
                            errors.push(error!(component.span(), "unresolved import"));
                            break;
                        };

                        if path.peek().is_some() {
                            // Nested import, as this is not the last component.
                            // This must be a submodule.
                            let Some(submodule) = value.scope() else {
                                let error = if matches!(value, Value::Func(function) if function.scope().is_none())
                                {
                                    error!(
                                        component.span(),
                                        "cannot import from user-defined functions"
                                    )
                                } else if !matches!(
                                    value,
                                    Value::Func(_) | Value::Module(_) | Value::Type(_)
                                ) {
                                    error!(
                                        component.span(),
                                        "expected module, function, or type, found {}",
                                        value.ty()
                                    )
                                } else {
                                    panic!("unexpected nested import failure")
                                };
                                errors.push(error);
                                break;
                            };

                            // Walk into the submodule.
                            scope = submodule;
                        } else {
                            // Now that we have the scope of the innermost submodule
                            // in the import path, we may extract the desired item from
                            // it.

                            // Warn on `import ...: x as x`
                            if let ast::ImportItem::Renamed(renamed_item) = &item {
                                if renamed_item.original_name().as_str()
                                    == renamed_item.new_name().as_str()
                                {
                                    vm.engine.sink.warn(warning!(
                                        renamed_item.new_name().span(),
                                        "unnecessary import rename to same name",
                                    ));
                                }
                            }

                            vm.define(item.bound_name(), value.clone());
                        }
                    }
                }
                if !errors.is_empty() {
                    return Err(errors);
                }
            }
        }

        Ok(Value::None)
    }
}

impl Eval for ast::ModuleInclude<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.source().span();
        let source = self.source().eval(vm)?;
        let module = import(vm, source, span, false)?;
        Ok(module.content())
    }
}

/// Process an import of a module relative to the current location.
pub fn import(
    vm: &mut Vm,
    source: Value,
    span: Span,
    allow_scopes: bool,
) -> SourceResult<Module> {
    let path = match source {
        Value::Str(path) => path,
        Value::Module(module) => return Ok(module),
        v if allow_scopes => {
            bail!(span, "expected path, module, function, or type, found {}", v.ty())
        }
        v => bail!(span, "expected path or module, found {}", v.ty()),
    };

    // Handle package and file imports.
    let path = path.as_str();
    if path.starts_with('@') {
        let spec = path.parse::<PackageSpec>().at(span)?;
        import_package(vm, spec, span)
    } else {
        import_file(vm, path, span)
    }
}

/// Import an external package.
fn import_package(vm: &mut Vm, spec: PackageSpec, span: Span) -> SourceResult<Module> {
    // Evaluate the manifest.
    let world = vm.world();
    let manifest_id = FileId::new(Some(spec.clone()), VirtualPath::new("typst.toml"));
    let bytes = world.file(manifest_id).at(span)?;
    let string = std::str::from_utf8(&bytes).map_err(FileError::from).at(span)?;
    let manifest: PackageManifest = toml::from_str(string)
        .map_err(|err| eco_format!("package manifest is malformed ({})", err.message()))
        .at(span)?;
    manifest.validate(&spec).at(span)?;

    // Evaluate the entry point.
    let entrypoint_id = manifest_id.join(&manifest.package.entrypoint);
    let source = world.source(entrypoint_id).at(span)?;

    // Prevent cyclic importing.
    if vm.engine.route.contains(source.id()) {
        bail!(span, "cyclic import");
    }

    let point = || Tracepoint::Import;
    Ok(eval(
        vm.engine.routines,
        vm.engine.world,
        vm.engine.traced,
        TrackedMut::reborrow_mut(&mut vm.engine.sink),
        vm.engine.route.track(),
        &source,
    )
    .trace(world, point, span)?
    .with_name(manifest.package.name))
}

/// Import a file from a path.
fn import_file(vm: &mut Vm, path: &str, span: Span) -> SourceResult<Module> {
    // Load the source file.
    let world = vm.world();
    let id = span.resolve_path(path).at(span)?;
    let source = world.source(id).at(span)?;

    // Prevent cyclic importing.
    if vm.engine.route.contains(source.id()) {
        bail!(span, "cyclic import");
    }

    // Evaluate the file.
    let point = || Tracepoint::Import;
    eval(
        vm.engine.routines,
        vm.engine.world,
        vm.engine.traced,
        TrackedMut::reborrow_mut(&mut vm.engine.sink),
        vm.engine.route.track(),
        &source,
    )
    .trace(world, point, span)
}
