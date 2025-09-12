use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::LazyLock;

use ecow::{EcoString, eco_format};
use rustc_hash::{FxHashMap, FxHashSet};
use typst_syntax::package::PackageVersion;
use typst_syntax::{
    FileId, Lines, Source, VirtualPath, is_id_continue, is_ident, is_newline,
};
use unscanny::Scanner;

/// Collects all tests from all files.
///
/// Returns:
/// - the tests and the number of skipped tests in the success case.
/// - parsing errors in the failure case.
pub fn collect() -> Result<(Vec<Test>, usize), Vec<TestParseError>> {
    Collector::new().collect()
}

/// A single test.
pub struct Test {
    pub pos: FilePos,
    pub name: EcoString,
    pub attrs: Vec<Attr>,
    pub source: Source,
    pub notes: Vec<Note>,
}

impl Display for Test {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // underline path
        write!(f, "{} (\x1B[4m{}\x1B[0m)", self.name, self.pos)
    }
}

/// A position in a file.
#[derive(Clone)]
pub struct FilePos {
    pub path: PathBuf,
    pub line: usize,
}

impl FilePos {
    fn new(path: impl Into<PathBuf>, line: usize) -> Self {
        Self { path: path.into(), line }
    }
}

impl Display for FilePos {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.line > 0 {
            write!(f, "{}:{}", self.path.display(), self.line)
        } else {
            write!(f, "{}", self.path.display())
        }
    }
}

/// A test attribute, given after the test name.
#[derive(Clone, Debug, PartialEq)]
pub enum Attr {
    Html,
    Render,
    Large,
    Pdftags,
}

/// The size of a file.
pub struct FileSize(pub usize);

impl Display for FileSize {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:.2} KiB", (self.0 as f64) / 1024.0)
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

/// Collects all tests from all files.
struct Collector {
    tests: Vec<Test>,
    errors: Vec<TestParseError>,
    seen: FxHashMap<EcoString, (FilePos, Vec<Attr>)>,
    skipped: usize,
}

impl Collector {
    /// Creates a new test collector.
    fn new() -> Self {
        Self {
            tests: vec![],
            errors: vec![],
            seen: FxHashMap::default(),
            skipped: 0,
        }
    }

    /// Collects tests from all files.
    fn collect(mut self) -> Result<(Vec<Test>, usize), Vec<TestParseError>> {
        self.walk_files();
        self.walk_references();

        if self.errors.is_empty() {
            Ok((self.tests, self.skipped))
        } else {
            Err(self.errors)
        }
    }

    /// Walks through all test files and collects the tests.
    fn walk_files(&mut self) {
        for entry in walkdir::WalkDir::new(crate::SUITE_PATH).sort_by_file_name() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "typ") {
                continue;
            }

            let text = std::fs::read_to_string(path).unwrap();
            if text.starts_with("// SKIP") {
                continue;
            }

            Parser::new(self, path, &text).parse();
        }
    }

    /// Walks through all reference output and ensures that a test exists for
    /// each one.
    fn walk_references(&mut self) {
        for entry in walkdir::WalkDir::new(crate::REF_PATH).sort_by_file_name() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "png" && ext != "html") {
                continue;
            }

            let stem = path.file_stem().unwrap().to_string_lossy();
            let name = &*stem;

            let Some((pos, attrs)) = self.seen.get(name) else {
                self.errors.push(TestParseError {
                    pos: FilePos::new(path, 0),
                    message: "dangling reference output".into(),
                });
                continue;
            };

            let len = path.metadata().unwrap().len() as usize;
            if !attrs.contains(&Attr::Large) && len > crate::REF_LIMIT {
                self.errors.push(TestParseError {
                    pos: pos.clone(),
                    message: format!(
                        "reference output size exceeds {}, but the test is not marked as `large`",
                        FileSize(crate::REF_LIMIT),
                    ),
                });
            }
        }
    }
}

/// Parses a single test file.
struct Parser<'a> {
    collector: &'a mut Collector,
    path: &'a Path,
    s: Scanner<'a>,
    test_start_line: usize,
    line: usize,
}

impl<'a> Parser<'a> {
    /// Creates a new parser for a file.
    fn new(collector: &'a mut Collector, path: &'a Path, source: &'a str) -> Self {
        Self {
            collector,
            path,
            s: Scanner::new(source),
            test_start_line: 1,
            line: 1,
        }
    }

    /// Parses an individual file.
    fn parse(&mut self) {
        self.skip_preamble();

        while !self.s.done() {
            let mut name = EcoString::new();
            let mut attrs = Vec::new();
            let mut notes = vec![];
            if self.s.eat_if("---") {
                self.s.eat_while(' ');
                name = self.s.eat_until(char::is_whitespace).into();
                self.s.eat_while(' ');

                if name.is_empty() {
                    self.error("expected test name");
                } else if !is_ident(&name) {
                    self.error(format!("test name `{name}` is not a valid identifier"));
                } else {
                    attrs = self.parse_attrs();
                }
            } else {
                self.error("expected opening ---");
            }

            if self.collector.seen.contains_key(&name) {
                self.error(format!("duplicate test {name}"));
            }

            if self.s.eat_newline() {
                self.line += 1;
            }

            let start = self.s.cursor();
            self.test_start_line = self.line;

            let pos = FilePos::new(self.path, self.test_start_line);
            self.collector.seen.insert(name.clone(), (pos.clone(), attrs.clone()));

            while !self.s.done() && !self.s.at("---") {
                self.s.eat_until(is_newline);
                if self.s.eat_newline() {
                    self.line += 1;
                }
            }

            let text = self.s.from(start);

            if !selected(&name, self.path.canonicalize().unwrap()) {
                self.collector.skipped += 1;
                continue;
            }

            let vpath = VirtualPath::new(self.path);
            let source = Source::new(FileId::new(None, vpath), text.into());

            self.s.jump(start);
            self.line = self.test_start_line;

            while !self.s.done() && !self.s.at("---") {
                self.s.eat_while(' ');
                if self.s.eat_if("// ") {
                    notes.extend(self.parse_note(&source));
                }

                self.s.eat_until(is_newline);
                if self.s.eat_newline() {
                    self.line += 1;
                }
            }

            self.collector.tests.push(Test { pos, name, source, notes, attrs });
        }
    }

    fn parse_attrs(&mut self) -> Vec<Attr> {
        let mut attrs = vec![];
        while !self.s.eat_if("---") {
            let attr = match self.s.eat_until(char::is_whitespace) {
                "large" => Attr::Large,
                "html" => Attr::Html,
                "render" => Attr::Render,
                "pdftags" => Attr::Pdftags,
                found => {
                    self.error(format!(
                        "expected attribute or closing ---, found `{found}`"
                    ));
                    break;
                }
            };
            if attrs.contains(&attr) {
                self.error(format!("duplicate attribute {attr:?}"));
            }
            attrs.push(attr);
            self.s.eat_while(' ');
        }
        attrs
    }

    /// Skips the preamble of a test.
    fn skip_preamble(&mut self) {
        let mut errored = false;
        while !self.s.done() && !self.s.at("---") {
            let line = self.s.eat_until(is_newline).trim();
            if !errored && !line.is_empty() && !line.starts_with("//") {
                self.error("test preamble may only contain comments and blank lines");
                errored = true;
            }
            if self.s.eat_newline() {
                self.line += 1;
            }
        }
    }

    /// Parses an annotation in a test.
    fn parse_note(&mut self, source: &Source) -> Option<Note> {
        let head = self.s.eat_while(is_id_continue);
        if !self.s.eat_if(':') {
            return None;
        }

        let kind: NoteKind = head.parse().ok()?;
        self.s.eat_if(' ');

        let mut file = None;
        if self.s.eat_if('"') {
            let path = self.s.eat_until(|c| is_newline(c) || c == '"');
            if !self.s.eat_if('"') {
                self.error("expected closing quote after file path");
                return None;
            }

            let vpath = VirtualPath::new(path);
            file = Some(FileId::new(None, vpath));

            self.s.eat_if(' ');
        }

        let mut range = None;
        if self.s.at('-') || self.s.at(char::is_numeric) {
            if let Some(file) = file {
                range = self.parse_range_external(file);
            } else {
                range = self.parse_range(source);
            }

            if range.is_none() {
                self.error("range is malformed");
                return None;
            }
        }

        let message = self
            .s
            .eat_until(is_newline)
            .trim()
            .replace("VERSION", &eco_format!("{}", PackageVersion::compiler()));

        Some(Note {
            pos: FilePos::new(self.path, self.line),
            kind,
            file: file.unwrap_or(source.id()),
            range,
            message,
        })
    }

    #[cfg(not(feature = "default"))]
    fn parse_range_external(&mut self, _file: FileId) -> Option<Range<usize>> {
        panic!("external file ranges are not expected when testing `typst_syntax`");
    }

    /// Parse a range in an external file, optionally abbreviated as just a position
    /// if the range is empty.
    #[cfg(feature = "default")]
    fn parse_range_external(&mut self, file: FileId) -> Option<Range<usize>> {
        use typst::foundations::Bytes;

        use crate::world::{read, system_path};

        let path = match system_path(file) {
            Ok(path) => path,
            Err(err) => {
                self.error(err.to_string());
                return None;
            }
        };

        let bytes = match read(&path) {
            Ok(data) => Bytes::new(data),
            Err(err) => {
                self.error(err.to_string());
                return None;
            }
        };

        let start = self.parse_line_col()?;
        let lines = Lines::try_from(&bytes).expect(
            "errors shouldn't be annotated for files \
            that aren't human readable (not valid utf-8)",
        );
        let range = if self.s.eat_if('-') {
            let (line, col) = start;
            let start = lines.line_column_to_byte(line, col);
            let (line, col) = self.parse_line_col()?;
            let end = lines.line_column_to_byte(line, col);
            Option::zip(start, end).map(|(a, b)| a..b)
        } else {
            let (line, col) = start;
            lines.line_column_to_byte(line, col).map(|i| i..i)
        };
        if range.is_none() {
            self.error("range is out of bounds");
        }
        range
    }

    /// Parses absolute `line:column` indices in an external file.
    fn parse_line_col(&mut self) -> Option<(usize, usize)> {
        let line = self.parse_number()?;
        if !self.s.eat_if(':') {
            self.error("positions in external files always require both `<line>:<col>`");
            return None;
        }
        let col = self.parse_number()?;
        if line < 0 || col < 0 {
            self.error("line and column numbers must be positive");
            return None;
        }

        Some(((line as usize).saturating_sub(1), (col as usize).saturating_sub(1)))
    }

    /// Parse a range, optionally abbreviated as just a position if the range
    /// is empty.
    fn parse_range(&mut self, source: &Source) -> Option<Range<usize>> {
        let start = self.parse_position(source)?;
        let end = if self.s.eat_if('-') { self.parse_position(source)? } else { start };
        Some(start..end)
    }

    /// Parses a relative `(line:)?column` position.
    fn parse_position(&mut self, source: &Source) -> Option<usize> {
        let first = self.parse_number()?;
        let (line_delta, column) =
            if self.s.eat_if(':') { (first, self.parse_number()?) } else { (1, first) };

        let text = source.text();
        let line_idx_in_test = self.line - self.test_start_line;
        let comments = text
            .lines()
            .skip(line_idx_in_test + 1)
            .take_while(|line| line.trim().starts_with("//"))
            .count();

        let line_idx = (line_idx_in_test + comments).checked_add_signed(line_delta)?;
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
    fn parse_number(&mut self) -> Option<isize> {
        let start = self.s.cursor();
        self.s.eat_if('-');
        self.s.eat_while(char::is_numeric);
        self.s.from(start).parse().ok()
    }

    /// Stores a test parsing error.
    fn error(&mut self, message: impl Into<String>) {
        self.collector.errors.push(TestParseError {
            pos: FilePos::new(self.path, self.line),
            message: message.into(),
        });
    }
}

/// Whether a test is within the selected set to run.
fn selected(name: &str, abs: PathBuf) -> bool {
    static SKIPPED: LazyLock<FxHashSet<&'static str>> = LazyLock::new(|| {
        String::leak(std::fs::read_to_string(crate::SKIP_PATH).unwrap())
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .collect()
    });

    if SKIPPED.contains(name) {
        return false;
    }

    let paths = &crate::ARGS.path;
    if !paths.is_empty() && !paths.iter().any(|path| abs.starts_with(path)) {
        return false;
    }

    let exact = crate::ARGS.exact;
    let patterns = &crate::ARGS.pattern;
    patterns.is_empty()
        || patterns.iter().any(|pattern: &regex::Regex| {
            if exact { name == pattern.as_str() } else { pattern.is_match(name) }
        })
}

/// An error in a test file.
pub struct TestParseError {
    pub pos: FilePos,
    pub message: String,
}

impl Display for TestParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.pos)
    }
}

trait ScannerExt {
    fn eat_newline(&mut self) -> bool;
}

impl ScannerExt for Scanner<'_> {
    fn eat_newline(&mut self) -> bool {
        let ate = self.eat_if(is_newline);
        if ate && self.before().ends_with('\r') {
            self.eat_if('\n');
        }
        ate
    }
}
