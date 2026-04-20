//! Exposes metadata about the standard library to the docs system.
//!
//! Cooperates with `docs/components/reflect.typ`.

use std::sync::LazyLock;

use ecow::EcoString;
use heck::ToTitleCase;
use rustc_hash::FxHashMap;
use typst::diag::bail;
use typst::foundations::{
    Array, CastInfo, Dict, Func, IntoValue, Module, NativeParamInfo, Repr, Str, Symbol,
    Type, Value, cast, dict, func,
};
use typst_utils::DefSite;
use unicode_math_class::MathClass;
use unicode_segmentation::UnicodeSegmentation;
use unscanny::Scanner;

/// Provides details about a binding in a module.
#[func]
pub fn binding(module: Module, name: EcoString) -> Option<Dict> {
    let binding = module.scope().get(&name)?;
    Some(dict! {
        "category" => binding.category().map(|c| c.name()),
        "deprecation" => binding.deprecation().map(|d| dict! {
            "message" => d.message(),
            "until" => d.until(),
        })
    })
}

/// Provides details about a value.
#[func]
pub fn describe(value: Value) -> Option<Dict> {
    match &value {
        Value::Func(func) => Some(describe_func(func)),
        Value::Type(ty) => Some(describe_ty(ty)),
        Value::Symbol(symbol) => Some(describe_symbol(symbol)),
        _ => None,
    }
}

/// Provides details about a native function.
fn describe_func(func: &Func) -> Dict {
    dict! {
        "name" => func.name(),
        "title" => func.title(),
        "docs" => func.docs(),
        "def-site" => func.def_site().map(describe_def_site),
        "element" => func.to_element().is_some(),
        "contextual" => func.contextual(),
        "params" => func
            .params()
            .filter_map(|info| {
                info.to_native().map(|info| describe_param(info).into_value())
            })
            .collect::<Array>(),
        "returns" => func.returns().map(describe_cast_info),
        "keywords" => func.keywords(),
        "scope" => func.scope().map(|s| Module::anonymous(s.clone())),
    }
}

/// Provides details about a parameter of a native function.
fn describe_param(param: &NativeParamInfo) -> Dict {
    dict! {
        "name" => param.name,
        "docs" => param.docs,
        "def-site" => param.def_site.map(describe_def_site),
        "input" => describe_cast_info(&param.input),
        "default" => param.default.map(|f| f()),
        "positional" => param.positional,
        "named" => param.named,
        "variadic" => param.variadic,
        "required" => param.required,
        "settable" => param.settable,
    }
}

/// Provides details about a native type.
fn describe_ty(ty: &Type) -> Dict {
    dict! {
        "short-name" => ty.short_name(),
        "long-name" => ty.long_name(),
        "title" => ty.title(),
        "docs" => ty.docs(),
        "def-site" => describe_def_site(ty.def_site()),
        "keywords" => ty.keywords(),
        "constructor" => ty.constructor().ok(),
        "scope" => Module::anonymous(ty.scope().clone()),
    }
}

/// Provides details about a built-in symbol.
fn describe_symbol(symbol: &Symbol) -> Dict {
    let variants = symbol
        .variants()
        .map(|(variant, value, deprecation)| {
            dict! {
                "variant" => variant.as_str(),
                "value" => value,
                "deprecation" => deprecation,
            }
            .into_value()
        })
        .collect::<Array>();
    dict! { "variants" => variants }
}

/// Provides details where a native definition is located in the sources.
fn describe_def_site(site: DefSite) -> Dict {
    dict! {
        "path" => site.path,
        "key" => site.key,
    }
}

/// Provides details about acceptable values for a parameter or return value.
fn describe_cast_info(info: &CastInfo) -> Dict {
    match info {
        CastInfo::Any => dict! {
            "kind" => "any"
        },
        CastInfo::Value(value, details) => dict! {
            "kind" => "value",
            "value" => value.clone(),
            "details" => *details,
        },
        CastInfo::Type(ty) => dict! {
            "kind" => "type",
            "ty" => *ty,
        },
        CastInfo::Union(infos) => dict! {
            "kind" => "union",
            "infos" => infos
                .iter()
                .map(describe_cast_info)
                .map(Value::Dict)
                .collect::<Array>(),
        },
    }
}

/// Returns the math class of a character.
///
/// Returns `None` if the provided string has more than one char or if it does
/// not have a math class.
#[func]
pub fn math_class(c: Cluster) -> Option<MathClass> {
    typst_utils::default_math_class(c.primary)
}

/// Returns whether the given string can be used as an accent with the
/// `math.accent` function.
#[func]
pub fn is_accent(s: Str) -> bool {
    typst::math::Accent::combining(&s).is_some()
}

/// Returns the full title-cased name of the given character in Unicode.
#[func]
pub fn unicode_name(c: Cluster) -> Option<String> {
    unicode_names2::name(c.primary).map(|n| n.to_string().to_title_case())
}

/// Returns the name of a character in LaTeX.
#[func]
pub fn latex_name(c: Cluster) -> Option<Str> {
    static NAMES: LazyLock<FxHashMap<u32, &'static str>> = LazyLock::new(|| {
        let data = typst_dev_assets::get("latex/unicode-math-table.tex").unwrap();
        let mut map = FxHashMap::default();
        for line in std::str::from_utf8(data).unwrap().lines() {
            let mut s = Scanner::new(line);
            if !s.eat_if("\\UnicodeMathSymbol{\"") {
                continue;
            }

            let code =
                u32::from_str_radix(s.eat_while(char::is_ascii_hexdigit), 16).unwrap();
            s.eat_if("}{");
            let name = s.eat_while(|c: char| c == '\\' || c.is_alphabetic());
            map.insert(code, name);
        }
        map
    });
    NAMES.get(&(c.primary as u32)).copied().map(Into::into)
}

/// A grapheme cluster with an extracted primary char.
pub struct Cluster {
    primary: char,
}

cast! {
    Cluster,
    s: Str => {
        if s.graphemes(true).count() != 1 {
            bail!("expected exactly one grapheme: `{}`", s.repr());
        }
        // Not every kind of cluster has a well-defined "base", but for our
        // purposes (getting the Unicode Name etc. in presence of a variation
        // selection) this is good enough.
        Self { primary: s.chars().next().unwrap() }
    }
}

/// Returns the list of available built-in shorthands for markup and math.
pub fn shorthands() -> Dict {
    dict! {
        "markup" => shorthand_dict(typst::syntax::ast::Shorthand::LIST),
        "math" => shorthand_dict(typst::syntax::ast::MathShorthand::LIST),
    }
}

fn shorthand_dict(list: &[(&'static str, char)]) -> Dict {
    list.iter().map(|&(s, c)| (Str::from(c), s.into_value())).collect()
}

/// Returns whether the given attribute names is one of the global ones in HTML
/// (as opposed to being element-specific).
#[func]
pub fn is_global_html_attr(name: EcoString) -> bool {
    use typst_assets::html as data;
    data::ATTRS[..data::ATTRS_GLOBAL]
        .iter()
        .any(|global| global.name == name)
}
