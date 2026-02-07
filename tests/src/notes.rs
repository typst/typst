//! Test sources and error/warning annotations in tests.
//!
//! Note that as of Feb 2025, there are only around 1400 annotations total in
//! the test suite, so optimizations here should be for developer comfort, not
//! size/speed.
use std::fmt::{self, Display, Formatter, Write};
use std::ops::Range;
use std::path::Path;
use std::sync::LazyLock;

use typst::diag::{StrResult, bail};
use ecow::EcoString;
use regex::{Captures, Regex};
use typst::{World, WorldExt as _};
use typst_syntax::package::PackageVersion;
use typst_syntax::{Lines, RootedPath, Source, Span, VirtualPath, VirtualRoot};
use unscanny::Scanner;

use crate::collect::{FilePos, TestParseError, TestStages};
use crate::world::{TestBase, TestFiles, TestWorld};

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
            // Compare by None < Some, external < internal, start-X < start-Y,
            // then end-X < end-Y.
            // Assume external notes are all for the same file (currently true).
            NoteRange::None => None, // Option::None < Option::Some
            NoteRange::Some { source: _, is_external, range, .. } => {
                Some((!is_external, range.start, range.end))
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
                NoteStatus::Emitted => added += 1,
                NoteStatus::Updated { .. } => adjusted += 1,
            }
            match &range {
                NoteRange::None => {
                    writeln!(new, "// {kind}: {message}").unwrap();
                }
                NoteRange::Some { start, end: _, .. } => {
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
        let range = if let Some(id) = span.id()
            && let Some(range) = world.range(span)
        {
            let source = world.source(id).unwrap();
            let lines = source.lines();
            NoteRange::Some {
                start: LineCol::from_index(range.start, lines).unwrap(),
                end: LineCol::from_index(range.end, lines).unwrap(),
                source,
                is_external: id != world.main(),
                range,
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
/// lines/columns for display.
///
/// We use an explicit enum instead of Option so we can implement Display.
#[derive(Debug, Clone)]
pub enum NoteRange {
    None,
    Some {
        source: Source,
        is_external: bool,
        range: Range<usize>,
        start: LineCol,
        end: LineCol,
    },
}

impl NoteRange {
    /// The annotated text of this range for display when logging.
    pub fn text(&self) -> String {
        match self {
            Self::None => "<detached-span>".to_string(),
            Self::Some { source, range, .. } => {
                if range.is_empty() {
                    "<empty>".to_string()
                } else {
                    source.text()[range.clone()].replace("\n", "\\n").replace("\r", "\\r")
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
            NoteRange::Some { is_external: true, .. } => None,
            NoteRange::Some { start, end, .. } => {
                (start.line == end.line).then_some(start.line)
            }
        }
    }

    /// The start and end columns for this range (even if multiline).
    fn columns(&self) -> Option<(usize, usize)> {
        match self {
            NoteRange::None => None,
            NoteRange::Some { start, end, .. } => Some((start.col, end.col)),
        }
    }
}

impl Display for NoteRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // This respects padding and prints more info if formatted with the
        // alternate flag `#` (e.g. format!("{:#}", range)).
        match self {
            Self::None if f.alternate() => f.pad("<no-range>"),
            Self::None => f.pad(""),
            Self::Some { source, range: _, start, end, is_external } => {
                let mut buffer = String::new();
                if *is_external {
                    let path = TestFiles.resolve(source.id());
                    write!(buffer, "\"{}\" ", path.display())?;
                }
                let must_print_column = f.alternate() || *is_external;
                if start == end {
                    if must_print_column {
                        write!(buffer, "{start}")?;
                    } else {
                        write!(buffer, "{}", start.col + 1)?;
                    }
                } else if start.line == end.line && !must_print_column {
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
                Self::Some { is_external: false, range: r1, .. },
                Self::Some { is_external: false, range: r2, .. },
            ) => r1 == r2,
            (
                Self::Some { source: s1, range: r1, .. },
                Self::Some { source: s2, range: r2, .. },
            ) => s1.id() == s2.id() && r1 == r2,
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
        let Some(index) = lines.line_column_to_byte(self.line, self.col) else {
            bail!("column {} is out-of-range for line {}", self.col + 1, self.line + 1);
        };
        Ok(index)
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
pub fn parse_note(
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

/// Parse the range of an annotation, either internal to this file or
/// external at the given path.
fn parse_note_range(
    s: &mut Scanner,
    annotated_line: usize,
    mut source: Source,
) -> StrResult<NoteRange> {
    let is_external = s.eat_if('"');
    if is_external {
        let path = s.eat_until('"');
        if !s.eat_if('"') {
            bail!("expected a closing quote after file path");
        }
        expect_space_after(s, "file path")?;
        let file = TestFiles::rooted_path(path).intern();
        let Ok(external_source) = TestBase::global().files.source(file) else {
            bail!("errors should only be annotated on valid UTF-8 files");
        };
        source = external_source;
    }

    if !s.at(char::is_numeric) {
        Ok(NoteRange::None)
    } else {
        let lines = source.lines();
        let (start, end) = parse_start_end(s, annotated_line, is_external)?;
        let range = start.to_index(lines)?..end.to_index(lines)?;
        expect_space_after(s, "range")?;
        Ok(NoteRange::Some { source, is_external, range, start, end })
    }
}

/// Parse the start and end of a range.
fn parse_start_end(
    s: &mut Scanner,
    annotated_line: usize,
    is_external: bool,
) -> StrResult<(LineCol, LineCol)> {
    // With `<line>:<col>` syntax, the line is actually an offset from the
    // following line.
    let line_base = if is_external { 0 } else { annotated_line };

    let mut had_colon = false;
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
                "expected a single position or either `<line>:<col>-<line>:<col>` \
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
