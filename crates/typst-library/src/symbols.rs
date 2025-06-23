//! Modifiable symbols.

use crate::foundations::{Module, Scope, Symbol, Value};

/// Hook up all `symbol` definitions.
pub(super) fn define(global: &mut Scope) {
    global.start_category(crate::Category::Symbols);
    extend_scope_from_codex_module(global, codex::ROOT);
    global.reset_category();
}

/// Hook up all math `symbol` definitions, i.e., elements of the `sym` module.
pub(super) fn define_math(math: &mut Scope) {
    extend_scope_from_codex_module(math, codex::SYM);
}

fn extend_scope_from_codex_module(scope: &mut Scope, module: codex::Module) {
    for (name, binding) in module.iter() {
        let value = match binding.def {
            codex::Def::Symbol(s) => Value::Symbol(s.into()),
            codex::Def::Module(m) => Value::Module(Module::new(name, m.into())),
        };

        let scope_binding = scope.define(name, value);
        if let Some(message) = binding.deprecation {
            scope_binding.deprecated(message);
        }
    }
}

impl From<codex::Module> for Scope {
    fn from(module: codex::Module) -> Scope {
        let mut scope = Self::new();
        extend_scope_from_codex_module(&mut scope, module);
        scope
    }
}

impl From<codex::Symbol> for Symbol {
    fn from(symbol: codex::Symbol) -> Self {
        match symbol {
            codex::Symbol::Single(value) => Symbol::single(value),
            codex::Symbol::Multi(list) => Symbol::list(list),
        }
    }
}
