//! Modifiable symbols.

use crate::foundations::{category, Category, Module, Scope, Symbol, Value};

/// These two modules give names to symbols and emoji to make them easy to
/// insert with a normal keyboard. Alternatively, you can also always directly
/// enter Unicode symbols into your text and formulas. In addition to the
/// symbols listed below, math mode defines `dif` and `Dif`. These are not
/// normal symbol values because they also affect spacing and font style.
#[category]
pub static SYMBOLS: Category;

impl From<codex::Module> for Scope {
    fn from(module: codex::Module) -> Scope {
        let mut scope = Self::new();
        extend_scope_from_codex_module(&mut scope, module, false); // TODO: not sure about `false` here.
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

fn extend_scope_from_codex_module(scope: &mut Scope, module: codex::Module, math: bool) {
    for (name, definition) in module.iter() {
        let value = match definition {
            codex::Def::Symbol(s) => {
                let s: Symbol = s.into();
                let s = if math { s.for_math() } else { s };
                Value::Symbol(s)
            }
            codex::Def::Module(m) => Value::Module(Module::new(name, m.into())),
        };
        scope.define(name, value);
    }
}

/// Hook up all `symbol` definitions.
pub(super) fn define(global: &mut Scope) {
    global.category(SYMBOLS);
    extend_scope_from_codex_module(global, codex::ROOT, false);
}

/// Hook up all math `symbol` definitions, i.e., elements of the `sym` module.
pub(super) fn define_math(math: &mut Scope) {
    extend_scope_from_codex_module(math, codex::SYM, true);
}
