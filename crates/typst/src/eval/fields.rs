use ecow::{eco_format, EcoString};

use crate::diag::StrResult;
use crate::geom::{Axes, Color, GenAlign, PartialStroke, Stroke};

use super::{IntoValue, Value};

/// Try to access a field on a value.
/// This function is exclusively for types which have
/// predefined fields, such as color and length.
pub(crate) fn field(value: &Value, field: &str) -> StrResult<Value> {
    let name = value.type_name();
    let not_supported = || Err(no_fields(name));
    let missing = || Err(missing_field(name, field));

    // Special cases, such as module and dict, are handled by Value itself
    let result = match value {
        Value::Color(color) => match field {
            "values" => match color {
                Color::Luma(luma) => vec![luma.0].into_value(),
                Color::Rgba(rgba) => rgba.to_array().into_value(),
                Color::Cmyk(cmyk) => cmyk.to_array().into_value(),
            },
            _ => return missing(),
        },
        Value::Length(length) => match field {
            "em" => length.em.into_value(),
            "pt" => length.abs.into_value(),
            _ => return missing(),
        },
        Value::Relative(rel) => match field {
            "relative" => rel.rel.into_value(),
            "absolute" => rel.abs.into_value(),
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
                    "line_cap" => stroke
                        .line_cap
                        .unwrap_or_else(|| Stroke::default().line_cap)
                        .into_value(),
                    "line_join" => stroke
                        .line_join
                        .unwrap_or_else(|| Stroke::default().line_join)
                        .into_value(),
                    "dash_pattern" => {
                        stroke.dash_pattern.clone().unwrap_or(None).into_value()
                    }
                    "miter_limit" => stroke
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
        "color" => &["value"],
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
        "2d alignment" => &["x", "y"],
        _ => &[],
    }
}
