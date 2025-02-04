//! Modifiable symbols.

use crate::foundations::{category, Category, Module, Scope, Symbol, Value};

/// These two modules give names to symbols and emoji to make them easy to
/// insert with a normal keyboard. Alternatively, you can also always directly
/// enter Unicode symbols into your text and formulas. In addition to the
/// symbols listed below, math mode defines `dif` and `Dif`. These are not
/// normal symbol values because they also affect spacing and font style.
#[category]
pub static SYMBOLS: Category;

/// Hook up all `symbol` definitions.
pub(super) fn define(global: &mut Scope) {
    global.start_category(SYMBOLS);
    extend_scope_from_codex_module(global, codex::ROOT);
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
            codex::Symbol::Single(c) => Symbol::single(c),
            codex::Symbol::Multi(list) => Symbol::list(list),
        }
    }
}
