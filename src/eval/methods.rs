//! Methods on values.

use super::{Args, Machine, Regex, StrExt, Value};
use crate::diag::{At, TypResult};
use crate::model::{Content, Group};
use crate::syntax::Span;
use crate::util::EcoString;

/// Call a method on a value.
pub fn call(
    vm: &mut Machine,
    value: Value,
    method: &str,
    mut args: Args,
    span: Span,
) -> TypResult<Value> {
    let name = value.type_name();
    let missing = || Err(missing_method(name, method)).at(span);

    let output = match value {
        Value::Str(string) => match method {
            "len" => Value::Int(string.len() as i64),
            "trim" => Value::Str(string.trim().into()),
            "split" => Value::Array(string.split(args.eat()?)),
            _ => missing()?,
        },

        Value::Array(array) => match method {
            "len" => Value::Int(array.len()),
            "slice" => {
                let start = args.expect("start")?;
                let mut end = args.eat()?;
                if end.is_none() {
                    end = args.named("count")?.map(|c: i64| start + c);
                }
                Value::Array(array.slice(start, end).at(span)?)
            }
            "map" => Value::Array(array.map(vm, args.expect("function")?)?),
            "filter" => Value::Array(array.filter(vm, args.expect("function")?)?),
            "flatten" => Value::Array(array.flatten()),
            "find" => array.find(args.expect("value")?).map_or(Value::None, Value::Int),
            "join" => {
                let sep = args.eat()?;
                let last = args.named("last")?;
                array.join(sep, last).at(span)?
            }
            "sorted" => Value::Array(array.sorted().at(span)?),
            _ => missing()?,
        },

        Value::Dict(dict) => match method {
            "len" => Value::Int(dict.len()),
            "keys" => Value::Array(dict.keys()),
            "values" => Value::Array(dict.values()),
            "pairs" => Value::Array(dict.map(vm, args.expect("function")?)?),
            _ => missing()?,
        },

        Value::Func(func) => match method {
            "with" => Value::Func(func.clone().with(args.take())),
            _ => missing()?,
        },

        Value::Args(args) => match method {
            "positional" => Value::Array(args.to_positional()),
            "named" => Value::Dict(args.to_named()),
            _ => missing()?,
        },

        Value::Dyn(dynamic) => match method {
            "matches" => {
                if let Some(regex) = dynamic.downcast::<Regex>() {
                    Value::Bool(regex.matches(&args.expect::<EcoString>("text")?))
                } else {
                    missing()?
                }
            }
            "entry" => {
                if let Some(group) = dynamic.downcast::<Group>() {
                    Value::Content(Content::Locate(group.entry(args.expect("recipe")?)))
                } else {
                    missing()?
                }
            }
            _ => missing()?,
        },

        _ => missing()?,
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
) -> TypResult<()> {
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
            _ => missing()?,
        },

        Value::Dict(dict) => match method {
            "remove" => dict.remove(&args.expect("key")?).at(span)?,
            _ => missing()?,
        },

        _ => missing()?,
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
