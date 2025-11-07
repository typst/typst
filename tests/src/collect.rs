use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::LazyLock;

use bitflags::{Flags, bitflags};
use ecow::{EcoString, eco_format};
use rustc_hash::{FxHashMap, FxHashSet};
use typst::foundations::Bytes;
use typst_pdf::PdfStandard;
use typst_syntax::package::PackageVersion;
use typst_syntax::{
    FileId, Lines, Source, VirtualPath, is_id_continue, is_ident, is_newline,
};
use unscanny::Scanner;

use crate::output::HashedRefs;
use crate::world::{read, system_path};
use crate::{ARGS, REF_PATH, STORE_PATH, SUITE_PATH};

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
    pub attrs: Attrs,
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

bitflags! {
    /// Just used for parsing attribute flags.
    #[derive(Copy, Clone)]
    struct AttrFlags: u16 {
        const LARGE = 1 << 0;
    }
}

/// The parsed and evaluated test attributes specified in the test header.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Attrs {
    pub large: bool,
    pub pdf_standard: Option<PdfStandard>,
    /// The test stages that are either directly specified or are implied by a
    /// test attribute. If not specified otherwise by the `--stages` flag a
    /// reference output will be generated.
    pub stages: TestStages,
}

impl Attrs {
    /// Whether the reference output should be compared and saved.
    pub fn should_check_ref(&self, output: TestOutput) -> bool {
        // TODO: Enable PDF and SVG once we have a diffing tool for hashed references.
        ARGS.should_run(self.stages & output.into())
            && output != TestOutput::Pdf
            && output != TestOutput::Svg
    }
}

pub trait TestStage: Into<TestStages> + Display + Copy {}

bitflags! {
    /// The stages a test in ran through. This combines both compilation targets
    /// and output formats.
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
    pub struct TestStages: u8 {
        const PAGED = 1 << 0;
        const RENDER = 1 << 1;
        const PDF = 1 << 2;
        const PDFTAGS = 1 << 3;
        const SVG = 1 << 4;
        const HTML = 1 << 5;
    }
}

macro_rules! union {
    ($union:expr) => {
        $union
    };
    ($a:expr, $b:expr $(,$flag:expr)*$(,)?) => {
        union!($a.union($b) $(,$flag)*)
    };
}

impl TestStages {
    /// All stages that require the paged target.
    pub const PAGED_STAGES: Self = union!(
        TestStages::PAGED,
        TestStages::RENDER,
        TestStages::PDF,
        TestStages::PDFTAGS,
        TestStages::SVG,
    );

    /// All stages that require a pdf document.
    pub const PDF_STAGES: Self = union!(TestStages::PDF, TestStages::PDFTAGS);

    /// The union the supplied stages and their implied stages.
    ///
    /// The `paged` target will test `render`, `pdf`, and `svg` by default.
    pub fn with_implied(&self) -> TestStages {
        let mut res = *self;
        for flag in self.iter() {
            res |= bitflags::bitflags_match!(flag, {
                TestStages::PAGED => TestStages::RENDER | TestStages::PDF | TestStages::SVG,
                TestStages::RENDER => TestStages::empty(),
                TestStages::PDF => TestStages::empty(),
                TestStages::PDFTAGS => TestStages::empty(),
                TestStages::SVG => TestStages::empty(),
                TestStages::HTML => TestStages::empty(),
                _ => unreachable!(),
            });
        }
        res
    }
}

/// A compilation target, analog to [`typst::Target`].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum TestTarget {
    Paged = TestStages::PAGED.bits(),
    Html = TestStages::HTML.bits(),
}

impl TestStage for TestTarget {}

impl From<TestTarget> for TestStages {
    fn from(value: TestTarget) -> Self {
        TestStages::from_bits(value as u8).unwrap()
    }
}

impl Display for TestTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TestTarget::Paged => "paged",
            TestTarget::Html => "html",
        })
    }
}

/// A test output format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum TestOutput {
    Render = TestStages::RENDER.bits(),
    Pdf = TestStages::PDF.bits(),
    Pdftags = TestStages::PDFTAGS.bits(),
    Svg = TestStages::SVG.bits(),
    Html = TestStages::HTML.bits(),
}

impl TestOutput {
    pub const ALL: [Self; 5] =
        [Self::Render, Self::Pdf, Self::Pdftags, Self::Svg, Self::Html];

    fn from_sub_dir(dir: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|o| o.sub_dir() == dir)
    }

    /// The sub directory inside the [`REF_PATH`] and [`STORE_PATH`].
    pub const fn sub_dir(&self) -> &'static str {
        match self {
            Self::Render => "render",
            Self::Pdf => "pdf",
            Self::Pdftags => "pdftags",
            Self::Svg => "svg",
            Self::Html => "html",
        }
    }

    /// The file extension used for live outputs and file references.
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Render => "png",
            Self::Pdf => "pdf",
            Self::Pdftags => "yml",
            Self::Svg => "svg",
            Self::Html => "html",
        }
    }

    /// The path at which the live output will be stored for inspection.
    pub fn live_path(&self, name: &str) -> PathBuf {
        let dir = self.sub_dir();
        let ext = self.extension();
        PathBuf::from(format!("{STORE_PATH}/{dir}/{name}.{ext}"))
    }

    /// The path at which file references will be saved.
    pub fn file_ref_path(&self, name: &str) -> PathBuf {
        let dir = self.sub_dir();
        let ext = self.extension();
        PathBuf::from(format!("{REF_PATH}/{dir}/{name}.{ext}"))
    }

    /// The path at which hashed references will be saved.
    pub fn hashed_ref_path(self, source_path: &Path) -> PathBuf {
        let sub_dir = self.sub_dir();
        let sub_path = source_path.strip_prefix(SUITE_PATH).unwrap();
        let trimmed_path = sub_path.to_str().unwrap().strip_suffix(".typ");
        let file_name = trimmed_path.unwrap().replace("/", "-");
        PathBuf::from(format!("{REF_PATH}/{sub_dir}/{file_name}.txt"))
    }

    /// The output kind.
    fn kind(&self) -> TestOutputKind {
        match self {
            TestOutput::Render | TestOutput::Pdftags | TestOutput::Html => {
                TestOutputKind::File
            }
            TestOutput::Pdf | TestOutput::Svg => TestOutputKind::Hash,
        }
    }
}

impl TestStage for TestOutput {}

impl From<TestOutput> for TestStages {
    fn from(value: TestOutput) -> Self {
        TestStages::from_bits(value as u8).unwrap()
    }
}

impl Display for TestOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.sub_dir())
    }
}

/// Whether the output format produces hashed or file references.
enum TestOutputKind {
    Hash,
    File,
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
    seen: FxHashMap<EcoString, (FilePos, Attrs)>,
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

    /// Walks through all reference outputs and ensures that a matching test
    /// exists.
    fn walk_references(&mut self) {
        for entry in walkdir::WalkDir::new(crate::REF_PATH).sort_by_file_name() {
            let entry = entry.unwrap();
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let sub_path = path.strip_prefix(crate::REF_PATH).unwrap();
            let sub_dir = sub_path.components().next().unwrap();
            let Some(output) =
                TestOutput::from_sub_dir(sub_dir.as_os_str().to_str().unwrap())
            else {
                continue;
            };

            match output.kind() {
                TestOutputKind::File => self.check_dangling_file_references(path, output),
                TestOutputKind::Hash => {
                    self.check_dangling_hashed_references(path, output)
                }
            }
        }
    }

    fn check_dangling_file_references(&mut self, path: &Path, output: TestOutput) {
        let stem = path.file_stem().unwrap().to_string_lossy();
        let name = &*stem;

        let Some((pos, attrs)) = self.seen.get(name) else {
            self.errors.push(TestParseError {
                pos: FilePos::new(path, 0),
                message: "dangling reference output".into(),
            });
            return;
        };

        if !attrs.stages.contains(output.into()) {
            self.errors.push(TestParseError {
                pos: FilePos::new(path, 0),
                message: "dangling reference output".into(),
            });
        }

        let len = path.metadata().unwrap().len() as usize;
        if !attrs.large && len > crate::REF_LIMIT {
            self.errors.push(TestParseError {
                pos: pos.clone(),
                message: format!(
                    "reference output size exceeds {}, but the test is not marked as `large`",
                    FileSize(crate::REF_LIMIT),
                ),
            });
        }
    }

    fn check_dangling_hashed_references(&mut self, path: &Path, output: TestOutput) {
        let string = std::fs::read_to_string(path).unwrap_or_default();
        let Ok(hashed_refs) = HashedRefs::from_str(&string) else { return };
        if hashed_refs.is_empty() {
            self.errors.push(TestParseError {
                pos: FilePos::new(path, 0),
                message: "dangling empty reference hash file".into(),
            });
        }

        let mut right_file = 0;
        let mut wrong_file = Vec::new();
        for (line, name) in hashed_refs.names().enumerate() {
            let Some((pos, attrs)) = self.seen.get(name) else {
                self.errors.push(TestParseError {
                    pos: FilePos::new(path, line),
                    message: format!("dangling reference hash ({name})"),
                });
                continue;
            };

            if !attrs.stages.contains(output.into()) {
                self.errors.push(TestParseError {
                    pos: FilePos::new(path, line),
                    message: format!("dangling reference hash ({name})"),
                });
                continue;
            }

            if output.hashed_ref_path(&pos.path) == path {
                right_file += 1;
            } else {
                wrong_file.push((line, name));
            }
        }

        if !wrong_file.is_empty() {
            if right_file == 0 {
                self.errors.push(TestParseError {
                    pos: FilePos::new(path, 0),
                    message: "dangling reference hash file".into(),
                });
            } else {
                for (line, name) in wrong_file {
                    self.errors.push(TestParseError {
                        pos: FilePos::new(path, line),
                        message: format!("dangling reference hash ({name})"),
                    });
                }
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
            let mut attrs = Attrs::default();
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
            self.collector.seen.insert(name.clone(), (pos.clone(), attrs));

            while !self.s.done() && !self.s.at("---") {
                self.s.eat_until(is_newline);
                if self.s.eat_newline() {
                    self.line += 1;
                }
            }

            let text = self.s.from(start);

            if !ARGS.should_run(attrs.stages)
                || !selected(&name, self.path.canonicalize().unwrap())
            {
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

    /// Parse the test attributes inside a test header.
    fn parse_attrs(&mut self) -> Attrs {
        let mut stages = TestStages::empty();
        let mut flags = AttrFlags::empty();
        let mut pdf_standard = None;
        while !self.s.eat_if("---") {
            let attr_name = self.s.eat_while(is_id_continue);
            let mut attr_params = None;
            if self.s.eat_if('(') {
                attr_params = Some(self.s.eat_until(')'));
                if !self.s.eat_if(')') {
                    self.error("expected closing parenthesis");
                }
            }
            if !self.s.at(' ') {
                self.error("expected a space after an attribute");
            }

            match attr_name {
                "paged" => self.set_attr(attr_name, &mut stages, TestStages::PAGED),
                "pdftags" => self.set_attr(attr_name, &mut stages, TestStages::PDFTAGS),
                "pdfstandard" => {
                    let Some(param) = attr_params.take() else {
                        self.error("expected parameter for `pdfstandard`");
                        continue;
                    };
                    pdf_standard = serde_yaml::from_str(param)
                        .inspect_err(|e| {
                            self.error(format!("unknown pdf standard `{param}`: {e}"))
                        })
                        .ok();
                }
                "html" => self.set_attr(attr_name, &mut stages, TestStages::HTML),
                "large" => self.set_attr(attr_name, &mut flags, AttrFlags::LARGE),

                found => {
                    self.error(format!(
                        "expected attribute or closing ---, found `{found}`"
                    ));
                    break;
                }
            }

            if attr_params.is_some() {
                self.error("unexpected attribute parameters");
            }

            self.s.eat_while(' ');
        }

        if stages.is_empty() {
            self.error("tests must specify at least one target or output");
        }

        Attrs {
            large: flags.contains(AttrFlags::LARGE),
            pdf_standard,
            stages: stages.with_implied(),
        }
    }

    /// Set an attribute flag and check for duplicates.
    fn set_attr<F: Flags + Copy>(&mut self, attr: &str, flags: &mut F, flag: F) {
        if flags.contains(flag) {
            self.error(format!("duplicate attribute `{attr}`"));
        }
        flags.insert(flag);
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
            .replace("VERSION", &eco_format!("{}", PackageVersion::compiler()))
            .replace("\\n", "\n");

        Some(Note {
            pos: FilePos::new(self.path, self.line),
            kind,
            file: file.unwrap_or(source.id()),
            range,
            message,
        })
    }

    /// Parse a range in an external file, optionally abbreviated as just a position
    /// if the range is empty.
    fn parse_range_external(&mut self, file: FileId) -> Option<Range<usize>> {
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
