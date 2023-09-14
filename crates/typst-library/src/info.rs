//! Information about typst itself

use typst::eval::{Module, Scope, Value, Version};

/// Construct the module with information about typst itself
pub fn module() -> Module {
    let mut scope = Scope::deduplicating();

    let version = Version::from_iter([
        env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap(),
        env!("CARGO_PKG_VERSION_MINOR").parse::<u32>().unwrap(),
        env!("CARGO_PKG_VERSION_PATCH").parse::<u32>().unwrap(),
    ]);

    scope.define("version", Value::Version(version));

    Module::new("info", scope)
}
