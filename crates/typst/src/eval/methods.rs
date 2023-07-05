//! Methods on values.

use ecow::EcoString;

use super::{Args, IntoValue, Str, Value, Vm};
use crate::diag::{At, SourceResult};
use crate::eval::Datetime;
use crate::model::{Location, Selector};
use crate::syntax::Span;

/// Call a method on a value.
pub fn call(
    vm: &mut Vm,
    value: Value,
    method: &str,
    mut args: Args,
    span: Span,
) -> SourceResult<Value> {
    let name = value.type_name();
    let missing = || Err(missing_method(name, method)).at(span);

    let output = match value {
        Value::Color(color) => match method {
            "lighten" => color.lighten(args.expect("amount")?).into_value(),
            "darken" => color.darken(args.expect("amount")?).into_value(),
            "negate" => color.negate().into_value(),
            _ => return missing(),
        },

        Value::Str(string) => match method {
            "len" => string.len().into_value(),
            "first" => string.first().at(span)?.into_value(),
            "last" => string.last().at(span)?.into_value(),
            "at" => {
                let index = args.expect("index")?;
                let default = args.named::<EcoString>("default")?;
                string.at(index, default.as_deref()).at(span)?.into_value()
            }
            "slice" => {
                let start = args.expect("start")?;
                let mut end = args.eat()?;
                if end.is_none() {
                    end = args.named("count")?.map(|c: i64| start + c);
                }
                string.slice(start, end).at(span)?.into_value()
            }
            "clusters" => string.clusters().into_value(),
            "codepoints" => string.codepoints().into_value(),
            "contains" => string.contains(args.expect("pattern")?).into_value(),
            "starts-with" => string.starts_with(args.expect("pattern")?).into_value(),
            "ends-with" => string.ends_with(args.expect("pattern")?).into_value(),
            "find" => string.find(args.expect("pattern")?).into_value(),
            "position" => string.position(args.expect("pattern")?).into_value(),
            "match" => string.match_(args.expect("pattern")?).into_value(),
            "matches" => string.matches(args.expect("pattern")?).into_value(),
            "replace" => {
                let pattern = args.expect("pattern")?;
                let with = args.expect("string or function")?;
                let count = args.named("count")?;
                string.replace(vm, pattern, with, count)?.into_value()
            }
            "trim" => {
                let pattern = args.eat()?;
                let at = args.named("at")?;
                let repeat = args.named("repeat")?.unwrap_or(true);
                string.trim(pattern, at, repeat).into_value()
            }
            "split" => string.split(args.eat()?).into_value(),
            _ => return missing(),
        },

        Value::Content(content) => match method {
            "func" => content.func().into_value(),
            "has" => content.has(&args.expect::<EcoString>("field")?).into_value(),
            "at" => content
                .at(&args.expect::<EcoString>("field")?, args.named("default")?)
                .at(span)?,
            "fields" => content.dict().into_value(),
            "location" => content
                .location()
                .ok_or("this method can only be called on content returned by query(..)")
                .at(span)?
                .into_value(),
            _ => return missing(),
        },

        Value::Array(array) => match method {
            "len" => array.len().into_value(),
            "first" => array.first().at(span)?.clone(),
            "last" => array.last().at(span)?.clone(),
            "at" => array
                .at(args.expect("index")?, args.named("default")?.as_ref())
                .at(span)?
                .clone(),
            "slice" => {
                let start = args.expect("start")?;
                let mut end = args.eat()?;
                if end.is_none() {
                    end = args.named("count")?.map(|c: i64| start + c);
                }
                array.slice(start, end).at(span)?.into_value()
            }
            "contains" => array.contains(&args.expect("value")?).into_value(),
            "find" => array.find(vm, args.expect("function")?)?.into_value(),
            "position" => array.position(vm, args.expect("function")?)?.into_value(),
            "filter" => array.filter(vm, args.expect("function")?)?.into_value(),
            "map" => array.map(vm, args.expect("function")?)?.into_value(),
            "fold" => {
                array.fold(vm, args.expect("initial value")?, args.expect("function")?)?
            }
            "sum" => array.sum(args.named("default")?, span)?,
            "product" => array.product(args.named("default")?, span)?,
            "any" => array.any(vm, args.expect("function")?)?.into_value(),
            "all" => array.all(vm, args.expect("function")?)?.into_value(),
            "flatten" => array.flatten().into_value(),
            "rev" => array.rev().into_value(),
            "split" => array.split(args.expect("separator")?).into_value(),
            "join" => {
                let sep = args.eat()?;
                let last = args.named("last")?;
                array.join(sep, last).at(span)?
            }
            "sorted" => array.sorted(vm, span, args.named("key")?)?.into_value(),
            "zip" => array.zip(args.expect("other")?).into_value(),
            "enumerate" => array.enumerate().into_value(),
            _ => return missing(),
        },

        Value::Dict(dict) => match method {
            "len" => dict.len().into_value(),
            "at" => dict
                .at(&args.expect::<Str>("key")?, args.named("default")?.as_ref())
                .at(span)?
                .clone(),
            "keys" => dict.keys().into_value(),
            "values" => dict.values().into_value(),
            "pairs" => dict.pairs().into_value(),
            _ => return missing(),
        },

        Value::Func(func) => match method {
            "with" => func.with(args.take()).into_value(),
            "where" => {
                let fields = args.to_named();
                args.items.retain(|arg| arg.name.is_none());
                func.element()
                    .ok_or("`where()` can only be called on element functions")
                    .at(span)?
                    .where_(fields)
                    .into_value()
            }
            _ => return missing(),
        },

        Value::Args(args) => match method {
            "pos" => args.to_pos().into_value(),
            "named" => args.to_named().into_value(),
            _ => return missing(),
        },

        Value::Dyn(dynamic) => {
            if let Some(location) = dynamic.downcast::<Location>() {
                match method {
                    "page" => vm.vt.introspector.page(*location).into_value(),
                    "position" => vm.vt.introspector.position(*location).into_value(),
                    "page-numbering" => vm.vt.introspector.page_numbering(*location),
                    _ => return missing(),
                }
            } else if let Some(selector) = dynamic.downcast::<Selector>() {
                match method {
                    "or" => selector.clone().or(args.all::<Selector>()?).into_value(),
                    "and" => selector.clone().and(args.all::<Selector>()?).into_value(),
                    "before" => {
                        let location = args.expect::<Selector>("selector")?;
                        let inclusive =
                            args.named_or_find::<bool>("inclusive")?.unwrap_or(true);
                        selector.clone().before(location, inclusive).into_value()
                    }
                    "after" => {
                        let location = args.expect::<Selector>("selector")?;
                        let inclusive =
                            args.named_or_find::<bool>("inclusive")?.unwrap_or(true);
                        selector.clone().after(location, inclusive).into_value()
                    }
                    _ => return missing(),
                }
            } else if let Some(&datetime) = dynamic.downcast::<Datetime>() {
                match method {
                    "display" => {
                        datetime.display(args.eat()?).at(args.span)?.into_value()
                    }
                    "year" => datetime.year().into_value(),
                    "month" => datetime.month().into_value(),
                    "weekday" => datetime.weekday().into_value(),
                    "day" => datetime.day().into_value(),
                    "hour" => datetime.hour().into_value(),
                    "minute" => datetime.minute().into_value(),
                    "second" => datetime.second().into_value(),
                    _ => return missing(),
                }
            } else {
                return (vm.items.library_method)(vm, &dynamic, method, args, span);
            }
        }

        _ => return missing(),
    };

    args.finish()?;
    Ok(output)
}

/// Call a mutating method on a value.
pub fn call_mut(
    value: &mut Value,
    method: &str,
    mut args: Args,
    span: Span,
) -> SourceResult<Value> {
    let name = value.type_name();
    let missing = || Err(missing_method(name, method)).at(span);
    let mut output = Value::None;

    match value {
        Value::Array(array) => match method {
            "push" => array.push(args.expect("value")?),
            "pop" => output = array.pop().at(span)?,
            "insert" => {
                array.insert(args.expect("index")?, args.expect("value")?).at(span)?
            }
            "remove" => output = array.remove(args.expect("index")?).at(span)?,
            _ => return missing(),
        },

        Value::Dict(dict) => match method {
            "insert" => dict.insert(args.expect::<Str>("key")?, args.expect("value")?),
            "remove" => {
                output = dict.remove(&args.expect::<EcoString>("key")?).at(span)?
            }
            _ => return missing(),
        },

        _ => return missing(),
    }

    args.finish()?;
    Ok(output)
}

/// Call an accessor method on a value.
pub fn call_access<'a>(
    value: &'a mut Value,
    method: &str,
    mut args: Args,
    span: Span,
) -> SourceResult<&'a mut Value> {
    let name = value.type_name();
    let missing = || Err(missing_method(name, method)).at(span);

    let slot = match value {
        Value::Array(array) => match method {
            "first" => array.first_mut().at(span)?,
            "last" => array.last_mut().at(span)?,
            "at" => array.at_mut(args.expect("index")?).at(span)?,
            _ => return missing(),
        },
        Value::Dict(dict) => match method {
            "at" => dict.at_mut(&args.expect::<Str>("key")?).at(span)?,
            _ => return missing(),
        },
        _ => return missing(),
    };

    args.finish()?;
    Ok(slot)
}

/// Whether a specific method is mutating.
pub fn is_mutating(method: &str) -> bool {
    matches!(method, "push" | "pop" | "insert" | "remove")
}

/// Whether a specific method is an accessor.
pub fn is_accessor(method: &str) -> bool {
    matches!(method, "first" | "last" | "at")
}

/// The missing method error message.
#[cold]
fn missing_method(type_name: &str, method: &str) -> String {
    format!("type {type_name} has no method `{method}`")
}

/// List the available methods for a type and whether they take arguments.
pub fn methods_on(type_name: &str) -> &[(&'static str, bool)] {
    match type_name {
        "color" => &[("lighten", true), ("darken", true), ("negate", false)],
        "string" => &[
            ("len", false),
            ("at", true),
            ("clusters", false),
            ("codepoints", false),
            ("contains", true),
            ("ends-with", true),
            ("find", true),
            ("first", false),
            ("last", false),
            ("match", true),
            ("matches", true),
            ("position", true),
            ("replace", true),
            ("slice", true),
            ("split", true),
            ("starts-with", true),
            ("trim", true),
        ],
        "content" => &[
            ("func", false),
            ("has", true),
            ("at", true),
            ("fields", false),
            ("location", false),
        ],
        "array" => &[
            ("all", true),
            ("any", true),
            ("at", true),
            ("contains", true),
            ("filter", true),
            ("find", true),
            ("first", false),
            ("flatten", false),
            ("fold", true),
            ("insert", true),
            ("split", true),
            ("join", true),
            ("last", false),
            ("len", false),
            ("map", true),
            ("pop", false),
            ("position", true),
            ("push", true),
            ("remove", true),
            ("rev", false),
            ("slice", true),
            ("sorted", false),
            ("enumerate", false),
            ("zip", true),
        ],
        "dictionary" => &[
            ("at", true),
            ("insert", true),
            ("keys", false),
            ("len", false),
            ("pairs", false),
            ("remove", true),
            ("values", false),
        ],
        "function" => &[("where", true), ("with", true)],
        "arguments" => &[("named", false), ("pos", false)],
        "location" => &[("page", false), ("position", false), ("page-numbering", false)],
        "selector" => &[("or", true), ("and", true), ("before", true), ("after", true)],
        "counter" => &[
            ("display", true),
            ("at", true),
            ("final", true),
            ("step", true),
            ("update", true),
        ],
        "state" => &[("display", true), ("at", true), ("final", true), ("update", true)],
        _ => &[],
    }
}
