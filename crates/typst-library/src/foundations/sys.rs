//! System-related things.

use crate::foundations::{Dict, Module, Scope, Version};

/// A module with system-related things.
pub fn module(inputs: Dict) -> Module {
    let mut scope = Scope::deduplicating();
    scope.define(
        "version",
        Version::try_from(typst_syntax::TypstVersion::new())
            .expect("Typst compiler version must be valid"),
    );
    scope.define("inputs", inputs);
    Module::new("sys", scope)
}
