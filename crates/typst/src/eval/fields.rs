use ecow::{eco_format, EcoString};

use crate::diag::StrResult;
use crate::geom::{Axes, GenAlign, PartialStroke, Stroke};

use super::{IntoValue, Value};

/// Try to access a field on a value.
/// This function is exclusively for types which have
/// predefined fields, such as stroke and length.
pub(crate) fn field(value: &Value, field: &str) -> StrResult<Value> {
    let name = value.type_name();
    let not_supported = || Err(no_fields(name));
    let missing = || Err(missing_field(name, field));

    // Special cases, such as module and dict, are handled by Value itself
    let result = match value {
        Value::Length(length) => match field {
            "em" => length.em.into_value(),
            "abs" => length.abs.into_value(),
            _ => return missing(),
        },
        Value::Relative(rel) => match field {
            "ratio" => rel.rel.into_value(),
            "length" => rel.abs.into_value(),
            _ => return missing(),
        },
        Value::Dyn(dynamic) => {
            if let Some(stroke) = dynamic.downcast::<PartialStroke>() {
                match field {
                    "paint" => stroke
                        .paint
                        .clone()
                        .unwrap_or_else(|| Stroke::default().paint)
                        .into_value(),
                    "thickness" => stroke
                        .thickness
                        .unwrap_or_else(|| Stroke::default().thickness.into())
                        .into_value(),
                    "cap" => stroke
                        .line_cap
                        .unwrap_or_else(|| Stroke::default().line_cap)
                        .into_value(),
                    "join" => stroke
                        .line_join
                        .unwrap_or_else(|| Stroke::default().line_join)
                        .into_value(),
                    "dash" => stroke.dash_pattern.clone().unwrap_or(None).into_value(),
                    "miter-limit" => stroke
                        .miter_limit
                        .unwrap_or_else(|| Stroke::default().miter_limit)
                        .0
                        .into_value(),
                    _ => return missing(),
                }
            } else if let Some(align2d) = dynamic.downcast::<Axes<GenAlign>>() {
                match field {
                    "x" => align2d.x.into_value(),
                    "y" => align2d.y.into_value(),
                    _ => return missing(),
                }
            } else {
                return not_supported();
            }
        }
        _ => return not_supported(),
    };

    Ok(result)
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
        "length" => &["em", "abs"],
        "relative length" => &["ratio", "length"],
        "stroke" => &["paint", "thickness", "cap", "join", "dash", "miter-limit"],
        "2d alignment" => &["x", "y"],
        _ => &[],
    }
}
