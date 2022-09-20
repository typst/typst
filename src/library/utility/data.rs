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
