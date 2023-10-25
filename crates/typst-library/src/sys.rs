//! System-related things.

use std::collections::HashMap;

use ecow::EcoString;
use typst::eval::{Dict, Module, Scope, Value, Version};

/// Arguments for the `sys` module that handle implementation-specific behaviour.
#[derive(Clone, Default)]
pub struct SysArguments {
    /// A number of keyed inputs that can be provided by the platform and will appear
    /// as `sys.inputs`. The main expected usecase is scripting.
    pub inputs: HashMap<EcoString, Value>,
}

/// Hook up all sys definitions.
pub(super) fn define(global: &mut Scope, args: SysArguments) {
    global.category("system");
    global.define_module(module(args));
}

/// A module with system-related things.
fn module(args: SysArguments) -> Module {
    let mut scope = Scope::deduplicating();
    scope.category("sys");
    scope.define(
        "version",
        Version::from_iter([
            env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap(),
            env!("CARGO_PKG_VERSION_MINOR").parse::<u32>().unwrap(),
            env!("CARGO_PKG_VERSION_PATCH").parse::<u32>().unwrap(),
        ]),
    );
    scope.define(
        "inputs",
        Dict::from_iter(args.inputs.into_iter().map(|(k, v)| (k.into(), v))),
    );
    Module::new("sys", scope)
}
