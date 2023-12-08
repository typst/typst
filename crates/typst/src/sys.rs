//! System-related things.

use crate::foundations::{Dict, Module, Scope, Version};

/// Arguments for the `sys` module that handle implementation-specific behaviour.
#[derive(Clone, Default)]
pub struct SysArguments {
    /// A number of keyed inputs that can be provided by the platform and will appear
    /// as `sys.inputs`. The main expected usecase is scripting.
    pub inputs: Dict,
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
    scope.define("inputs", args.inputs);

    Module::new("sys", scope)
}
