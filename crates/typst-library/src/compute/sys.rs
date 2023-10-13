//! System-related things.

use typst::eval::{Module, Scope, Version};

/// Hook up all calculation definitions.
pub(super) fn define(global: &mut Scope) {
    global.category("sys");
    global.define_module(module());
}

/// A module with system-related things.
fn module() -> Module {
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
    Module::new("sys", scope)
}
