//! Information about typst itself

use typst::eval::{Dict, IntoValue, Module, Scope};

/// Construct the module with information about typst itself
pub fn module() -> Module {
    let mut scope = Scope::deduplicating();

    let mut version_dict = Dict::new();
    version_dict.insert(
        "major".into(),
        env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap().into_value(),
    );
    version_dict.insert(
        "minor".into(),
        env!("CARGO_PKG_VERSION_MINOR").parse::<u32>().unwrap().into_value(),
    );
    version_dict.insert(
        "patch".into(),
        env!("CARGO_PKG_VERSION_PATCH").parse::<u32>().unwrap().into_value(),
    );

    scope.define("version", version_dict);
    scope.define("version_string", env!("CARGO_PKG_VERSION"));
    scope.define("commit", option_env!("TYPST_COMMIT"));

    Module::new("info").with_scope(scope)
}
