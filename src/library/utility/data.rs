use std::fmt::Write;

use crate::library::prelude::*;

/// Read structured data from a CSV file.
pub fn csv(vm: &mut Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v: path, span } =
        args.expect::<Spanned<EcoString>>("path to csv file")?;

    let path = vm.locate(&path).at(span)?;
    let data = vm.world.file(&path).at(span)?;

    let mut builder = csv::ReaderBuilder::new();
    builder.has_headers(false);

    let mut reader = builder.from_reader(data.as_slice());
    let mut vec = vec![];

    for result in reader.records() {
        let row = result.map_err(format_csv_error).at(span)?;
        let array = row.iter().map(|field| Value::Str(field.into())).collect();
        vec.push(Value::Array(array))
    }

    Ok(Value::Array(Array::from_vec(vec)))
}

/// Format the user-facing CSV error message.
fn format_csv_error(error: csv::Error) -> String {
    match error.kind() {
        csv::ErrorKind::Utf8 { .. } => "file is not valid utf-8".into(),
        csv::ErrorKind::UnequalLengths { pos, expected_len, len } => {
            let mut msg = format!(
                "failed to parse csv file: found {len} instead of {expected_len} fields"
            );
            if let Some(pos) = pos {
                write!(msg, " in line {}", pos.line()).unwrap();
            }
            msg
        }
        _ => "failed to parse csv file".into(),
    }
}

/// Read structured data from a JSON file.
pub fn json(vm: &mut Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v: path, span } =
        args.expect::<Spanned<EcoString>>("path to json file")?;

    let path = vm.locate(&path).at(span)?;
    let data = vm.world.file(&path).at(span)?;
    let value: serde_json::Value =
        serde_json::from_slice(&data).map_err(format_json_error).at(span)?;

    Ok(convert_json(value))
}

/// Convert a JSON value to a Typst value.
fn convert_json(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::None,
        serde_json::Value::Bool(v) => Value::Bool(v),
        serde_json::Value::Number(v) => match v.as_i64() {
            Some(int) => Value::Int(int),
            None => Value::Float(v.as_f64().unwrap_or(f64::NAN)),
        },
        serde_json::Value::String(v) => Value::Str(v.into()),
        serde_json::Value::Array(v) => {
            Value::Array(v.into_iter().map(convert_json).collect())
        }
        serde_json::Value::Object(v) => Value::Dict(
            v.into_iter()
                .map(|(key, value)| (key.into(), convert_json(value)))
                .collect(),
        ),
    }
}

/// Format the user-facing JSON error message.
fn format_json_error(error: serde_json::Error) -> String {
    assert!(error.is_syntax() || error.is_eof());
    format!(
        "failed to parse json file: syntax error in line {}",
        error.line()
    )
}
