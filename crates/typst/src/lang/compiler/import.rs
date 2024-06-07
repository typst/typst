use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use comemo::TrackedMut;
use ecow::{eco_format, EcoString};
use indexmap::IndexMap;
use typst_syntax::ast::{self, AstNode};
use typst_syntax::package::{PackageManifest, PackageSpec};
use typst_syntax::{FileId, Span, VirtualPath};
use typst_utils::PicoStr;

use crate::diag::{bail, error, warning, At, FileError, SourceResult, Trace, Tracepoint};
use crate::engine::Engine;
use crate::foundations::{unknown_variable, Module, Value};
use crate::lang::compiler::CompileAccess;
use crate::lang::eval;
use crate::World;

use super::{Compile, Compiler, ReadableGuard, RegisterGuard, WritableGuard};

pub enum ImportedModule {
    Dynamic(DynamicModule),
    Static(Module),
}

#[derive(Clone)]
pub struct DynamicModule {
    pub path: ReadableGuard,
    pub imports: IndexMap<EcoString, DynamicImport>,
    pub glob: Option<RegisterGuard>,
}

impl Hash for DynamicModule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        self.glob.hash(state);

        state.write_usize(self.imports.len());
        for import in self.imports.values() {
            import.hash(state);
        }
    }
}

impl DynamicModule {
    pub fn new(path: impl Into<ReadableGuard>) -> Self {
        Self {
            path: path.into(),
            imports: IndexMap::new(),
            glob: None,
        }
    }
}

#[derive(Debug, Clone, Hash)]
pub struct DynamicImport {
    pub span: Span,
    pub name: PicoStr,
    pub location: RegisterGuard,
}

impl Compile for ast::ModuleImport<'_> {
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
        // Load the actual module.
        let mut module = self.source().load(compiler, engine)?;

        // Handle imports.
        if let Some(imports) = self.imports() {
            module.import(compiler, engine, self.span(), &imports)?;
        } else if self.new_name().is_none() {
            let ImportedModule::Static(module) = &module else {
                bail!(
                    self.span(),
                    "cannot import all items from a dynamic module";
                    hint: "use `import \"...\" as x` to give a name to the module"
                )
            };

            compiler.declare_default(
                self.span(),
                module.name().as_str(),
                Value::Module(module.clone()),
            );
        }

        // Handle renaming.
        if let Some(new_name) = self.new_name() {
            if let ast::Expr::Ident(ident) = self.source() {
                if ident.as_str() == new_name.as_str() {
                    // Warn on `import x as x`
                    engine.tracer.warn(warning!(
                        new_name.span(),
                        "unnecessary import rename to same name",
                    ));
                }
            }
            match &mut module {
                ImportedModule::Dynamic(dynamic) => {
                    let location = compiler.declare(self.span(), new_name.get().as_str());

                    dynamic.glob = Some(location);
                }
                ImportedModule::Static(module) => {
                    compiler.declare_default(
                        self.span(),
                        new_name.as_str(),
                        Value::Module(module.clone()),
                    );
                }
            }
        };

        // If the module is dynamic, generate the import instructions.
        // This involves:
        // - Calling the module instantiate op
        // - Importing all the items
        if let ImportedModule::Dynamic(dyn_) = module {
            let module_id = compiler.module(dyn_.clone());

            // If we glob import, we do not need to allocate a register.
            let target =
                if let Some(reg) = dyn_.glob { reg } else { compiler.allocate() };

            // Instantiate the module
            compiler.instantiate_module(
                self.span(),
                dyn_.path,
                module_id,
                target.clone(),
            );
        }

        Ok(ReadableGuard::None)
    }
}

trait ModuleLoad {
    fn load(
        &self,
        compiler: &mut Compiler,
        engine: &mut Engine,
    ) -> SourceResult<ImportedModule>;
}

impl ModuleLoad for ast::Expr<'_> {
    fn load(
        &self,
        compiler: &mut Compiler,
        engine: &mut Engine,
    ) -> SourceResult<ImportedModule> {
        match self {
            ast::Expr::Ident(ident) => ident.load(compiler, engine),
            ast::Expr::Str(str) => str.load(compiler, engine),
            ast::Expr::FieldAccess(field_access) => field_access.load(compiler, engine),
            other => {
                // For all other value (even invalid one), we defer to the dynamic implementation
                let value = other.compile_to_readable(compiler, engine)?;
                Ok(ImportedModule::Dynamic(DynamicModule::new(value)))
            }
        }
    }
}

impl ModuleLoad for ast::Ident<'_> {
    fn load(
        &self,
        compiler: &mut Compiler,
        engine: &mut Engine,
    ) -> SourceResult<ImportedModule> {
        let Some(readable) = compiler.read(self.span(), self.as_str(), false) else {
            return Err(unknown_variable(self.as_str())).at(self.span());
        };

        let forbidden =
            |name| error!(self.span(), "{} is not a valid import location", name);

        match readable.clone() {
            ReadableGuard::Register(register) => {
                // If we are a constant alias, we can try and import it.
                if let Some(variable) = compiler.resolve_var(&register) {
                    if variable.constant {
                        let default = compiler.resolve_default(&register);
                        return import_value(
                            engine,
                            &default.unwrap(),
                            self.span(),
                            true,
                        );
                    }
                }
            }
            ReadableGuard::Constant(constant) => {
                // If we are a constant, we can try and import it.
                let value = compiler.get_constant(&constant).unwrap();

                return import_value(engine, value, self.span(), true);
            }
            ReadableGuard::Captured(_) => {}
            ReadableGuard::String(string) => {
                // If we are a string, we can try and import it.
                let path = compiler.get_string(&string).unwrap();
                let Value::Str(path) = path else {
                    bail!(
                        self.span(),
                        "expected string, found {}",
                        path.ty().short_name()
                    );
                };

                return import(engine, path.as_str(), self.span());
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

                return import_value(engine, lib, self.span(), true);
            }
            ReadableGuard::GlobalModule => {
                return Ok(ImportedModule::Static(compiler.library().global.clone()))
            }
            ReadableGuard::Math(_) => bail!(forbidden("a math expression")),
            ReadableGuard::Bool(_) => bail!(forbidden("a boolean")),
            ReadableGuard::Label(_) => bail!(forbidden("a label")),
            ReadableGuard::None => bail!(forbidden("none")),
            ReadableGuard::Auto => bail!(forbidden("auto")),
        }

        // If it's not statically known, we import dynamically.
        Ok(ImportedModule::Dynamic(DynamicModule::new(readable)))
    }
}

impl ModuleLoad for ast::Str<'_> {
    fn load(
        &self,
        _: &mut Compiler,
        engine: &mut Engine,
    ) -> SourceResult<ImportedModule> {
        import(engine, self.get().as_str(), self.span())
    }
}

impl ModuleLoad for ast::FieldAccess<'_> {
    fn load(
        &self,
        compiler: &mut Compiler,
        engine: &mut Engine,
    ) -> SourceResult<ImportedModule> {
        let access = self.access(compiler, engine, false)?;

        // If we can resolve it as a constant, we can try and import it.
        let Some(resolved) = access.resolve(compiler)? else {
            // Otherwise we default to the dynamic case.
            let id = compiler.access(access);

            // Copy the value to a register.
            let reg = compiler.allocate();
            compiler.access_isr(self.span(), id, reg.clone());

            return Ok(ImportedModule::Dynamic(DynamicModule::new(reg)));
        };

        import_value(engine, &resolved, self.span(), true)
    }
}

pub fn import_value(
    engine: &mut Engine,
    value: &Value,
    span: Span,
    allow_scopes: bool,
) -> SourceResult<ImportedModule> {
    match value {
        Value::Module(module) => Ok(ImportedModule::Static(module.clone())),
        Value::Str(path) => import(engine, path.as_str(), span),
        Value::Func(func) => {
            let Some(scope) = func.scope() else {
                bail!(span, "cannot import from user-defined functions");
            };

            Ok(ImportedModule::Static(Module::new(func.name().unwrap(), scope.clone())))
        }
        v if allow_scopes => {
            bail!(span, "expected path, module, function, or type, found {}", v.ty())
        }
        v => bail!(span, "expected path or module, found {}", v.ty()),
    }
}

fn import(engine: &mut Engine, path: &str, span: Span) -> SourceResult<ImportedModule> {
    // Handle package and file imports.
    if path.starts_with('@') {
        let spec = path.parse::<PackageSpec>().at(span)?;
        import_package(engine, spec, span).map(ImportedModule::Static)
    } else {
        import_file(engine, path, span).map(ImportedModule::Static)
    }
}

trait Import {
    fn import(
        &mut self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        span: Span,
        import: &ast::Imports<'_>,
    ) -> SourceResult<()>;
}

impl Import for ImportedModule {
    fn import(
        &mut self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        span: Span,
        imports: &ast::Imports<'_>,
    ) -> SourceResult<()> {
        match self {
            ImportedModule::Dynamic(dynamic) => {
                dynamic.import(compiler, engine, span, imports)
            }
            ImportedModule::Static(module) => {
                module.import(compiler, engine, span, imports)
            }
        }
    }
}

impl Import for DynamicModule {
    fn import(
        &mut self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        span: Span,
        imports: &ast::Imports<'_>,
    ) -> SourceResult<()> {
        match imports {
            ast::Imports::Wildcard => {
                bail!(
                    span,
                    "cannot import all items from a dynamic module";
                    hint: "use `import \"...\" as x` to give a name to the module";
                    hint: "or use `import \"...\": a, b, c` to import specific items"
                );
            }
            ast::Imports::Items(items) => {
                for item in items.iter() {
                    match item {
                        ast::ImportItem::Simple(simple) => {
                            let name = simple.as_str();
                            self.imports.entry(name.into())
                                .and_modify(|_| {
                                    // If it already exists, warn the user.
                                    engine
                                        .tracer
                                        .warn(warning!(
                                            simple.span(), "importing {} multiple times", simple.as_str();
                                            hint: "remove the duplicate import statement",
                                        ));
                                })
                                .or_insert_with(|| {
                                    let alloc = compiler.declare(span, name);
                                    DynamicImport {
                                        span: simple.span(),
                                        name: name.into(),
                                        location: alloc,
                                    }
                                });
                        }
                        ast::ImportItem::Renamed(renamed) => {
                            let new_name = renamed.new_name().as_str();
                            let old_name = renamed.original_name().as_str();
                            if new_name == old_name {
                                engine.tracer.warn(warning!(
                                    renamed.span(),
                                    "unnecessary import rename to same name",
                                ));
                            }

                            self.imports.entry(old_name.into())
                                .and_modify(|_| {
                                    // If it already exists, warn the user.
                                    engine
                                        .tracer
                                        .warn(warning!(
                                            renamed.span(), "importing {} multiple times", renamed.original_name().as_str();
                                            hint: "remove the duplicate import statement",
                                        ));
                                })
                                .or_insert_with(|| {
                                    let alloc = compiler.declare(span, new_name);
                                    DynamicImport {
                                        span: renamed.span(),
                                        name: old_name.into(),
                                        location: alloc,
                                    }
                                });
                        }
                    }
                }

                Ok(())
            }
        }
    }
}

impl Import for Module {
    fn import(
        &mut self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        span: Span,
        imports: &ast::Imports<'_>,
    ) -> SourceResult<()> {
        let mut names = HashSet::new();
        match imports {
            ast::Imports::Wildcard => {
                // Import all names.
                for (name, value) in self.scope().iter() {
                    compiler.declare_default(span, name.as_str(), value.clone());
                }
            }
            ast::Imports::Items(items) => {
                for item in items.iter() {
                    match item {
                        ast::ImportItem::Simple(name) => {
                            if names.contains(name.as_str()) {
                                engine.tracer.warn(warning!(
                                    name.span(),
                                    "importing {} multiple times",
                                    name.as_str();
                                    hint: "remove the duplicate import statement",
                                ));
                            }

                            let Some(value) = self.scope().get(name.get().as_str())
                            else {
                                bail!(
                                    name.span(),
                                    "cannot find `{}` in module `{}`",
                                    name.get(),
                                    self.name().as_str()
                                );
                            };

                            names.insert(name.as_str());
                            compiler.declare_default(
                                span,
                                name.get().as_str(),
                                value.clone(),
                            );
                        }
                        ast::ImportItem::Renamed(renamed) => {
                            let original = renamed.original_name();
                            if names.contains(original.as_str()) {
                                engine.tracer.warn(warning!(
                                    renamed.span(),
                                    "importing {} multiple times",
                                    original.as_str();
                                    hint: "remove the duplicate import statement",
                                ));
                            }

                            if renamed.original_name().as_str()
                                == renamed.new_name().as_str()
                            {
                                engine.tracer.warn(warning!(
                                    renamed.span(),
                                    "unnecessary import rename to same name",
                                ));
                            }

                            let Some(value) = self.scope().get(original.get()) else {
                                bail!(
                                    original.span(),
                                    "cannot find `{}` in module `{}`",
                                    original.get(),
                                    self.name(),
                                )
                            };

                            compiler.declare_default(
                                renamed.new_name().span(),
                                renamed.new_name().get().as_str(),
                                value.clone(),
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Import an external package.
fn import_package(
    engine: &mut Engine,
    spec: PackageSpec,
    span: Span,
) -> SourceResult<Module> {
    // Evaluate the manifest.
    let manifest_id = FileId::new(Some(spec.clone()), VirtualPath::new("typst.toml"));
    let bytes = engine.world.file(manifest_id).at(span)?;
    let string = std::str::from_utf8(&bytes).map_err(FileError::from).at(span)?;
    let manifest: PackageManifest = toml::from_str(string)
        .map_err(|err| eco_format!("package manifest is malformed ({})", err.message()))
        .at(span)?;
    manifest.validate(&spec).at(span)?;

    // Evaluate the entry point.
    let entrypoint_id = manifest_id.join(&manifest.package.entrypoint);
    let source = engine.world.source(entrypoint_id).at(span)?;
    let point = || Tracepoint::Import;
    Ok(eval(
        engine.world,
        engine.route.track(),
        TrackedMut::reborrow_mut(&mut engine.tracer),
        &source,
    )
    .trace(engine.world, point, span)?
    .with_name(manifest.package.name.clone()))
}

/// Import a file from a path.
pub fn import_file(engine: &mut Engine, path: &str, span: Span) -> SourceResult<Module> {
    // Load the source file.
    let world = engine.world;
    let id = span.resolve_path(path).at(span)?;
    let source = world.source(id).at(span)?;

    // Prevent cyclic importing.
    if engine.route.contains(source.id()) {
        bail!(span, "cyclic import");
    }

    // Evaluate the file.
    let point = || Tracepoint::Import;
    eval(
        world,
        engine.route.track(),
        TrackedMut::reborrow_mut(&mut engine.tracer),
        &source,
    )
    .trace(world, point, span)
}
