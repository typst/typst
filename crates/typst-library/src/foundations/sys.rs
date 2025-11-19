//! System-related things.

use crate::foundations::{Dict, Module, Scope, Version};

/// A module with system-related things.
///
/// # Panics
///
/// If any version component of the Typst version overflows the numeric range of the respective
/// [`Version`] component here.
pub fn module(inputs: Dict) -> Module {
    let typst_version = typst_utils::TypstVersion::new();
    let version = Version::from_iter([
        u32::try_from(typst_version.major()).unwrap(),
        u32::try_from(typst_version.minor()).unwrap(),
        u32::try_from(typst_version.patch()).unwrap(),
    ]);

    let mut scope = Scope::deduplicating();
    scope.define("version", version);
    scope.define("inputs", inputs);
    Module::new("sys", scope)
}
