//! Test sources and error/warning annotations in tests.
//!
//! Note that as of Feb 2025, there are only around 1400 annotations total in
//! the test suite, so optimizations here should be for developer comfort, not
//! size/speed.
use std::fmt::{self, Display, Formatter, Write};
use std::ops::Range;
use std::path::Path;
use std::sync::LazyLock;

use ecow::EcoString;
use regex::{Captures, Regex};
use typst::diag::{StrResult, bail};
use typst::foundations::Bytes;
use typst::{World, WorldExt as _};
use typst_kit::files::FileLoader as _;
use typst_syntax::package::PackageVersion;
use typst_syntax::{FileId, Lines, RootedPath, Source, Span, VirtualPath, VirtualRoot};
use unscanny::Scanner;

use crate::collect::{FilePos, TestParseError, TestStages};
use crate::world::{TestFiles, TestWorld};

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

    /// Search for a matching note to mark as seen, or update the notes vector
    /// by changing a close match or appending a new emitted note.
    pub fn mark_seen_or_update(
        &mut self,
        Note { status, seen, kind, range, message }: Note,
    ) {
        let mut best = None;
        let mut range_message_lines_cols = [false; 4];
        for (i, note) in self.notes.iter_mut().enumerate() {
            // Only consider notes that haven't been seen yet.
            if note.seen.contains(seen) {
                continue;
            }
            let same_range = note.range == range;
            let same_message = note.message == message;

            // If kind differs, we skip unless both range and message match.
            if note.kind != kind {
                if same_range
                    && same_message
                    && let NoteStatus::Annotated { pos } = &note.status
                {
                    best = Some((i, pos.clone()));
                    break;
                } else {
                    continue;
                }
            } else if same_range && same_message {
                // A perfect match! Mark as seen and return.
                note.seen |= seen;
                return;
            } else if let NoteStatus::Annotated { pos } = &note.status
                && note.seen.is_empty()
            {
                // Order the other possible matches by comparing the range, then
                // message, then single lines, then columns. If none match exactly,
                // treat as not emitted.
                let same_r_m_l_c = [
                    same_range,
                    same_message,
                    note.range.single_line() == range.single_line(),
                    note.range.columns() == range.columns(),
                ];
                if same_r_m_l_c > range_message_lines_cols {
                    best = Some((i, pos.clone()));
                    range_message_lines_cols = same_r_m_l_c;
                }
            }
        }
        match best {
            Some((i, pos)) => {
                let note = &mut self.notes[i];
                note.seen |= seen;
                // We replace the notes kind/range/message (which should cause
                // it to match exactly in future stages), but keep the old
                // values to report the difference.
                let annotated = Box::new((
                    std::mem::replace(&mut note.kind, kind),
                    std::mem::replace(&mut note.range, range),
                    std::mem::replace(&mut note.message, message),
                ));
                note.status = NoteStatus::Updated { pos, annotated };
            }
            None => self.notes.push(Note { status, seen, kind, range, message }),
        }
    }

    /// Create a new body for this test with only the seen annotations.
    pub fn write_seen_annotations(&mut self) -> (String, [(&'static str, usize); 3]) {
        let old = self.source.text();
        let mut new = String::with_capacity(old.len());
        let mut lines = old.lines().enumerate().peekable();

        // Stable sort keeps Warning/Error annotations well-ordered relative to
        // their Hints since hints usually have the same range.
        self.notes.sort_by_key(|note| match &note.range {
            // Compare by None < Some, external < internal, start-A < start-B,
            // then end-A < end-B.
            // Assume external notes are all for the same file (currently true).
            NoteRange::None => None, // Option::None < Option::Some
            NoteRange::Some { external_file, indices, .. } => {
                Some((external_file.is_none(), indices.start, indices.end))
            }
        });

        let (mut removed, mut added, mut adjusted) = (0, 0, 0);
        for Note { kind, range, message, status, seen } in &self.notes {
            match status {
                NoteStatus::Annotated { .. } if seen.is_empty() => {
                    removed += 1;
                    continue;
                }
                NoteStatus::Annotated { .. } => {}
                NoteStatus::Updated { .. } => adjusted += 1,
                NoteStatus::Emitted => added += 1,
            }
            match &range {
                NoteRange::None => {
                    writeln!(new, "// {kind}: {message}").unwrap();
                }
                NoteRange::Some { positions: Range { start, .. }, .. } => {
                    while let Some(&(line_index, line)) = lines.peek()
                        && line_index < start.line
                    {
                        new.push_str(line);
                        new.push('\n');
                        lines.next();
                    }
                    // Indent the same as the line being annotated.
                    if let Some(&(_, line)) = lines.peek() {
                        new.push_str(Scanner::new(line).eat_while(' '));
                    }
                    writeln!(new, "// {kind}: {range} {message}").unwrap();
                }
            }
        }
        for (_, line) in lines {
            new.push_str(line);
            new.push('\n');
        }

        let stats = [("removed", removed), ("added", added), ("adjusted", adjusted)];
        (new, stats)
    }
}

/// An annotation like `// Error: 2-6 message` in a test.
pub struct Note {
    pub status: NoteStatus,
    pub seen: TestStages,
    pub kind: NoteKind,
    pub range: NoteRange,
    pub message: String,
}

/// The status of an annotated diagnostic.
pub enum NoteStatus {
    /// An annotated note that has only seen perfect matches.
    Annotated { pos: FilePos },
    /// An annotated note that saw a close match and has been updated. Retains
    /// its orginal kind, range, and message.
    Updated { pos: FilePos, annotated: Box<(NoteKind, NoteRange, String)> },
    /// An emitted note that was not annotated.
    Emitted,
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

impl Note {
    /// Build a new emitted note from a span and world.
    pub fn emitted(
        kind: NoteKind,
        stage: TestStages,
        message: &EcoString,
        span: Span,
        world: &TestWorld,
    ) -> Self {
        let range = if let Some(file) = span.id()
            && let Some(indices) = world.range(span)
        {
            // We don't call `.source()` because file may not be Typst code.
            let lines = world.file(file).unwrap().lines().unwrap();
            let start = LineCol::from_index(indices.start, &lines).unwrap();
            let end = LineCol::from_index(indices.end, &lines).unwrap();
            NoteRange::Some {
                lines,
                external_file: (file != world.main()).then_some(file),
                indices,
                positions: start..end,
            }
        } else {
            assert!(span.is_detached());
            NoteRange::None
        };

        let mut message: String = message.into();
        if message.contains('\\') {
            // HACK: Replace backslashes in path sepators with slashes for cross
            // platform reproducible error messages.
            static RE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new("\\((.*) (at|in) (.+)\\)").unwrap());
            message = RE
                .replace(&message, |caps: &Captures| {
                    let path = caps[3].replace('\\', "/");
                    format!("({} {} {})", &caps[1], &caps[2], path)
                })
                .into();
        }

        Self {
            status: NoteStatus::Emitted,
            seen: stage,
            kind,
            range,
            message,
        }
    }
}

/// The range of an annotated diagnostic as indices into a source file and as
/// line/column positions for display.
///
/// We use an explicit enum instead of Option so we can implement Display.
#[derive(Clone)]
pub enum NoteRange {
    None,
    Some {
        lines: Lines<String>,
        external_file: Option<FileId>,
        indices: Range<usize>,
        positions: Range<LineCol>,
    },
}

impl NoteRange {
    /// The annotated text of this range for display when logging.
    pub fn text(&self) -> String {
        match self {
            Self::None => "<detached-span>".to_string(),
            Self::Some { lines, indices, .. } => {
                if indices.is_empty() {
                    "<empty>".to_string()
                } else {
                    lines.text()[indices.clone()]
                        .replace("\n", "\\n")
                        .replace("\r", "\\r")
                }
            }
        }
    }

    /// Displayable strings for a difference between two ranges.
    pub fn diff(&self, other: &Self) -> Option<(String, String)> {
        if self == other {
            None
        } else if let Some(l1) = self.single_line()
            && let Some(l2) = other.single_line()
            && l1 == l2
        {
            // If annotating the same line, just print the columns.
            Some((format!("{self}"), format!("{other}")))
        } else {
            Some((format!("{self:#}"), format!("{other:#}")))
        }
    }

    /// Whether this range spans only a single line in the internal source file.
    fn single_line(&self) -> Option<usize> {
        match self {
            NoteRange::None => None,
            NoteRange::Some { external_file: Some(_), .. } => None,
            NoteRange::Some { positions: Range { start, end }, .. } => {
                (start.line == end.line).then_some(start.line)
            }
        }
    }

    /// The start and end columns for this range (even if multiline).
    fn columns(&self) -> Option<(usize, usize)> {
        match self {
            NoteRange::None => None,
            NoteRange::Some { positions: Range { start, end }, .. } => {
                Some((start.col, end.col))
            }
        }
    }
}

impl Display for NoteRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // This respects padding and prints more info if formatted with the
        // alternate flag `#` (e.g. `format!("{:#}", range)`).
        match self {
            Self::None if f.alternate() => f.pad("<no-range>"),
            Self::None => f.pad(""),
            Self::Some { external_file, positions: Range { start, end }, .. } => {
                let mut buffer = String::new();
                if let Some(file) = external_file {
                    let path = TestFiles.resolve(*file);
                    write!(buffer, "\"{}\" ", path.display())?;
                }
                let must_print_lines = f.alternate() || external_file.is_some();
                if start == end {
                    if must_print_lines {
                        write!(buffer, "{start}")?;
                    } else {
                        write!(buffer, "{}", start.col + 1)?;
                    }
                } else if start.line == end.line && !must_print_lines {
                    write!(buffer, "{}-{}", start.col + 1, end.col + 1)?;
                } else {
                    write!(buffer, "{start}-{end}")?;
                }
                f.pad(&buffer)
            }
        }
    }
}

// Note that we should only ever compare ranges from the same test.
impl PartialEq for NoteRange {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (
                Self::Some { external_file: f1, indices: i1, .. },
                Self::Some { external_file: f2, indices: i2, .. },
            ) => f1 == f2 && i1 == i2,
            _ => false,
        }
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

    fn from_index(index: usize, lines: &Lines<String>) -> Option<Self> {
        let (line, col) = lines.byte_to_line_column(index)?;
        Some(Self { line, col })
    }
}

impl Display for LineCol {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line + 1, self.col + 1)
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
        match parse_note(note_pos, annotated_line, &mut s, kind, source.clone()) {
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
    source: Source,
) -> StrResult<Note> {
    expect_space_after(s, "annotation kind")?;

    let range = parse_note_range(s, annotated_line, source)?;

    let message = s
        .after()
        .trim()
        .replace("VERSION", &format!("{}", PackageVersion::compiler()))
        .replace("\\n", "\n");

    Ok(Note {
        status: NoteStatus::Annotated { pos },
        seen: TestStages::empty(),
        kind,
        range,
        message,
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
    source: Source,
) -> StrResult<NoteRange> {
    let (lines, external_file) = if s.eat_if('"') {
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
        (lines, Some(file))
    } else {
        (source.lines().clone(), None)
    };

    if !s.at(char::is_numeric) {
        Ok(NoteRange::None)
    } else {
        let positions = parse_line_col_range(s, annotated_line, external_file.is_some())?;
        let start_index = positions.start.to_index(&lines)?;
        let end_index = positions.end.to_index(&lines)?;
        let indices = start_index..end_index;
        expect_space_after(s, "range")?;
        Ok(NoteRange::Some { lines, external_file, indices, positions })
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
) -> StrResult<Range<LineCol>> {
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

    Ok(start..end)
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
