//! System-related things.

use crate::foundations::{Dict, Module, Scope, Version};
use crate::OutputFormat;

/// A module with system-related things.
pub fn module(inputs: Dict, output_format: OutputFormat) -> Module {
    let mut scope = Scope::deduplicating();
    scope.define(
        "version",
        Version::from_iter([
            env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap(),
            env!("CARGO_PKG_VERSION_MINOR").parse::<u32>().unwrap(),
            env!("CARGO_PKG_VERSION_PATCH").parse::<u32>().unwrap(),
        ]),
    );
    scope.define("inputs", inputs);
    scope.define("output-format", output_format.to_string());
    Module::new("sys", scope)
}
