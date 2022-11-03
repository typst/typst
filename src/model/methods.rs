//! Methods on values.

use super::{Args, Value, Vm};
use crate::diag::{At, SourceResult};
use crate::syntax::Span;
use crate::util::EcoString;

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
        Value::Str(string) => match method {
            "len" => Value::Int(string.len() as i64),
            "slice" => {
                let start = args.expect("start")?;
                let mut end = args.eat()?;
                if end.is_none() {
                    end = args.named("count")?.map(|c: i64| start + c);
                }
                Value::Str(string.slice(start, end).at(span)?)
            }
            "contains" => Value::Bool(string.contains(args.expect("pattern")?)),
            "starts-with" => Value::Bool(string.starts_with(args.expect("pattern")?)),
            "ends-with" => Value::Bool(string.ends_with(args.expect("pattern")?)),
            "find" => {
                string.find(args.expect("pattern")?).map_or(Value::None, Value::Str)
            }
            "position" => string
                .position(args.expect("pattern")?)
                .map_or(Value::None, Value::Int),
            "match" => string
                .match_(args.expect("pattern")?)
                .map_or(Value::None, Value::Dict),
            "matches" => Value::Array(string.matches(args.expect("pattern")?)),
            "replace" => {
                let pattern = args.expect("pattern")?;
                let with = args.expect("replacement string")?;
                let count = args.named("count")?;
                Value::Str(string.replace(pattern, with, count))
            }
            "trim" => {
                let pattern = args.eat()?;
                let at = args.named("at")?;
                let repeat = args.named("repeat")?.unwrap_or(true);
                Value::Str(string.trim(pattern, at, repeat))
            }
            "split" => Value::Array(string.split(args.eat()?)),
            _ => return missing(),
        },

        Value::Array(array) => match method {
            "len" => Value::Int(array.len()),
            "first" => array.first().cloned().unwrap_or(Value::None),
            "last" => array.last().cloned().unwrap_or(Value::None),
            "slice" => {
                let start = args.expect("start")?;
                let mut end = args.eat()?;
                if end.is_none() {
                    end = args.named("count")?.map(|c: i64| start + c);
                }
                Value::Array(array.slice(start, end).at(span)?)
            }
            "contains" => Value::Bool(array.contains(&args.expect("value")?)),
            "find" => array.find(vm, args.expect("function")?)?.unwrap_or(Value::None),
            "position" => array
                .position(vm, args.expect("function")?)?
                .map_or(Value::None, Value::Int),
            "filter" => Value::Array(array.filter(vm, args.expect("function")?)?),
            "map" => Value::Array(array.map(vm, args.expect("function")?)?),
            "any" => Value::Bool(array.any(vm, args.expect("function")?)?),
            "all" => Value::Bool(array.all(vm, args.expect("function")?)?),
            "flatten" => Value::Array(array.flatten()),
            "rev" => Value::Array(array.rev()),
            "join" => {
                let sep = args.eat()?;
                let last = args.named("last")?;
                array.join(sep, last).at(span)?
            }
            "sorted" => Value::Array(array.sorted().at(span)?),
            _ => return missing(),
        },

        Value::Dict(dict) => match method {
            "len" => Value::Int(dict.len()),
            "keys" => Value::Array(dict.keys()),
            "values" => Value::Array(dict.values()),
            "pairs" => Value::Array(dict.map(vm, args.expect("function")?)?),
            _ => return missing(),
        },

        Value::Func(func) => match method {
            "with" => Value::Func(func.with(args.take())),
            _ => return missing(),
        },

        Value::Args(args) => match method {
            "positional" => Value::Array(args.to_positional()),
            "named" => Value::Dict(args.to_named()),
            _ => return missing(),
        },

        Value::Color(color) => match method {
            "lighten" => Value::Color(color.lighten(args.expect("amount")?)),
            "darken" => Value::Color(color.darken(args.expect("amount")?)),
            "negate" => Value::Color(color.negate()),
            _ => return missing(),
        },

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
) -> SourceResult<()> {
    let name = value.type_name();
    let missing = || Err(missing_method(name, method)).at(span);

    match value {
        Value::Array(array) => match method {
            "push" => array.push(args.expect("value")?),
            "pop" => array.pop().at(span)?,
            "insert" => {
                array.insert(args.expect("index")?, args.expect("value")?).at(span)?
            }
            "remove" => array.remove(args.expect("index")?).at(span)?,
            _ => return missing(),
        },

        Value::Dict(dict) => match method {
            "remove" => dict.remove(&args.expect::<EcoString>("key")?).at(span)?,
            _ => return missing(),
        },

        _ => return missing(),
    }

    args.finish()?;
    Ok(())
}

/// Whether a specific method is mutating.
pub fn is_mutating(method: &str) -> bool {
    matches!(method, "push" | "pop" | "insert" | "remove")
}

/// The missing method error message.
#[cold]
fn missing_method(type_name: &str, method: &str) -> String {
    format!("type {type_name} has no method `{method}`")
}
