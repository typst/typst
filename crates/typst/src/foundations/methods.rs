//! Handles special built-in methods on values.

use ecow::EcoString;

use crate::diag::{At, SourceResult};
use crate::foundations::{Args, Array, Dict, Str, Type, Value};
use crate::syntax::Span;
use crate::util::PicoStr;

/// List the available methods for a type and whether they take arguments.
pub fn mutable_methods_on(ty: Type) -> &'static [(&'static str, bool)] {
    if ty == Type::of::<Array>() {
        &[
            ("first", false),
            ("last", false),
            ("at", true),
            ("pop", false),
            ("push", true),
            ("insert", true),
            ("remove", true),
        ]
    } else if ty == Type::of::<Dict>() {
        &[("at", true), ("insert", true), ("remove", true)]
    } else {
        &[]
    }
}

/// Whether a specific method is mutating.
pub(crate) fn is_mutating_method(method: PicoStr) -> bool {
    let push = pico!("push");
    let pop = pico!("pop");
    let insert = pico!("insert");
    let remove = pico!("remove");

    method == push || method == pop || method == insert || method == remove
}

/// Whether a specific method is an accessor.
pub(crate) fn is_accessor_method(method: &str) -> bool {
    matches!(method, "first" | "last" | "at")
}

/// Call a mutating method on a value.
pub(crate) fn call_method_mut(
    value: &mut Value,
    method: impl Into<PicoStr>,
    mut args: Args,
    span: Span,
) -> SourceResult<Value> {
    let method = method.into();
    let ty = value.ty();
    let missing = || Err(missing_method(ty, method.resolve())).at(span);
    let mut output = Value::None;

    let push = pico!("push");
    let pop = pico!("pop");
    let insert = pico!("insert");
    let remove = pico!("remove");

    match value {
        Value::Array(array) => {
            if method == push {
                array.push(args.expect(pico!("value"))?)
            } else if method == pop {
                output = array.pop().at(span)?
            } else if method == insert {
                array
                    .insert(args.expect(pico!("index"))?, args.expect(pico!("value"))?)
                    .at(span)?
            } else if method == remove {
                output = array
                    .remove(args.expect(pico!("index"))?, args.named(pico!("default"))?)
                    .at(span)?
            } else {
                return missing();
            }
        }
        Value::Dict(dict) => {
            if method == insert {
                dict.insert(
                    args.expect::<Str>(pico!("key"))?,
                    args.expect(pico!("value"))?,
                )
            } else if method == remove {
                output = dict
                    .remove(args.expect(pico!("key"))?, args.named(pico!("default"))?)
                    .at(span)?
            } else {
                return missing();
            }
        }

        _ => return missing(),
    }

    args.finish()?;
    Ok(output)
}

/// Call an accessor method on a value.
pub(crate) fn call_method_access<'a>(
    value: &'a mut Value,
    method: impl Into<PicoStr>,
    mut args: Args,
    span: Span,
) -> SourceResult<&'a mut Value> {
    let ty = value.ty();
    let method = method.into();
    let missing = || Err(missing_method(ty, method.resolve())).at(span);

    let first = pico!("first");
    let last = pico!("last");
    let at = pico!("at");

    let slot = match value {
        Value::Array(array) => {
            if method == first {
                array.first_mut().at(span)?
            } else if method == last {
                array.last_mut().at(span)?
            } else if method == at {
                array.at_mut(args.expect(pico!("index"))?).at(span)?
            } else {
                return missing();
            }
        }
        Value::Dict(dict) => {
            if method == at {
                let key: EcoString = args.expect(pico!("key"))?;
                dict.at_mut(&key).at(span)?
            } else {
                return missing();
            }
        }
        _ => return missing(),
    };

    args.finish()?;
    Ok(slot)
}

/// The missing method error message.
#[cold]
fn missing_method(ty: Type, method: &str) -> String {
    format!("type {ty} has no method `{method}`")
}
