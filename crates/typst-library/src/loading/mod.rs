//! Data loading.

#[path = "cbor.rs"]
mod cbor_;
#[path = "csv.rs"]
mod csv_;
#[path = "json.rs"]
mod json_;
#[path = "read.rs"]
mod read_;
#[path = "toml.rs"]
mod toml_;
#[path = "xml.rs"]
mod xml_;
#[path = "yaml.rs"]
mod yaml_;

use comemo::Tracked;
use ecow::{eco_vec, EcoString, EcoVec};
use typst_syntax::{FileId, Span, Spanned};
use utf8_iter::ErrorReportingUtf8Chars;

pub use self::cbor_::*;
pub use self::csv_::*;
pub use self::json_::*;
pub use self::read_::*;
pub use self::toml_::*;
pub use self::xml_::*;
pub use self::yaml_::*;

use crate::diag::{error, At, FileError, SourceDiagnostic, SourceResult};
use crate::foundations::OneOrMultiple;
use crate::foundations::{cast, Bytes, Scope, Str};
use crate::World;

/// Hook up all `data-loading` definitions.
pub(super) fn define(global: &mut Scope) {
    global.start_category(crate::Category::DataLoading);
    global.define_func::<read>();
    global.define_func::<csv>();
    global.define_func::<json>();
    global.define_func::<toml>();
    global.define_func::<yaml>();
    global.define_func::<cbor>();
    global.define_func::<xml>();
    global.reset_category();
}

/// Something we can retrieve byte data from.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum DataSource {
    /// A path to a file.
    Path(EcoString),
    /// Raw bytes.
    Bytes(Bytes),
}

cast! {
    DataSource,
    self => match self {
        Self::Path(v) => v.into_value(),
        Self::Bytes(v) => v.into_value(),
    },
    v: EcoString => Self::Path(v),
    v: Bytes => Self::Bytes(v),
}

/// Loads data from a path or provided bytes.
pub trait Load {
    /// Bytes or a list of bytes (if there are multiple sources).
    type Output;

    /// Load the bytes.
    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Self::Output>;
}

impl Load for Spanned<DataSource> {
    type Output = Loaded;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Self::Output> {
        self.as_ref().load(world)
    }
}

impl Load for Spanned<&DataSource> {
    type Output = Loaded;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Self::Output> {
        match &self.v {
            DataSource::Path(path) => {
                let file_id = self.span.resolve_path(path).at(self.span)?;
                let bytes = world.file(file_id).at(self.span)?;
                let source = Spanned::new(LoadSource::Path(file_id), self.span);
                Ok(Loaded::new(source, bytes))
            }
            DataSource::Bytes(bytes) => {
                let source = Spanned::new(LoadSource::Bytes, self.span);
                Ok(Loaded::new(source, bytes.clone()))
            }
        }
    }
}

impl Load for Spanned<OneOrMultiple<DataSource>> {
    type Output = Vec<Loaded>;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Self::Output> {
        self.as_ref().load(world)
    }
}

impl Load for Spanned<&OneOrMultiple<DataSource>> {
    type Output = Vec<Loaded>;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Self::Output> {
        self.v
            .0
            .iter()
            .map(|source| Spanned::new(source, self.span).load(world))
            .collect()
    }
}

/// Data loaded from a [`DataSource`].
#[derive(Clone, Hash)]
pub struct Loaded {
    pub source: Spanned<LoadSource>,
    pub bytes: Bytes,
}

impl Loaded {
    pub fn dummy() -> Self {
        Loaded::new(
            typst_syntax::Spanned::new(LoadSource::Bytes, Span::detached()),
            Bytes::new([]),
        )
    }

    pub fn new(source: Spanned<LoadSource>, bytes: Bytes) -> Self {
        Self { source, bytes }
    }

    pub fn as_str(&self) -> SourceResult<&str> {
        self.bytes.as_str().map_err(|err| {
            // TODO: should the error even be reported in the file if it's possibly binary?
            let start = err.valid_up_to();
            let end = start + err.error_len().unwrap_or(0);
            self.err_in_text(start..end, "failed to convert to string", FileError::from(err))
        })
    }

    /// Report an error, possibly in an external file.
    pub fn err_in_text(
        &self,
        pos: impl Into<ReportPos>,
        msg: impl std::fmt::Display,
        error: impl std::fmt::Display,
    ) -> EcoVec<SourceDiagnostic> {
        let pos = pos.into();
        let error = match self.source.v {
            LoadSource::Path(file_id) => {
                if let Some(range) = pos.range(self.bytes.as_slice()) {
                    let span = Span::from_range(file_id, range);
                    return eco_vec!(error!(span, "{msg} ({error})"));
                }

                // Either there was no range provided, or resolving the range
                // from the line/column failed. If present report the possibly
                // wrong line/column anyway.
                let span = Span::from_range(file_id, 0..self.bytes.len());
                if let Some(pair) = pos.line_col(self.bytes.as_slice()) {
                    let (line, col) = pair.numbers();
                    error!(span, "{msg} ({error} at {line}:{col})")
                } else {
                    error!(span, "{msg} ({error})")
                }
            }
            LoadSource::Bytes => {
                if let Some(pair) = pos.line_col(self.bytes.as_slice()) {
                    let (line, col) = pair.numbers();
                    error!(self.source.span, "{msg} ({error} at {line}:{col})")
                } else {
                    error!(self.source.span, "{msg} ({error})")
                }
            }
        };
        eco_vec![error]
    }
}

/// A loaded [`DataSource`].
#[derive(Clone, Copy, Hash)]
pub enum LoadSource {
    Path(FileId),
    Bytes,
}

#[derive(Debug, Default)]
pub enum ReportPos {
    /// Contains the range, and the 0-based line/column.
    Full(std::ops::Range<usize>, LineCol),
    /// Contains the range.
    Range(std::ops::Range<usize>),
    /// Contains the 0-based line/column.
    LineCol(LineCol),
    #[default]
    None,
}

impl From<std::ops::Range<usize>> for ReportPos {
    fn from(value: std::ops::Range<usize>) -> Self {
        Self::Range(value)
    }
}

impl From<LineCol> for ReportPos {
    fn from(value: LineCol) -> Self {
        Self::LineCol(value)
    }
}

impl ReportPos {
    fn range(&self, bytes: &[u8]) -> Option<std::ops::Range<usize>> {
        match self {
            ReportPos::Full(range, _) => Some(range.clone()),
            ReportPos::Range(range) => Some(range.clone()),
            &ReportPos::LineCol(pair) => pair.byte_pos(bytes).map(|i| i..i),
            ReportPos::None => None,
        }
    }

    fn line_col(&self, bytes: &[u8]) -> Option<LineCol> {
        match self {
            &ReportPos::Full(_, pair) => Some(pair),
            ReportPos::Range(range) => LineCol::from_byte_pos(range.start, bytes),
            &ReportPos::LineCol(pair) => Some(pair),
            ReportPos::None => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LineCol {
    /// The 0-based line.
    line: usize,
    /// The 0-based column.
    col: usize,
}

impl LineCol {
    /// Constructs the line/column pair from 0-based indices.
    pub fn zero_based(line: usize, col: usize) -> Self {
        Self { line, col }
    }

    /// Constructs the line/column pair from 1-based numbers.
    pub fn one_based(line: usize, col: usize) -> Self {
        Self {
            line: line.saturating_sub(1),
            col: col.saturating_sub(1),
        }
    }

    pub fn from_byte_pos(pos: usize, bytes: &[u8]) -> Option<Self> {
        let bytes = &bytes[..pos];
        let mut line = 0;
        let line_start = memchr::memchr_iter(b'\n', bytes)
            .inspect(|_| line += 1)
            .last()
            .map(|i| i + 1)
            .unwrap_or(bytes.len());

        // Try to compute a column even if the string isn't valid utf-8.
        let col = ErrorReportingUtf8Chars::new(&bytes[line_start..]).count();
        Some(LineCol::zero_based(line, col))
    }

    pub fn byte_pos(&self, bytes: &[u8]) -> Option<usize> {
        let line_offset = if let Some(idx) = self.line.checked_sub(1) {
            memchr::memchr_iter(b'\n', bytes).nth(idx).map(|i| i + 1)?
        } else {
            0
        };

        let col_offset = col_offset(line_offset, self.col, bytes)?;
        let pos = line_offset + col_offset;
        Some(pos)
    }

    pub fn byte_range(
        range: std::ops::Range<Self>,
        bytes: &[u8],
    ) -> Option<std::ops::Range<usize>> {
        let mut line_iter = memchr::memchr_iter(b'\n', bytes);
        let start_line_offset = if let Some(idx) = range.start.line.checked_sub(1) {
            line_iter.nth(idx).map(|i| i + 1)?
        } else {
            0
        };
        let line_delta = range.end.line - range.start.line;
        let end_line_offset = if let Some(idx) = line_delta.checked_sub(1) {
            line_iter.nth(idx).map(|i| i + 1)?
        } else {
            start_line_offset
        };

        let start_col_offset = col_offset(start_line_offset, range.start.col, bytes)?;
        let end_col_offset = col_offset(end_line_offset, range.end.col, bytes)?;

        let start = start_line_offset + start_col_offset;
        let end = end_line_offset + end_col_offset;
        Some(start..end)
    }

    pub fn numbers(&self) -> (usize, usize) {
        (self.line + 1, self.col + 1)
    }
}

fn col_offset(line_offset: usize, col: usize, bytes: &[u8]) -> Option<usize> {
    let line = &bytes[line_offset..];
    // TODO: streaming-utf8 decoding ignore invalid characters
    // might neeed to update error reporting too (use utf8_iter)
    if let Some(idx) = col.checked_sub(1) {
        // Try to compute position even if the string isn't valid utf-8.
        let mut iter = ErrorReportingUtf8Chars::new(line);
        _ = iter.nth(idx)?;
        Some(line.len() - iter.as_slice().len())
    } else {
        Some(0)
    }
}

/// A value that can be read from a file.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Readable {
    /// A decoded string.
    Str(Str),
    /// Raw bytes.
    Bytes(Bytes),
}

impl Readable {
    pub fn into_bytes(self) -> Bytes {
        match self {
            Self::Bytes(v) => v,
            Self::Str(v) => Bytes::from_string(v),
        }
    }

    pub fn into_source(self) -> DataSource {
        DataSource::Bytes(self.into_bytes())
    }
}

cast! {
    Readable,
    self => match self {
        Self::Str(v) => v.into_value(),
        Self::Bytes(v) => v.into_value(),
    },
    v: Str => Self::Str(v),
    v: Bytes => Self::Bytes(v),
}
