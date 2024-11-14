use ecow::{eco_vec, EcoString, EcoVec};
use typst_syntax::{FileId, Span, Spanned};

use crate::diag::{bail, error, At, SourceDiagnostic, SourceResult};
use crate::engine::Engine;
use crate::foundations::{cast, func, scope, Array, Dict, IntoValue, Type, Value};
use crate::loading::Readable;
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
    /// The engine.
    engine: &mut Engine,
    /// Path to a CSV file.
    ///
    /// For more details, see the [Paths section]($syntax/#paths).
    path: Spanned<EcoString>,
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
    let Spanned { v: path, span } = path;
    let id = span.resolve_path(&path).at(span)?;
    let data = engine.world.file(id).at(span)?;
    decode_inner(Spanned::new(Readable::Bytes(data), span), delimiter, row_type, Some(id))
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
        decode_inner(data, delimiter, row_type, None)
    }
}

fn decode_inner(
    data: Spanned<Readable>,
    delimiter: Delimiter,
    row_type: RowType,
    source: Option<FileId>,
) -> SourceResult<Array> {
    let Spanned { v: data, span } = data;
    let has_headers = row_type == RowType::Dict;

    let mut builder = ::csv::ReaderBuilder::new();
    builder.has_headers(has_headers);
    builder.delimiter(delimiter.0 as u8);

    // Counting lines from 1 by default.
    let mut line_offset: usize = 1;
    let mut reader = builder.from_reader(data.as_slice());
    let mut headers: Option<::csv::StringRecord> = None;

    if has_headers {
        // Counting lines from 2 because we have a header.
        line_offset += 1;
        headers = Some(
            reader
                .headers()
                .map_err(|err| format_csv_error(err, 1, span, source))?
                .clone(),
        );
    }

    let mut array = Array::new();
    for (line, result) in reader.records().enumerate() {
        // Original solution was to use line from error, but that is
        // incorrect with `has_headers` set to `false`. See issue:
        // https://github.com/BurntSushi/rust-csv/issues/184
        let line = line + line_offset;
        let row = result.map_err(|err| format_csv_error(err, line, span, source))?;
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
    err: ::csv::Error,
    line: usize,
    span: Span,
    file_id: Option<FileId>,
) -> EcoVec<SourceDiagnostic> {
    let span = file_id
        .and_then(|id| err.position().map(|pos| (id, pos.byte() as usize)))
        .map_or(span, |(id, pos)| Span::from_range(id, pos..pos));

    eco_vec![match err.kind() {
        ::csv::ErrorKind::Utf8 { .. } => error!(span, "file is not valid utf-8"),
        ::csv::ErrorKind::UnequalLengths { expected_len, len, .. } => {
            error!(
                span,
                "failed to parse CSV (found {len} instead of \
                 {expected_len} fields in line {line})"
            )
        }
        _ => error!(span, "failed to parse CSV ({err})"),
    }]
}
