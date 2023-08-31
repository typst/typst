//! Information about typst itself

use typst::eval::{Module, Scope, Value, Version};

/// Construct the module with information about typst itself
pub fn module() -> Module {
    let mut scope = Scope::deduplicating();

    let version = Version::new([
        env!("CARGO_PKG_VERSION_MAJOR").parse::<i64>().unwrap(),
        env!("CARGO_PKG_VERSION_MINOR").parse::<i64>().unwrap(),
        env!("CARGO_PKG_VERSION_PATCH").parse::<i64>().unwrap(),
    ])
    .unwrap();

    scope.define("version", Value::Version(version));
    scope.define("commit", option_env!("TYPST_COMMIT"));

    Module::new("info").with_scope(scope)
}
