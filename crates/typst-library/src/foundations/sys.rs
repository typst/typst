//! System-related things.

use crate::foundations::{BindingDocumentation, Dict, Module, Scope, Since, Version};

/// A module with system-related things.
pub fn module(inputs: Dict) -> Module {
    let typst_version = typst_utils::version();
    let version = Version::from_iter([
        typst_version.major(),
        typst_version.minor(),
        typst_version.patch(),
    ]);

    let mut scope = Scope::deduplicating();
    scope
        .define("version", version)
        .with_documentation(BindingDocumentation {
            name: "version",
            title: "Compiler Version",
            docs: "
            The currently active Typst compiler version.

            ```example
            #sys.version
            ```
            ",
            since: Some(Since::Version([0, 9, 0])),
            keywords: &[],
            def_site: None,
        });
    scope
        .define("inputs", inputs)
        .with_documentation(BindingDocumentation {
            name: "inputs",
            title: "CLI inputs",
            docs: r#"
            Makes external inputs available to the project.

            An input specified in the command line as `--input key=value` becomes
            available under `{sys.inputs.key}` as `{"value"}`. To include spaces in
            the value, it may be enclosed with single or double quotes.

            The value is always of type @str[string]. More complex data may be
            parsed manually using functions like @json.
            "#,
            since: Some(Since::Version([0, 11, 0])),
            keywords: &[],
            def_site: None,
        });
    Module::new("sys", scope)
}
