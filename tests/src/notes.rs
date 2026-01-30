//! Error and warning annotations in tests.
//!
//! Note that as of Feb 2025, there are only around 1400 annotations total in
//! the test suite, so optimizations here should be for developer comfort, not
//! size/speed.
use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::str::FromStr;

use typst::foundations::Bytes;
use typst_kit::files::FileLoader;
use typst_syntax::package::PackageVersion;
use typst_syntax::{FileId, Source, is_id_continue, is_newline};
use unscanny::Scanner;

use crate::collect::{FilePos, TestStages};
use crate::world::TestFiles;

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

impl FromStr for NoteKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Error" => Self::Error,
            "Warning" => Self::Warning,
            "Hint" => Self::Hint,
            _ => return Err(()),
        })
    }
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

/// Parse the start of a note into a kind.
pub fn parse_note_start(s: &mut Scanner) -> Option<NoteKind> {
    s.eat_while(' ');
    if s.eat_if("// ")
        && let head = s.eat_while(is_id_continue)
        && s.eat_if(':')
        && let Ok(kind) = head.parse::<NoteKind>()
    {
        s.eat_if(' ');
        Some(kind)
    } else {
        None
    }
}

/// Parses an annotation in a test, continuing from `parse_note_start`.
pub fn parse_note(
    pos: FilePos,
    line_idx: usize,
    s: &mut Scanner,
    kind: NoteKind,
    source: &Source,
) -> Result<Note, String> {
    let mut file = None;
    if s.eat_if('"') {
        let path = s.eat_until(|c| is_newline(c) || c == '"');
        if !s.eat_if('"') {
            return Err("expected closing quote after file path".to_string());
        }

        file = Some(TestFiles::rooted_path(path).intern());

        s.eat_if(' ');
    }

    let mut range = None;
    if s.at('-') || s.at(char::is_numeric) {
        if let Some(file) = file {
            range = parse_range_external(s, file)?;
        } else {
            range = parse_range_internal(s, line_idx, source);
        }

        if range.is_none() {
            return Err("range is malformed".to_string());
        }
    }

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

/// Parse a range in an external file, optionally abbreviated as just a position
/// if the range is empty.
fn parse_range_external(
    s: &mut Scanner,
    file: FileId,
) -> Result<Option<Range<usize>>, String> {
    let bytes = match TestFiles.load(file) {
        Ok(data) => Bytes::new(data),
        Err(err) => return Err(err.to_string()),
    };

    let Some(start) = parse_line_col(s)? else { return Ok(None) };
    let lines = bytes.lines().expect(
        "errors shouldn't be annotated for files \
             that aren't human readable (not valid UTF-8)",
    );
    let range = if s.eat_if('-') {
        let (line, col) = start;
        let start = lines.line_column_to_byte(line, col);
        let Some((line, col)) = parse_line_col(s)? else { return Ok(None) };
        let end = lines.line_column_to_byte(line, col);
        Option::zip(start, end).map(|(a, b)| a..b)
    } else {
        let (line, col) = start;
        lines.line_column_to_byte(line, col).map(|i| i..i)
    };
    if range.is_none() {
        return Err("range is out of bounds".to_string());
    }
    Ok(range)
}

/// Parses absolute `line:column` indices in an external file.
fn parse_line_col(s: &mut Scanner) -> Result<Option<(usize, usize)>, String> {
    let Some(line) = parse_number(s) else { return Ok(None) };
    if !s.eat_if(':') {
        return Err(
            "positions in external files always require both `<line>:<col>`".to_string()
        );
    }
    let Some(col) = parse_number(s) else { return Ok(None) }; // TODO: This is incorrect.
    if line < 0 || col < 0 {
        return Err("line and column numbers must be positive".to_string());
    }

    Ok(Some(((line as usize).saturating_sub(1), (col as usize).saturating_sub(1))))
}

/// Parse a range, optionally abbreviated as just a position if the range
/// is empty.
fn parse_range_internal(
    s: &mut Scanner,
    line_idx: usize,
    source: &Source,
) -> Option<Range<usize>> {
    let start = parse_position(s, line_idx, source)?;
    let end = if s.eat_if('-') { parse_position(s, line_idx, source)? } else { start };
    Some(start..end)
}

/// Parses a relative `(line:)?column` position.
fn parse_position(s: &mut Scanner, line_idx: usize, source: &Source) -> Option<usize> {
    let first = parse_number(s)?;
    let (line_delta, column) =
        if s.eat_if(':') { (first, parse_number(s)?) } else { (1, first) };

    let text = source.text();
    let comments = text
        .lines()
        .skip(line_idx + 1)
        .take_while(|line| line.trim().starts_with("//"))
        .count();

    let line_idx = (line_idx + comments).checked_add_signed(line_delta)?;
    let column_idx = if column < 0 {
        // Negative column index is from the back.
        let range = source.lines().line_to_range(line_idx)?;
        text[range].chars().count().saturating_add_signed(column)
    } else {
        usize::try_from(column).ok()?.checked_sub(1)?
    };

    source.lines().line_column_to_byte(line_idx, column_idx)
}

/// Parse a number.
fn parse_number(s: &mut Scanner) -> Option<isize> {
    let start = s.cursor();
    s.eat_if('-');
    s.eat_while(char::is_numeric);
    s.from(start).parse().ok()
}
