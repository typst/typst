//! System-related things.

use crate::foundations::{Dict, Module, Scope, Version};

/// A module with system-related things.
pub fn module(inputs: Dict) -> Module {
    let typst_version = typst_utils::TypstVersion::new();
    let version = Version::from_iter([
        typst_version.major(),
        typst_version.minor(),
        typst_version.patch(),
    ]);

    let mut scope = Scope::deduplicating();
    scope.define("version", version);
    scope.define("inputs", inputs);
    Module::new("sys", scope)
}
