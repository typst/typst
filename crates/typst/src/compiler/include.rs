use comemo::TrackedMut;
use ecow::{eco_format, EcoString};
use serde::{Deserialize, Serialize};
use typst_syntax::ast::AstNode;
use typst_syntax::{ast, FileId, PackageSpec, PackageVersion, Span, VirtualPath};

use crate::diag::{
    bail, error, At, FileError, SourceResult, StrResult, Trace, Tracepoint,
};
use crate::engine::Engine;
use crate::foundations::{Module, Value};
use crate::vm::eval;
use crate::World;

use super::{Compile, Compiler, ReadableGuard, WritableGuard};

impl Compile for ast::ModuleImport<'_> {
    type Output = ();
    type IntoOutput = ();

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        _: Self::Output,
    ) -> SourceResult<()> {
        self.compile(engine, compiler)
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Load the actual module.
        let module = self.source().load(engine, compiler)?;

        // Handle imports.
        if let Some(imports) = self.imports() {
            match imports {
                ast::Imports::Wildcard => {
                    // Import all names.
                    for (name, value) in module.scope().iter() {
                        compiler.declare_default(
                            self.span(),
                            name.clone(),
                            value.clone(),
                        );
                    }
                }
                ast::Imports::Items(items) => {
                    for item in items.iter() {
                        match item {
                            ast::ImportItem::Simple(name) => {
                                let Some(value) = module.scope().get(name.get()) else {
                                    bail!(
                                        name.span(),
                                        "cannot find {} in module {}",
                                        name.get(),
                                        module.name()
                                    )
                                };

                                compiler.declare_default(
                                    self.span(),
                                    name.get().to_owned(),
                                    value.clone(),
                                );
                            }
                            ast::ImportItem::Renamed(renamed) => {
                                let original = renamed.original_name();
                                let Some(value) = module.scope().get(original.get())
                                else {
                                    bail!(
                                        original.span(),
                                        "cannot find {} in module {}",
                                        original.get(),
                                        module.name()
                                    )
                                };

                                compiler.declare_default(
                                    renamed.new_name().span(),
                                    renamed.new_name().get().clone(),
                                    value.clone(),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Handle renaming.
        if let Some(rename) = self.new_name() {
            compiler.declare_default(
                rename.span(),
                rename.get().to_owned(),
                Value::Module(module),
            );
        }

        Ok(())
    }
}

impl Compile for ast::ModuleInclude<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        if let Some(output) = output {
            let module = self.source().load(engine, compiler)?;
            let constant = compiler.const_(module);

            compiler.copy(self.span(), constant, &output);
        }

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut super::Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let module = self.source().load(engine, compiler)?;
        let constant = compiler.const_(module);

        Ok(ReadableGuard::Constant(constant))
    }
}

trait ModuleLoad {
    fn load(&self, engine: &mut Engine, compiler: &mut Compiler) -> SourceResult<Module>;
}

impl ModuleLoad for ast::Expr<'_> {
    fn load(&self, engine: &mut Engine, compiler: &mut Compiler) -> SourceResult<Module> {
        let span = self.span();
        let forbidden =
            |name: &str| error!(span, "{} is not a valid import source", name);

        match self {
            ast::Expr::Ident(ident) => ident.load(engine, compiler),
            ast::Expr::Str(str) => str.load(engine, compiler),
            ast::Expr::FieldAccess(field_access) => field_access.load(engine, compiler),
            ast::Expr::Dict(_) => bail!(forbidden("a dictionary")),
            ast::Expr::Text(_) => bail!(forbidden("a text element")),
            ast::Expr::Space(_) => bail!(forbidden("a space element")),
            ast::Expr::Linebreak(_) => bail!(forbidden("a linebreak element")),
            ast::Expr::Parbreak(_) => bail!(forbidden("a parbreak element")),
            ast::Expr::Escape(_) => bail!(forbidden("an escaped character")),
            ast::Expr::Shorthand(_) => bail!(forbidden("a shorthand")),
            ast::Expr::SmartQuote(_) => bail!(forbidden("a smart quote")),
            ast::Expr::Strong(_) => bail!(forbidden("a strong element")),
            ast::Expr::Emph(_) => bail!(forbidden("an emphasis element")),
            ast::Expr::Raw(_) => bail!(forbidden("a raw element")),
            ast::Expr::Link(_) => bail!(forbidden("a link element")),
            ast::Expr::Label(_) => bail!(forbidden("a label")),
            ast::Expr::Ref(_) => bail!(forbidden("a reference")),
            ast::Expr::Heading(_) => bail!(forbidden("a heading")),
            ast::Expr::List(_) => bail!(forbidden("a list item")),
            ast::Expr::Enum(_) => bail!(forbidden("an enumerated list item")),
            ast::Expr::Term(_) => bail!(forbidden("a term")),
            ast::Expr::Equation(_) => bail!(forbidden("an equation")),
            ast::Expr::Math(_)
            | ast::Expr::MathIdent(_)
            | ast::Expr::MathAlignPoint(_)
            | ast::Expr::MathDelimited(_)
            | ast::Expr::MathAttach(_)
            | ast::Expr::MathPrimes(_)
            | ast::Expr::MathFrac(_)
            | ast::Expr::MathRoot(_) => bail!(forbidden("a math element")),
            ast::Expr::None(_) => bail!(forbidden("none")),
            ast::Expr::Auto(_) => bail!(forbidden("auto")),
            ast::Expr::Bool(_) => bail!(forbidden("a boolean")),
            ast::Expr::Int(_) => bail!(forbidden("an integer")),
            ast::Expr::Float(_) => bail!(forbidden("a float")),
            ast::Expr::Numeric(_) => bail!(forbidden("a numeric")),
            ast::Expr::Code(_) => bail!(forbidden("a code block")),
            ast::Expr::Content(_) => bail!(forbidden("a content block")),
            ast::Expr::Parenthesized(paren) => paren.expr().load(engine, compiler),
            ast::Expr::Array(_) => bail!(forbidden("an array")),
            ast::Expr::Unary(_) => bail!(forbidden("a unary expression")),
            ast::Expr::Binary(_) => bail!(forbidden("a binary expression")),
            ast::Expr::FuncCall(_) => bail!(forbidden("a function call")),
            ast::Expr::Closure(_) => bail!(forbidden("a closure")),
            ast::Expr::Let(_) => bail!(forbidden("a let statement")),
            ast::Expr::DestructAssign(_) => {
                bail!(forbidden("a destructuring assignment"))
            }
            ast::Expr::Set(_) => bail!(forbidden("a set statement")),
            ast::Expr::Show(_) => bail!(forbidden("a show statement")),
            ast::Expr::Conditional(_) => bail!(forbidden("a conditional expression")),
            ast::Expr::While(_) => bail!(forbidden("a while loop")),
            ast::Expr::For(_) => bail!(forbidden("a for loop")),
            ast::Expr::Import(_) => bail!(forbidden("an import statement")),
            ast::Expr::Include(_) => bail!(forbidden("an include statement")),
            ast::Expr::Break(_) => bail!(forbidden("a break statement")),
            ast::Expr::Continue(_) => bail!(forbidden("a continue statement")),
            ast::Expr::Return(_) => bail!(forbidden("a return statement")),
        }
    }
}

impl ModuleLoad for ast::Str<'_> {
    fn load(&self, engine: &mut Engine, _: &mut Compiler) -> SourceResult<Module> {
        // Handle package and file imports.
        let path = self.get();
        if path.starts_with('@') {
            let spec = path.parse::<PackageSpec>().at(self.span())?;
            import_package(engine, spec, self.span())
        } else {
            import_file(engine, &path, self.span())
        }
    }
}

fn get_field(name: &str, compiler: &mut Compiler) -> StrResult<Value> {
    let Some(value) = compiler.default(name)? else {
        bail!("cannot import from a dynamic scope");
    };

    Ok(value)
}

impl ModuleLoad for ast::Ident<'_> {
    fn load(&self, _: &mut Engine, compiler: &mut Compiler) -> SourceResult<Module> {
        let field = get_field(self.get(), compiler).at(self.span())?;
        if let Value::Module(module) = field {
            return Ok(module.clone());
        } else {
            bail!(self.span(), "expected a module, found {}", field.ty());
        }
    }
}

impl ModuleLoad for ast::FieldAccess<'_> {
    fn load(&self, _: &mut Engine, compiler: &mut Compiler) -> SourceResult<Module> {
        fn parse_tree<'a, 'b>(
            access: &ast::FieldAccess<'a>,
            out: &'b mut Vec<ast::Ident<'a>>,
        ) -> SourceResult<()> {
            match access.target() {
                ast::Expr::Ident(ident) => out.push(ident),
                ast::Expr::FieldAccess(access) => {
                    parse_tree(&access, out)?;
                    out.push(access.field());
                }
                _ => bail!(
                    access.target().span(),
                    "expected an identifier or field access"
                ),
            }

            Ok(())
        }

        // We create the tree of field accesses.
        let mut tree = Vec::new();
        parse_tree(self, &mut tree)?;

        // We resolve each field access.
        let mut iter = tree.into_iter();
        let first = get_field(iter.next().unwrap().get(), compiler).at(self.span())?;
        let Value::Module(first) = &first else {
            bail!(self.span(), "expected a module, found {}", first.ty().short_name());
        };

        let mut scope = first;
        for ident in iter {
            let field = scope.field(ident.get()).at(ident.span())?;
            if let Value::Module(module) = field {
                scope = module;
            } else {
                bail!(ident.span(), "expected a module, found {}", field.ty());
            }
        }

        return Ok(scope.clone());
    }
}

/// Import an external package.
fn import_package(
    engine: &mut Engine,
    spec: PackageSpec,
    span: Span,
) -> SourceResult<Module> {
    // Evaluate the manifest.
    let world = engine.world;
    let manifest_id = FileId::new(Some(spec.clone()), VirtualPath::new("typst.toml"));
    let bytes = engine.world.file(manifest_id).at(span)?;
    let manifest = PackageManifest::parse(&bytes).at(span)?;
    manifest.validate(engine, &spec).at(span)?;

    // Evaluate the entry point.
    let entrypoint_id = manifest_id.join(&manifest.package.entrypoint);
    let source = engine.world.source(entrypoint_id).at(span)?;
    let point = || Tracepoint::Import;
    Ok(eval(
        world,
        engine.route.track(),
        TrackedMut::reborrow_mut(&mut engine.tracer),
        &source,
    )
    .trace(world, point, span)?
    .with_name(manifest.package.name))
}

/// Import a file from a path.
fn import_file(engine: &mut Engine, path: &str, span: Span) -> SourceResult<Module> {
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

/// A parsed package manifest.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
struct PackageManifest {
    /// Details about the package itself.
    package: PackageInfo,
}

/// The `package` key in the manifest.
///
/// More fields are specified, but they are not relevant to the compiler.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
struct PackageInfo {
    /// The name of the package within its namespace.
    name: EcoString,
    /// The package's version.
    version: PackageVersion,
    /// The path of the entrypoint into the package.
    entrypoint: EcoString,
    /// The minimum required compiler version for the package.
    compiler: Option<PackageVersion>,
}

impl PackageManifest {
    /// Parse the manifest from raw bytes.
    fn parse(bytes: &[u8]) -> StrResult<Self> {
        let string = std::str::from_utf8(bytes).map_err(FileError::from)?;
        toml::from_str(string).map_err(|err| {
            eco_format!("package manifest is malformed: {}", err.message())
        })
    }

    /// Ensure that this manifest is indeed for the specified package.
    fn validate(&self, _: &mut Engine, spec: &PackageSpec) -> StrResult<()> {
        if self.package.name != spec.name {
            bail!("package manifest contains mismatched name `{}`", self.package.name);
        }

        if self.package.version != spec.version {
            bail!(
                "package manifest contains mismatched version {}",
                self.package.version
            );
        }

        if let Some(compiler) = self.package.compiler {
            let current = PackageVersion::compiler();
            if current < compiler {
                bail!(
                    "package requires typst {compiler} or newer \
                     (current version is {current})"
                );
            }
        }

        Ok(())
    }
}
