use std::sync::Arc;
use typst_syntax::{ast, Source};

use crate::compiler::CompileTopLevel;
use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::World;
use crate::util::LazyHash;

use super::{CompiledCode, Compiler, Export};

/// A module that has been compiled but is not yet executed.
#[derive(Clone, Hash)]
pub struct CompiledModule {
    /// The common data.
    pub inner: Arc<LazyHash<CompiledCode>>,
}

impl CompiledModule {
    pub fn new(resource: CompiledCode) -> Self {
        Self { inner: Arc::new(LazyHash::new(resource)) }
    }
}

#[typst_macros::time(name = "module compile", span = source.root().span())]
pub fn compile_module(
    source: &Source,
    engine: &mut Engine,
) -> SourceResult<CompiledModule> {
    // Parse the source.
    let root = source.root();

    // Check for well-formedness unless we are in trace mode.
    let errors = root.errors();
    if !errors.is_empty() {
        return Err(errors.into_iter().map(Into::into).collect());
    }

    // Evaluate the module.
    let markup = root.cast::<ast::Markup>().unwrap();

    // Assemble the module.
    let name = source
        .id()
        .vpath()
        .as_rootless_path()
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    // Prepare Compiler.
    let mut compiler = Compiler::module(engine.world.library().clone().into_inner());

    // Compile the module.
    markup.compile_top_level(engine, &mut compiler)?;

    let scopes = compiler.scope.borrow();
    let exports = scopes
        .variables
        .iter()
        .map(|(name, var)| Export {
            name: *name,
            value: var.register.as_readable(),
            span: var.span,
        })
        .collect();

    drop(scopes);
    Ok(CompiledModule::new(compiler.finish_module(root.span(), &*name, exports)))
}
