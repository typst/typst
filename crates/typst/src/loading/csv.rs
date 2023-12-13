use ecow::{eco_format, EcoString};

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{cast, func, scope, Array, IntoValue, Value};
use crate::loading::Readable;
use crate::syntax::Spanned;
use crate::World;

/// Reads structured data from a CSV file.
///
/// The CSV file will be read and parsed into a 2-dimensional array of strings:
/// Each row in the CSV file will be represented as an array of strings, and all
/// rows will be collected into a single array. Header rows will not be
/// stripped.
///
/// # Example
/// ```example
/// #let results = csv("data.csv")
///
/// #table(
///   columns: 2,
///   [*Condition*], [*Result*],
///   ..results.flatten(),
/// )
/// ```
#[func(scope, title = "CSV")]
pub fn csv(
    /// The engine.
    engine: &mut Engine,
    /// Path to a CSV file.
    path: Spanned<EcoString>,
    /// The delimiter that separates columns in the CSV file.
    /// Must be a single ASCII character.
    #[named]
    #[default]
    delimiter: Delimiter,
) -> SourceResult<Array> {
    let Spanned { v: path, span } = path;
    let id = span.resolve_path(&path).at(span)?;
    let data = engine.world.file(id).at(span)?;
    self::csv::decode(Spanned::new(Readable::Bytes(data), span), delimiter)
}

#[scope]
impl csv {
    /// Reads structured data from a CSV string/bytes.
    #[func(title = "Decode CSV")]
    pub fn decode(
        /// CSV data.
        data: Spanned<Readable>,
        /// The delimiter that separates columns in the CSV file.
        /// Must be a single ASCII character.
        #[named]
        #[default]
        delimiter: Delimiter,
    ) -> SourceResult<Array> {
        let Spanned { v: data, span } = data;
        let mut builder = ::csv::ReaderBuilder::new();
        builder.has_headers(false);
        builder.delimiter(delimiter.0 as u8);
        let mut reader = builder.from_reader(data.as_slice());
        let mut array = Array::new();

        for (line, result) in reader.records().enumerate() {
            // Original solution use line from error, but that is incorrect with
            // `has_headers` set to `false`. See issue:
            // https://github.com/BurntSushi/rust-csv/issues/184
            let line = line + 1; // Counting lines from 1
            let row = result.map_err(|err| format_csv_error(err, line)).at(span)?;
            let sub = row.into_iter().map(|field| field.into_value()).collect();
            array.push(Value::Array(sub))
        }

        Ok(array)
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
    v: EcoString => {
        let mut chars = v.chars();
        let first = chars.next().ok_or("delimiter must not be empty")?;
        if chars.next().is_some() {
            bail!("delimiter must be a single character");
        }

        if !first.is_ascii() {
            bail!("delimiter must be an ASCII character");
        }

        Self(first)
    },
}

/// Format the user-facing CSV error message.
fn format_csv_error(err: ::csv::Error, line: usize) -> EcoString {
    match err.kind() {
        ::csv::ErrorKind::Utf8 { .. } => "file is not valid utf-8".into(),
        ::csv::ErrorKind::UnequalLengths { expected_len, len, .. } => {
            eco_format!(
                "failed to parse CSV (found {len} instead of \
                 {expected_len} fields in line {line})"
            )
        }
        _ => eco_format!("failed to parse CSV ({err})"),
    }
}
