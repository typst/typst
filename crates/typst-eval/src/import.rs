use comemo::TrackedMut;
use ecow::{eco_format, eco_vec, EcoString};
use typst_library::diag::{
    bail, error, warning, At, FileError, SourceResult, Trace, Tracepoint,
};
use typst_library::engine::Engine;
use typst_library::foundations::{Content, Module, Value};
use typst_library::World;
use typst_syntax::ast::{self, AstNode, BareImportError};
use typst_syntax::package::{PackageManifest, PackageSpec};
use typst_syntax::{FileId, Span, VirtualPath};

use crate::{eval, Eval, Vm};

impl Eval for ast::ModuleImport<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let source_expr = self.source();
        let source_span = source_expr.span();

        let mut source = source_expr.eval(vm)?;
        let mut is_str = false;

        match &source {
            Value::Func(func) => {
                if func.scope().is_none() {
                    bail!(source_span, "cannot import from user-defined functions");
                }
            }
            Value::Type(_) => {}
            Value::Module(_) => {}
            Value::Str(path) => {
                source = Value::Module(import(&mut vm.engine, path, source_span)?);
                is_str = true;
            }
            v => {
                bail!(
                    source_span,
                    "expected path, module, function, or type, found {}",
                    v.ty()
                )
            }
        }

        // Source itself is imported if there is no import list or a rename.
        let bare_name = self.bare_name();
        let new_name = self.new_name();
        if let Some(new_name) = new_name {
            if let Ok(source_name) = &bare_name {
                if source_name == new_name.as_str() {
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
        match self.imports() {
            None => {
                if new_name.is_none() {
                    match self.bare_name() {
                        Ok(name)
                            if !is_str || matches!(source_expr, ast::Expr::Str(_)) =>
                        {
                            if matches!(source_expr, ast::Expr::Ident(_)) {
                                // Warn on `import x` where `x` is not a string.
                                vm.engine.sink.warn(warning!(
                                    source_expr.span(),
                                    "this import has no effect",
                                ));
                            }
                            vm.scopes.top.define_spanned(name, source, source_span);
                        }
                        Ok(_) | Err(BareImportError::Dynamic) => bail!(
                            source_span, "dynamic import requires an explicit name";
                            hint: "you can name the import with `as`"
                        ),
                        Err(BareImportError::PathInvalid) => bail!(
                            source_span, "module name would not be a valid identifier";
                            hint: "you can rename the import with `as`",
                        ),
                        // Bad package spec would have failed the import already.
                        Err(BareImportError::PackageInvalid) => unreachable!(),
                    }
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
        let module = match source {
            Value::Str(path) => import(&mut vm.engine, &path, span)?,
            Value::Module(module) => module,
            v => bail!(span, "expected path or module, found {}", v.ty()),
        };
        Ok(module.content())
    }
}

/// Process an import of a package or file relative to the current location.
pub fn import(engine: &mut Engine, from: &str, span: Span) -> SourceResult<Module> {
    if from.starts_with('@') {
        let spec = from.parse::<PackageSpec>().at(span)?;
        import_package(engine, spec, span)
    } else {
        let id = span.resolve_path(from).at(span)?;
        import_file(engine, id, span)
    }
}

/// Import a file from a path. The path is resolved relative to the given
/// `span`.
fn import_file(engine: &mut Engine, id: FileId, span: Span) -> SourceResult<Module> {
    // Load the source file.
    let source = engine.world.source(id).at(span)?;

    // Prevent cyclic importing.
    if engine.route.contains(source.id()) {
        bail!(span, "cyclic import");
    }

    // Evaluate the file.
    let point = || Tracepoint::Import;
    eval(
        engine.routines,
        engine.world,
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        &source,
    )
    .trace(engine.world, point, span)
}

/// Import an external package.
fn import_package(
    engine: &mut Engine,
    spec: PackageSpec,
    span: Span,
) -> SourceResult<Module> {
    let (name, id) = resolve_package(engine, spec, span)?;
    import_file(engine, id, span).map(|module| module.with_name(name))
}

/// Resolve the name and entrypoint of a package.
fn resolve_package(
    engine: &mut Engine,
    spec: PackageSpec,
    span: Span,
) -> SourceResult<(EcoString, FileId)> {
    // Evaluate the manifest.
    let manifest_id = FileId::new(Some(spec.clone()), VirtualPath::new("typst.toml"));
    let bytes = engine.world.file(manifest_id).at(span)?;
    let string = bytes.as_str().map_err(FileError::from).at(span)?;
    let manifest: PackageManifest = toml::from_str(string)
        .map_err(|err| eco_format!("package manifest is malformed ({})", err.message()))
        .at(span)?;
    manifest.validate(&spec).at(span)?;

    // Evaluate the entry point.
    Ok((manifest.package.name, manifest_id.join(&manifest.package.entrypoint)))
}
