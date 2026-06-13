use az::SaturatingAs;
use typst_library::{
    diag::{LineCol, LoadError, ReportTextPos},
    routines::{CsvReader, CsvReaderBuilder, CsvRecords},
};

pub(crate) struct ReaderBuilder {
    builder: ::csv::ReaderBuilder,
    line_offset: usize,
}

impl ReaderBuilder {
    pub(crate) fn new() -> Self {
        // By default, no header, so first data line is at line 1.
        Self {
            builder: ::csv::ReaderBuilder::new(),
            line_offset: 1,
        }
    }
}

impl CsvReaderBuilder for ReaderBuilder {
    fn has_headers(&mut self, has_headers: bool) {
        // Body lines start from 2 if there is a header line.
        self.line_offset = if has_headers { 2 } else { 1 };
        self.builder.has_headers(has_headers);
    }

    fn delimiter(&mut self, delimiter: u8) {
        self.builder.delimiter(delimiter);
    }

    fn create_reader<'a>(&self, data: &'a [u8]) -> Box<dyn CsvReader + 'a> {
        Box::new(Reader {
            reader: self.builder.from_reader(data),
            line_offset: self.line_offset,
        })
    }
}

struct Reader<'a> {
    reader: ::csv::Reader<&'a [u8]>,
    line_offset: usize,
}

impl CsvReader for Reader<'_> {
    fn header(&mut self) -> Result<Box<dyn CsvRecords>, typst_library::diag::LoadError> {
        self.reader
            .headers()
            .cloned()
            .map_err(|err| format_csv_error(err, 1))
            .map(|res| Box::new(RecordWrapper(res)) as Box<dyn CsvRecords>)
    }

    fn records<'a>(
        &'a mut self,
    ) -> Box<
        dyn Iterator<
                Item = Result<Box<dyn CsvRecords + 'a>, typst_library::diag::LoadError>,
            > + 'a,
    > {
        Box::new(self.reader.records().enumerate().map(|(line, record)| {
            record
                .map_err(|err| format_csv_error(err, line + self.line_offset))
                .map(|res| Box::new(RecordWrapper(res)) as Box<dyn CsvRecords>)
        }))
    }
}

struct RecordWrapper(::csv::StringRecord);

impl CsvRecords for RecordWrapper {
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a str> + 'a> {
        Box::new(self.0.iter())
    }
}

/// Format the user-facing CSV error message.
fn format_csv_error(err: ::csv::Error, line: usize) -> LoadError {
    let msg = "failed to parse CSV";
    let pos = (err.kind().position())
        .map(|pos| {
            let start = pos.byte().saturating_as();
            ReportTextPos::from(start..start)
        })
        .unwrap_or(LineCol::one_based(line, 1).into());
    match err.kind() {
        ::csv::ErrorKind::Utf8 { .. } => {
            LoadError::text(pos, msg, "file is not valid UTF-8")
        }
        ::csv::ErrorKind::UnequalLengths { expected_len, len, .. } => {
            let err =
                format!("found {len} instead of {expected_len} fields in line {line}");
            LoadError::text(pos, msg, err)
        }
        _ => LoadError::text(pos, "failed to parse CSV", err),
    }
}
