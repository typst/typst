use typst_syntax::Spanned;

use crate::diag::{LoadedWithin, SourceResult, bail};
use crate::engine::Engine;
use crate::foundations::{Array, Dict, IntoValue, Type, Value, cast, func};
use crate::loading::{DataSource, Load};
use crate::routines::CsvRecords;

/// Reads structured data from a CSV file.
///
/// The CSV file will be read and parsed into a 2-dimensional array of strings:
/// Each row in the CSV file will be represented as an array of strings, and all
/// rows will be collected into a single array. Header rows will not be
/// stripped.
///
/// = Example <example>
/// ```example
/// #let results = csv("example.csv")
///
/// #table(
///   columns: 2,
///   [*Condition*], [*Result*],
///   ..results.flatten(),
/// )
/// ```
#[func(title = "CSV")]
pub fn csv(
    engine: &mut Engine,
    /// A path to a CSV file or raw CSV bytes.
    source: Spanned<DataSource>,
    /// The delimiter that separates columns in the CSV file. Must be a single
    /// ASCII character.
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
    let loaded = source.load(engine.world)?;

    let mut builder = (engine.library.routines.new_csv_reader_builder)();
    let has_headers = row_type == RowType::Dict;
    builder.has_headers(has_headers);
    builder.delimiter(delimiter.0 as u8);

    let mut reader = builder.create_reader(loaded.data.as_slice());
    let mut headers: Option<Box<dyn CsvRecords>> = None;

    if has_headers {
        headers = Some(reader.header().within(&loaded)?);
    }

    let mut array = Array::new();
    for result in reader.records() {
        // Original solution was to use line from error, but that is
        // incorrect with `has_headers` set to `false`. See issue:
        // https://github.com/BurntSushi/rust-csv/issues/184
        let row = result.within(&loaded)?;
        let item = if let Some(headers) = &headers {
            let mut dict = Dict::new();
            for (field, value) in headers.iter().zip(row.iter()) {
                dict.insert(field.into(), value.into_value());
            }
            dict.into_value()
        } else {
            let sub = row.iter().map(|field| field.into_value()).collect();
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
