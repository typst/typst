use ecow::EcoVec;
use typst_syntax::Spanned;

use crate::diag::{bail, SourceDiagnostic, SourceResult};
use crate::engine::Engine;
use crate::foundations::{cast, func, scope, Array, Dict, IntoValue, Type, Value};
use crate::loading::{Loaded, DataSource, LineCol, Load, Readable, ReportPos};

/// Reads structured data from a CSV file.
///
/// The CSV file will be read and parsed into a 2-dimensional array of strings:
/// Each row in the CSV file will be represented as an array of strings, and all
/// rows will be collected into a single array. Header rows will not be
/// stripped.
///
/// # Example
/// ```example
/// #let results = csv("example.csv")
///
/// #table(
///   columns: 2,
///   [*Condition*], [*Result*],
///   ..results.flatten(),
/// )
/// ```
#[func(scope, title = "CSV")]
pub fn csv(
    engine: &mut Engine,
    /// A [path]($syntax/#paths) to a CSV file or raw CSV bytes.
    source: Spanned<DataSource>,
    /// The delimiter that separates columns in the CSV file.
    /// Must be a single ASCII character.
    #[named]
    #[default]
    delimiter: Delimiter,
    /// How to represent the file's rows.
    ///
    /// - If set to `array`, each row is represented as a plain array of
    ///   strings.
    /// - If set to `dictionary`, each row is represented as a dictionary
    ///   mapping from header keys to strings. This option only makes sense when
    ///   a header row is present in the CSV file.
    #[named]
    #[default(RowType::Array)]
    row_type: RowType,
) -> SourceResult<Array> {
    let data = source.load(engine.world)?;

    let mut builder = ::csv::ReaderBuilder::new();
    let has_headers = row_type == RowType::Dict;
    builder.has_headers(has_headers);
    builder.delimiter(delimiter.0 as u8);

    // Counting lines from 1 by default.
    let mut line_offset: usize = 1;
    let mut reader = builder.from_reader(data.bytes.as_slice());
    let mut headers: Option<::csv::StringRecord> = None;

    if has_headers {
        // Counting lines from 2 because we have a header.
        line_offset += 1;
        headers = Some(
            reader
                .headers()
                .cloned()
                .map_err(|err| format_csv_error(&data, err, 1))?,
        );
    }

    let mut array = Array::new();
    for (line, result) in reader.records().enumerate() {
        // Original solution was to use line from error, but that is
        // incorrect with `has_headers` set to `false`. See issue:
        // https://github.com/BurntSushi/rust-csv/issues/184
        let line = line + line_offset;
        let row = result.map_err(|err| format_csv_error(&data, err, line))?;
        let item = if let Some(headers) = &headers {
            let mut dict = Dict::new();
            for (field, value) in headers.iter().zip(&row) {
                dict.insert(field.into(), value.into_value());
            }
            dict.into_value()
        } else {
            let sub = row.into_iter().map(|field| field.into_value()).collect();
            Value::Array(sub)
        };
        array.push(item);
    }

    Ok(array)
}

#[scope]
impl csv {
    /// Reads structured data from a CSV string/bytes.
    #[func(title = "Decode CSV")]
    #[deprecated = "`csv.decode` is deprecated, directly pass bytes to `csv` instead"]
    pub fn decode(
        engine: &mut Engine,
        /// CSV data.
        data: Spanned<Readable>,
        /// The delimiter that separates columns in the CSV file.
        /// Must be a single ASCII character.
        #[named]
        #[default]
        delimiter: Delimiter,
        /// How to represent the file's rows.
        ///
        /// - If set to `array`, each row is represented as a plain array of
        ///   strings.
        /// - If set to `dictionary`, each row is represented as a dictionary
        ///   mapping from header keys to strings. This option only makes sense
        ///   when a header row is present in the CSV file.
        #[named]
        #[default(RowType::Array)]
        row_type: RowType,
    ) -> SourceResult<Array> {
        csv(engine, data.map(Readable::into_source), delimiter, row_type)
    }
}

/// The delimiter to use when parsing CSV files.
pub struct Delimiter(char);

impl Default for Delimiter {
    fn default() -> Self {
        Self(',')
    }
}

cast! {
    Delimiter,
    self => self.0.into_value(),
    c: char => if c.is_ascii() {
        Self(c)
    } else {
        bail!("delimiter must be an ASCII character")
    },
}

/// The type of parsed rows.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum RowType {
    Array,
    Dict,
}

cast! {
    RowType,
    self => match self {
        Self::Array => Type::of::<Array>(),
        Self::Dict => Type::of::<Dict>(),
    }.into_value(),
    ty: Type => {
        if ty == Type::of::<Array>() {
            Self::Array
        } else if ty == Type::of::<Dict>() {
            Self::Dict
        } else {
            bail!("expected `array` or `dictionary`");
        }
    },
}

/// Format the user-facing CSV error message.
fn format_csv_error(
    data: &Loaded,
    err: ::csv::Error,
    line: usize,
) -> EcoVec<SourceDiagnostic> {
    let msg = "failed to parse CSV";
    let pos = (err.kind().position())
        .map(|pos| {
            let start = pos.byte() as usize;
            ReportPos::Range(start..start)
        })
        .unwrap_or(LineCol::one_based(line, 1).into());
    match err.kind() {
        ::csv::ErrorKind::Utf8 { .. } => data.err_at(pos, msg, "file is not valid utf-8"),
        ::csv::ErrorKind::UnequalLengths { expected_len, len, .. } => {
            let err =
                format!("found {len} instead of {expected_len} fields in line {line}");
            data.err_at(pos, msg, err)
        }
        _ => data.err_at(pos, "failed to parse CSV", err),
    }
}
