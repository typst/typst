//! Test sources and error/warning annotations in tests.
//!
//! Note that as of Feb 2025, there are only around 1400 annotations total in
//! the test suite, so optimizations here should be for developer comfort, not
//! size/speed.
use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::path::Path;

use typst::diag::{StrResult, bail};
use typst::foundations::Bytes;
use typst_kit::files::FileLoader;
use typst_syntax::package::PackageVersion;
use typst_syntax::{
    FileId, Lines, RootedPath, Source, VirtualPath, VirtualRoot
};
use unscanny::Scanner;

use crate::collect::{FilePos, TestParseError, TestStages};
use crate::world::TestFiles;

/// The body of a test.
pub struct TestBody {
    /// The start of the body.
    pub pos: FilePos,
    /// The source of the body, excluding annotation comments.
    pub source: Source,
    /// The annotation comments for this test.
    pub notes: Vec<Note>,
}

impl TestBody {
    /// Whether there are any annotated errors.
    pub fn has_error(&self) -> bool {
        self.notes.iter().any(|n| n.kind == NoteKind::Error)
    }
}

/// An annotation like `// Error: 2-6 message` in a test.
pub struct Note {
    pub pos: FilePos,
    pub kind: NoteKind,
    /// The file [`Self::range`] belongs to.
    pub file: FileId,
    pub range: Option<Range<usize>>,
    pub message: String,
    pub seen: TestStages,
}

/// A kind of annotation in a test.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum NoteKind {
    Error,
    Warning,
    Hint,
}

impl Display for NoteKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
            Self::Hint => "Hint",
        })
    }
}

/// Line and column indices for a range. Stored with 0-indexing, but displayed
/// with 1-indexing.
///
/// Note that columns are char-indices, not bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineCol {
    pub line: usize,
    pub col: usize,
}

impl LineCol {
    fn to_index(self, lines: &Lines<String>) -> StrResult<usize> {
        if lines.line_to_range(self.line).is_none() {
            bail!("line {} is out-of-range", self.line + 1);
        }
        if let Some(index) = lines.line_column_to_byte(self.line, self.col) {
            Ok(index)
        } else {
            bail!("column {} is out-of-range for line {}", self.col + 1, self.line + 1);
        }
    }
}

/// Parse the body of a test.
pub fn parse_test_body(
    pos: FilePos,
    full_body: &str,
    errors: &mut Vec<TestParseError>,
) -> TestBody {
    // First parse the source string, adding note lines to a separate vec.
    let mut body = String::with_capacity(full_body.len());
    let mut annotated_line = 0;
    let mut note_lines: Vec<(usize, usize, NoteKind, Scanner)> = vec![];
    for (i, line) in full_body.lines().enumerate() {
        let mut s = Scanner::new(line);
        if let Some(kind) = parse_note_start(&mut s) {
            // Notes annotate the next non-note line.
            note_lines.push((i, annotated_line, kind, s));
        } else {
            body.push_str(line);
            body.push('\n');
            annotated_line += 1;
        }
    }

    // Then create a source file for the test body.
    let vpath = VirtualPath::virtualize(Path::new(""), &pos.path).unwrap();
    let source = Source::new(RootedPath::new(VirtualRoot::Project, vpath).intern(), body);

    // Finish by actually parsing the note lines now that we have the body.
    let mut notes = Vec::with_capacity(note_lines.len());
    for (i, annotated_line, kind, mut s) in note_lines {
        let line = pos.line + i;
        let note_pos = FilePos { path: pos.path.clone(), line };
        match parse_note(note_pos, annotated_line, &mut s, kind, &source) {
            Ok(note) => notes.push(note),
            Err(message) => errors.push(TestParseError::new(message, &pos.path, line)),
        }
    }

    TestBody { pos, source, notes }
}

/// Parse the start of a note into a kind.
fn parse_note_start(s: &mut Scanner) -> Option<NoteKind> {
    s.eat_while(' ');
    for (pattern, kind) in [
        ("// Error:", NoteKind::Error),
        ("// Warning:", NoteKind::Warning),
        ("// Hint:", NoteKind::Hint),
    ] {
        if s.eat_if(pattern) {
            return Some(kind);
        }
    }
    None
}

/// Parses an annotation in a test, continuing from `parse_note_start`.
fn parse_note(
    pos: FilePos,
    annotated_line: usize,
    s: &mut Scanner,
    kind: NoteKind,
    source: &Source,
) -> StrResult<Note> {
    expect_space_after(s, "annotation kind")?;

    let (file, range) = parse_note_range(s, annotated_line, source)?;

    let message = s
        .after()
        .trim()
        .replace("VERSION", &format!("{}", PackageVersion::compiler()))
        .replace("\\n", "\n");

    Ok(Note {
        pos,
        kind,
        file: file.unwrap_or(source.id()),
        range,
        message,
        seen: TestStages::empty(),
    })
}

/// Eat a space or return an error that a space was expected.
fn expect_space_after(s: &mut Scanner, thing: &str) -> StrResult<()> {
    if s.eat_if(' ') { Ok(()) } else { bail!("expected a space after {thing}") }
}

/// Parse the range of an annotation, either internal to this test or external
/// at the given path.
fn parse_note_range(
    s: &mut Scanner,
    annotated_line: usize,
    source: &Source,
) -> StrResult<(Option<FileId>, Option<Range<usize>>)> {
    let external_lines;
    let (file, lines) = if s.eat_if('"') {
        let path = s.eat_until('"');
        if !s.eat_if('"') {
            bail!("expected a closing quote after file path");
        }
        expect_space_after(s, "file path")?;
        let file = TestFiles::rooted_path(path).intern();
        let data = TestFiles.load(file).map_err(|err| err.to_string())?;
        let Ok(lines) = Bytes::new(data).lines() else {
            bail!("errors should only be annotated on valid UTF-8 files");
        };
        external_lines = lines;
        (Some(file), &external_lines)
    } else {
        (None, source.lines())
    };

    if !s.at(char::is_numeric) {
        Ok((file, None))
    } else {
        let (start, end) = parse_line_col_range(s, annotated_line, file.is_some())?;
        let start = start.to_index(lines)?;
        let end = end.to_index(lines)?;

        expect_space_after(s, "range")?;
        Ok((file, Some(start..end)))
    }
}

/// Parse the human-readable line-column range being annotated.
///
/// This can be any of:
/// - No range (handled by caller)
/// - A full line/column range: `<line>:<col>-<line>:<col>`
/// - A column range on a single line: `<col>-<col>`
/// - A line/column position: `<line>:<col>`
/// - A column position: `<col>`
///
/// Note that columns are character indices, not byte indices.
///
/// For an internal file, the line is an offset from the the next non-annotation
/// line in the test body. For an external annotation, both line and column are
/// required.
///
/// For example, in:
/// ```typ
/// --- example-annotation-test eval ---
/// // Error: 1-2 First
/// // Error: 2:1-3:2 Second
/// A
/// // Error: 2 Third
/// B
/// C
/// // Error: "tests/README.md" 1:5 Fourth
/// ```
/// - `First` annotates "A"
/// - `Second` annotates "B\nC"
/// - `Third` annotates the position immediately after B
/// - `Fourth` annotates the position after the fifth character in the README
fn parse_line_col_range(
    s: &mut Scanner,
    annotated_line: usize,
    is_external: bool,
) -> StrResult<(LineCol, LineCol)> {
    let mut had_colon = false;
    let line_base = if is_external { 0 } else { annotated_line };

    let start = {
        let position = parse_position_number(s)?;
        if s.eat_if(':') {
            let col = parse_position_number(s)?;
            had_colon = true;
            LineCol { line: position + line_base, col }
        } else if !is_external {
            LineCol { line: line_base, col: position }
        } else {
            bail!("positions in external files require line and column: `<line>:<col>`");
        }
    };

    let end = if !s.eat_if('-') {
        start // Just a position, reuse start as the end.
    } else {
        let position = parse_position_number(s)?;
        if had_colon && s.eat_if(':') {
            let col = parse_position_number(s)?;
            LineCol { line: position + line_base, col }
        } else if !had_colon && !s.at(':') {
            LineCol { line: start.line, col: position }
        } else if is_external {
            bail!("expected either `<line>:<col>` or `<line>:<col>-<line>:<col>`");
        } else {
            bail!(
                "expected a single position or a range of `<line>:<col>-<line>:<col>` \
                 or `<col>-<col>`"
            );
        }
    };

    if start.line > end.line {
        bail!(
            "start-line is greater than end-line: {} > {}",
            start.line + 1,
            end.line + 1,
        );
    } else if start.line == end.line && start.col > end.col {
        bail!(
            "start-column is greater than end-column: {} > {}",
            start.col + 1,
            end.col + 1,
        );
    }

    Ok((start, end))
}

/// Parse a number for a line or column position.
fn parse_position_number(s: &mut Scanner) -> StrResult<usize> {
    let text = s.eat_while(char::is_numeric);
    if text.is_empty() {
        bail!("expected a range position number")
    } else {
        let n: usize = text.parse().unwrap();
        if n == 0 {
            bail!("0 is not a valid position number, use 1 instead")
        } else {
            Ok(n - 1)
        }
    }
}
