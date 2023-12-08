//! System-related things.

use std::collections::HashMap;

use ecow::EcoString;

use crate::foundations::{Dict, Module, Scope, Value, Version};

/// Arguments for the `sys` module that handle implementation-specific behaviour.
#[derive(Clone, Default)]
pub struct SysArguments {
    /// A number of keyed inputs that can be provided by the platform and will appear
    /// as `sys.inputs`. The main expected usecase is scripting.
    pub inputs: HashMap<EcoString, Value>,
}

/// A module with system-related things.
pub fn module(args: SysArguments) -> Module {
    let mut scope = Scope::deduplicating();
    scope.define(
        "version",
        Version::from_iter([
            env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap(),
            env!("CARGO_PKG_VERSION_MINOR").parse::<u32>().unwrap(),
            env!("CARGO_PKG_VERSION_PATCH").parse::<u32>().unwrap(),
        ]),
    );
    let inputs = Dict::from_iter(args.inputs.into_iter().map(|(k, v)| (k.into(), v)));
    scope.define("inputs", inputs);

    Module::new("sys", scope)
}
