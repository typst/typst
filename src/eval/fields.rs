use ecow::{eco_format, EcoString};

use crate::diag::StrResult;
use crate::geom::{Color, PartialStroke, Stroke};

use super::Value;

/// Try to access a field on a value.
pub(crate) fn field(value: &Value, field: &str) -> StrResult<Value> {
    let name = value.type_name();
    let not_supported = || Err(no_fields(name));
    let missing = || Err(missing_field(name, field));

    match value {
        Value::Symbol(symbol) => symbol.clone().modified(field).map(Value::Symbol),
        Value::Dict(dict) => dict.at(field, None).cloned(),
        Value::Content(content) => content.at(field, None),
        Value::Module(module) => module.get(field).cloned(),
        Value::Func(func) => func.get(field).cloned(),
        Value::Color(color) => match field {
            "kind" => Ok(color.kind().into()),
            "values" => Ok(match color {
                Color::Luma(luma) => vec![luma.0].into(),
                Color::Rgba(rgba) => rgba.to_array().into(),
                Color::Cmyk(cmyk) => cmyk.to_array().into(),
            }),
            _ => missing(),
        },
        Value::Length(length) => match field {
            "em" => Ok(length.em.into()),
            "pt" => Ok(length.abs.into()),
            _ => missing(),
        },
        Value::Relative(rel) => match field {
            "relative" => Ok(rel.rel.into()),
            "absolute" => Ok(rel.abs.into()),
            _ => missing(),
        },
        Value::Dyn(dynamic) => {
            if let Some(stroke) = dynamic.downcast::<PartialStroke>() {
                match field {
                    "paint" => Ok(stroke
                        .paint
                        .clone()
                        .unwrap_or_else(|| Stroke::default().paint)
                        .into()),
                    "thickness" => Ok(stroke
                        .thickness
                        .unwrap_or_else(|| Stroke::default().thickness.into())
                        .into()),
                    "line_cap" => Ok(stroke
                        .line_cap
                        .clone()
                        .unwrap_or_else(|| Stroke::default().line_cap)
                        .into()),
                    "line_join" => Ok(stroke
                        .line_join
                        .clone()
                        .unwrap_or_else(|| Stroke::default().line_join)
                        .into()),
                    "dash_pattern" => {
                        Ok(stroke.dash_pattern.clone().unwrap_or(None).into())
                    }
                    "miter_limit" => Ok(stroke
                        .miter_limit
                        .unwrap_or_else(|| Stroke::default().miter_limit)
                        .0
                        .into()),
                    _ => missing(),
                }
            } else {
                not_supported()
            }
        }
        _ => not_supported(),
    }
}

/// The error message for a type not supporting field access.
#[cold]
fn no_fields(type_name: &str) -> EcoString {
    eco_format!("cannot access fields on type {type_name}")
}

/// The missing field error message.
#[cold]
fn missing_field(type_name: &str, field: &str) -> EcoString {
    eco_format!("{type_name} does not contain field \"{field}\"")
}

/// List the available fields for a type.
pub fn fields_on(type_name: &str) -> &[&'static str] {
    match type_name {
        "color" => &["kind", "value"],
        "length" => &["em", "pt"],
        "relative length" => &["relative", "absolute"],
        "stroke" => &[
            "paint",
            "thickness",
            "line_cap",
            "line_join",
            "dash_pattern",
            "miter_limit",
        ],
        _ => &[],
    }
}
