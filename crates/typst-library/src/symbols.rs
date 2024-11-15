//! Modifiable symbols.

use crate::foundations::{
    category, Category, Func, Module, Scope, SymChar, Symbol, Value,
};

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
        extend_scope_from_codex_module(&mut scope, module);
        scope
    }
}

impl From<codex::Symbol> for Symbol {
    fn from(symbol: codex::Symbol) -> Self {
        match symbol {
            codex::Symbol::Single(c) => Symbol::single(c.into()),
            codex::Symbol::Multi(list) => Symbol::list(
                list.iter()
                    .map(|&(modifier, c)| (modifier, SymChar::pure(c)))
                    .collect(),
            ),
        }
    }
}

fn extend_scope_from_codex_module(scope: &mut Scope, module: codex::Module) {
    for (name, definition) in module.iter() {
        let value = match definition {
            codex::Def::Symbol(s) => Value::Symbol(s.into()),
            codex::Def::Module(m) => Value::Module(Module::new(name, m.into())),
        };
        scope.define(name, value);
    }
}

/// Hook up all `symbol` definitions.
pub(super) fn define(global: &mut Scope) {
    global.category(SYMBOLS);
    extend_scope_from_codex_module(global, codex::ROOT);
}

macro_rules! declare_callables {
    {
        $(
            $name:literal $( [$default:path] )? $( {
                $( $modifiers:literal => $handler:path ),*
                $(,)?
            } )?
        )*
    } => {
        &[
            $(
                (
                    $name,
                    &[
                        $( ("", <$default as ::typst_library::foundations::NativeFunc>::func), )?
                        $($(
                            (
                                $modifiers,
                                <$handler as ::typst_library::foundations::NativeFunc>::func,
                            )
                        ),*)?
                    ]
                )
            ),*
        ]
    };
}

#[allow(clippy::type_complexity)]
const MATH_CALLABLES: &[(&str, &[(&str, fn() -> Func)])] = declare_callables! {
    // Multiple modifiers should be specified in the same order as they are
    // defined in `codex`. For example, `r.l` instead of `l.r` in `arrow` below
    // would not work. If something is wrong, the test should detect it.
    "ceil" { "l" => crate::math::ceil }
    "floor" { "l" => crate::math::floor }
    "dash" { "en" => crate::math::accent::dash }
    "dot" {
        "op" => crate::math::accent::dot,
        "double" => crate::math::accent::dot_double,
        "triple" => crate::math::accent::dot_triple,
        "quad" => crate::math::accent::dot_quad,
    }
    "tilde" { "op" => crate::math::accent::tilde }
    "acute" [crate::math::accent::acute] {
        "double" => crate::math::accent::acute_double,
    }
    "breve" [crate::math::accent::breve]
    "caron" [crate::math::accent::caron]
    "hat" [crate::math::accent::hat]
    "diaer" [crate::math::accent::dot_double]
    "grave" [crate::math::accent::grave]
    "macron" [crate::math::accent::macron]
    "circle" { "stroked" => crate::math::accent::circle }
    "arrow" {
        "r" => crate::math::accent::arrow,
        "l" => crate::math::accent::arrow_l,
        "l.r" => crate::math::accent::arrow_l_r,
    }
    "harpoon" {
        "rt" => crate::math::accent::harpoon,
        "lt" => crate::math::accent::harpoon_lt,
    }
};

fn build_math_symbol(name: &str, symbol: codex::Symbol) -> Symbol {
    let Some(handlers) = MATH_CALLABLES.iter().find(|(n, _)| *n == name) else {
        return symbol.into();
    };
    match (symbol, handlers.1) {
        (codex::Symbol::Single(c), &[]) => Symbol::single(SymChar::pure(c)),
        (codex::Symbol::Single(c), &[("", func)]) => {
            Symbol::single(SymChar::with_func(c, func))
        }
        (codex::Symbol::Single(_), _) => {
            panic!("symbol {name} has no variant")
        }
        (codex::Symbol::Multi(list), handlers) => Symbol::list(
            list.iter()
                .map(|&(modifiers, c)| {
                    if let Some(handler) = handlers.iter().find(|(m, _)| *m == modifiers)
                    {
                        (modifiers, SymChar::with_func(c, handler.1))
                    } else {
                        (modifiers, SymChar::pure(c))
                    }
                })
                .collect(),
        ),
    }
}

/// Hook up all math `symbol` definitions, i.e., elements of the `sym` module.
pub(super) fn define_math(math: &mut Scope) {
    for (name, definition) in codex::SYM.iter() {
        match definition {
            codex::Def::Symbol(s) => {
                math.define(name, Value::Symbol(build_math_symbol(name, s)))
            }
            codex::Def::Module(_) => {}
        };
    }
}

#[cfg(test)]
mod tests {
    use super::{define_math, MATH_CALLABLES};
    use crate::foundations::{Scope, Value};

    #[test]
    fn all_handlers_are_valid_variants() {
        let mut math = Scope::new();
        define_math(&mut math);
        for (name, handlers) in MATH_CALLABLES {
            let Some(value) = math.get(name) else {
                panic!("{name} does not exist");
            };
            let Value::Symbol(symbol) = value else {
                panic!("{name} is not a symbol");
            };
            for (modifiers, _) in *handlers {
                let Ok(variant) = symbol.clone().modified(modifiers) else {
                    if modifiers.is_empty() {
                        panic!("{name} is not a valid symbol")
                    } else {
                        panic!("{name}.{modifiers} is not a valid variant")
                    }
                };
                if variant.func().is_err() {
                    if modifiers.is_empty() {
                        panic!("{name} should be callable, but is not")
                    } else {
                        panic!("{name}.{modifiers} should be callable, but is not (hint: the \
                        modifiers must be specified in the right order when defining a callable \
                        variant)")
                    }
                }
            }
        }
    }
}
