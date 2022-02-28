//! Computational utility functions.

mod math;
mod numbering;

pub use math::*;
pub use numbering::*;

use std::str::FromStr;

use crate::library::prelude::*;
use crate::library::text::{Case, TextNode};

/// Ensure that a condition is fulfilled.
pub fn assert(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<bool>>("condition")?;
    if !v {
        bail!(span, "assertion failed");
    }
    Ok(Value::None)
}

/// The name of a value's type.
pub fn type_(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    Ok(args.expect::<Value>("value")?.type_name().into())
}

/// The string representation of a value.
pub fn repr(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    Ok(args.expect::<Value>("value")?.repr().into())
}

/// Join a sequence of values, optionally interspersing it with another value.
pub fn join(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    let span = args.span;
    let sep = args.named::<Value>("sep")?.unwrap_or(Value::None);

    let mut result = Value::None;
    let mut iter = args.all::<Value>()?.into_iter();

    if let Some(first) = iter.next() {
        result = first;
    }

    for value in iter {
        result = result.join(sep.clone()).at(span)?;
        result = result.join(value).at(span)?;
    }

    Ok(result)
}

/// Convert a value to a integer.
pub fn int(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("value")?;
    Ok(Value::Int(match v {
        Value::Bool(v) => v as i64,
        Value::Int(v) => v,
        Value::Float(v) => v as i64,
        Value::Str(v) => match v.parse() {
            Ok(v) => v,
            Err(_) => bail!(span, "invalid integer"),
        },
        v => bail!(span, "cannot convert {} to integer", v.type_name()),
    }))
}

/// Convert a value to a float.
pub fn float(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("value")?;
    Ok(Value::Float(match v {
        Value::Int(v) => v as f64,
        Value::Float(v) => v,
        Value::Str(v) => match v.parse() {
            Ok(v) => v,
            Err(_) => bail!(span, "invalid float"),
        },
        v => bail!(span, "cannot convert {} to float", v.type_name()),
    }))
}

/// Cconvert a value to a string.
pub fn str(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("value")?;
    Ok(Value::Str(match v {
        Value::Int(v) => format_eco!("{}", v),
        Value::Float(v) => format_eco!("{}", v),
        Value::Str(v) => v,
        v => bail!(span, "cannot convert {} to string", v.type_name()),
    }))
}

/// Create an RGB(A) color.
pub fn rgb(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    Ok(Value::from(
        if let Some(string) = args.find::<Spanned<EcoString>>()? {
            match RgbaColor::from_str(&string.v) {
                Ok(color) => color,
                Err(_) => bail!(string.span, "invalid hex string"),
            }
        } else {
            struct Component(u8);

            castable! {
                Component,
                Expected: "integer or relative",
                Value::Int(v) => match v {
                    0 ..= 255 => Self(v as u8),
                    _ => Err("must be between 0 and 255")?,
                },
                Value::Relative(v) => if (0.0 ..= 1.0).contains(&v.get()) {
                    Self((v.get() * 255.0).round() as u8)
                } else {
                    Err("must be between 0% and 100%")?
                },
            }

            let Component(r) = args.expect("red component")?;
            let Component(g) = args.expect("green component")?;
            let Component(b) = args.expect("blue component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(255));
            RgbaColor::new(r, g, b, a)
        },
    ))
}

/// Create a CMYK color.
pub fn cmyk(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    struct Component(u8);

    castable! {
        Component,
        Expected: "relative",
        Value::Relative(v) => if (0.0 ..= 1.0).contains(&v.get()) {
            Self((v.get() * 255.0).round() as u8)
        } else {
            Err("must be between 0% and 100%")?
        },
    }

    let Component(c) = args.expect("cyan component")?;
    let Component(m) = args.expect("magenta component")?;
    let Component(y) = args.expect("yellow component")?;
    let Component(k) = args.expect("key component")?;
    Ok(Value::Color(CmykColor::new(c, m, y, k).into()))
}

/// The length of a string, an array or a dictionary.
pub fn len(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("collection")?;
    Ok(Value::Int(match v {
        Value::Str(v) => v.len() as i64,
        Value::Array(v) => v.len(),
        Value::Dict(v) => v.len(),
        v => bail!(
            span,
            "expected string, array or dictionary, found {}",
            v.type_name(),
        ),
    }))
}

/// Convert a string to lowercase.
pub fn lower(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    case(Case::Lower, args)
}

/// Convert a string to uppercase.
pub fn upper(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    case(Case::Upper, args)
}

/// Change the case of a string or template.
fn case(case: Case, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("string or template")?;
    Ok(match v {
        Value::Str(v) => Value::Str(case.apply(&v).into()),
        Value::Template(v) => Value::Template(v.styled(TextNode::CASE, Some(case))),
        v => bail!(span, "expected string or template, found {}", v.type_name()),
    })
}

/// The sorted version of an array.
pub fn sorted(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<Array>>("array")?;
    Ok(Value::Array(v.sorted().at(span)?))
}
