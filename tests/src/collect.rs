use std::fmt::{self, Display, Formatter};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, LazyLock};

use bitflags::{Flags, bitflags};
use ecow::EcoString;
use rustc_hash::{FxHashMap, FxHashSet};
use typst_pdf::PdfStandard;
use typst_syntax::{is_id_continue, is_ident, is_newline};
use unscanny::Scanner;

use crate::notes::{TestBody, parse_test_body};
use crate::output::{self, HashOutputType, HashedRef, HashedRefs};
use crate::{ARGS, REF_PATH, STORE_PATH, SUITE_PATH};

/// Collects all tests from all files.
///
/// Returns:
/// - the tests and the number of skipped tests in the success case.
/// - parsing errors in the failure case.
pub fn collect() -> Result<([HashedRefs; 2], Vec<Test>, usize), Vec<TestParseError>> {
    Collector::new().collect()
}

/// A single test.
pub struct Test {
    pub name: EcoString,
    pub attrs: Attrs,
    pub body: TestBody,
}

impl Test {
    /// Whether this test output should be compared and saved, this is true for
    /// stages that are explicitly specified and those that are
    /// [implied](TestStages::with_implied).
    pub fn should_check(&self, output: TestOutput) -> bool {
        ARGS.required_stages()
            .intersects(self.attrs.implied_stages() & output.into())
    }

    /// Whether this test stage should be run, test stages that are
    /// [required](TestStages::with_required) by another stage mus be run, even
    /// if they aren't explicitly specified.
    pub fn should_run(&self, stage: impl TestStage) -> bool {
        ARGS.required_stages()
            .intersects(self.attrs.implied_stages().with_required() & stage.into())
    }
}

impl Display for Test {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // underline path
        if std::io::stdout().is_terminal() {
            write!(f, "{} (\x1B[4m{}\x1B[0m)", self.name, self.body.pos)
        } else {
            write!(f, "{} ({})", self.name, self.body.pos)
        }
    }
}

/// A position in a file. This allows us to print errors that point to specific
/// line numbers and create a clickable link in editors like VSCode.
#[derive(Clone)]
pub struct FilePos {
    pub path: Arc<PathBuf>,
    /// Line numbers are 1-indexed, so if this is zero we treat it as pointing
    /// to the file as a whole.
    pub line: usize,
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
        const EMPTY = 1 << 1;
    }
}

/// The parsed and evaluated test attributes specified in the test header.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Attrs {
    pub large: bool,
    pub empty: bool,
    pub pdf_standard: Option<PdfStandard>,
    /// The test stages that are either directly specified or are implied by a
    /// test attribute. If not specified otherwise by the `--stages` flag a
    /// reference output will be generated.
    stages: TestStages,
}

impl Attrs {
    /// The stages, the way they were parsed.
    pub fn parsed_stages(&self) -> TestStages {
        self.stages
    }

    /// The stages that were parsed and the ones that are implied.
    pub fn implied_stages(&self) -> TestStages {
        self.stages.with_implied()
    }
}

pub trait TestStage: Into<TestStages> + Display + Copy {}

bitflags! {
    /// The stages a test in ran through. This combines both compilation targets
    /// and output formats.
    ///
    /// Here's a visual representation of the stage tree:
    /// ```txt
    ///                  ╭─> render
    ///       ╭─> paged ─┼─> pdf ───> pdftags
    /// eval ─┤          ╰─> svg
    ///       ╰─> html  ───> html
    /// ```
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
    pub struct TestStages: u8 {
        const EVAL = 1 << 0;
        const PAGED = 1 << 1;
        const RENDER = 1 << 2;
        const PDF = 1 << 3;
        const PDFTAGS = 1 << 4;
        const SVG = 1 << 5;
        const HTML = 1 << 6;
    }
}

impl TestStages {
    /// The union of the supplied stages and their implied stages.
    ///
    /// The `paged` target will test `render`, `pdf`, and `svg` by default.
    pub fn with_implied(&self) -> TestStages {
        let mut res = *self;
        for flag in self.iter() {
            res |= bitflags::bitflags_match!(flag, {
                TestStages::EVAL => TestStages::empty(),
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

    /// The union of the supplied stages and their required stages.
    ///
    /// For example, the `pdf` output requires the `paged` target.
    /// And the `pdftags` output requires both `pdf` and `paged`.
    pub fn with_required(&self) -> TestStages {
        let mut res = *self;
        for flag in self.iter() {
            res |= bitflags::bitflags_match!(flag, {
                TestStages::EVAL => TestStages::empty(),
                TestStages::PAGED => TestStages::EVAL,
                TestStages::RENDER => TestStages::EVAL | TestStages::PAGED,
                TestStages::PDF => TestStages::EVAL | TestStages::PAGED,
                TestStages::PDFTAGS => TestStages::EVAL | TestStages::PAGED | TestStages::PDF,
                TestStages::SVG => TestStages::EVAL | TestStages::PAGED,
                TestStages::HTML => TestStages::EVAL,
                _ => unreachable!(),
            });
        }
        res
    }

    /// The union of the supplied stages and their sibling stages.
    ///
    /// See the tree in [`TestStages`].
    pub fn with_siblings(&self) -> TestStages {
        let mut res = *self;
        for flag in self.iter() {
            res |= bitflags::bitflags_match!(flag, {
                TestStages::PAGED => TestStages::PAGED | TestStages::HTML,
                TestStages::HTML => TestStages::PAGED | TestStages::HTML,

                TestStages::RENDER => TestStages::RENDER | TestStages::PDF | TestStages::SVG,
                TestStages::PDF => TestStages::RENDER | TestStages::PDF | TestStages::SVG,
                TestStages::SVG => TestStages::RENDER | TestStages::PDF | TestStages::SVG,

                TestStages::PDFTAGS => TestStages::PDFTAGS,
                _ => unreachable!("{flag}"),
            });
        }
        res
    }
}

impl Display for TestStages {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (i, flag) in self.iter().enumerate() {
            if i != 0 {
                f.write_str(", ")?;
            }
            bitflags::bitflags_match!(flag, {
                TestStages::EVAL => Display::fmt(&TestEval, f),
                TestStages::PAGED => Display::fmt(&TestTarget::Paged, f),
                TestStages::RENDER => Display::fmt(&TestOutput::Render, f),
                TestStages::PDF => Display::fmt(&TestOutput::Pdf, f),
                TestStages::PDFTAGS => Display::fmt(&TestOutput::Pdftags, f),
                TestStages::SVG => Display::fmt(&TestOutput::Svg, f),
                TestStages::HTML => Display::fmt(&TestTarget::Html, f),
                _ => unreachable!(),
            })?;
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TestEval;

impl TestStage for TestEval {}

impl From<TestEval> for TestStages {
    fn from(_: TestEval) -> Self {
        TestStages::EVAL
    }
}

impl Display for TestEval {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("eval")
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

impl FromStr for TestTarget {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "paged" => Ok(Self::Paged),
            "html" => Ok(Self::Html),
            _ => Err(()),
        }
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
        [Self::Render, Self::Svg, Self::Pdf, Self::Pdftags, Self::Html];

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

    /// The file extension used for live output and file references.
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Render => "png",
            Self::Pdf => "pdf",
            Self::Pdftags => "yml",
            Self::Svg => "svg",
            Self::Html => "html",
        }
    }

    /// The path at which the live output will be stored.
    pub fn hash_path(&self, hash: HashedRef, name: &str) -> PathBuf {
        let ext = self.extension();
        PathBuf::from(format!("{STORE_PATH}/by-hash/{hash}_{name}.{ext}"))
    }

    /// The path at which a symlink to the [`Self::hash_path`] will be created
    /// for inspection.
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
    pub fn hash_refs_path(self) -> PathBuf {
        let dir = self.sub_dir();
        PathBuf::from(format!("{REF_PATH}/{dir}/hashes.txt"))
    }

    /// The output kind.
    pub fn kind(&self) -> TestOutputKind {
        match self {
            TestOutput::Render | TestOutput::Pdftags | TestOutput::Html => {
                TestOutputKind::File
            }
            TestOutput::Pdf => TestOutputKind::Hash(output::Pdf::INDEX),
            TestOutput::Svg => TestOutputKind::Hash(output::Svg::INDEX),
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
pub enum TestOutputKind {
    Hash(usize),
    File,
}

/// The size of a file.
pub struct FileSize(pub usize);

impl Display for FileSize {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:.2} KiB", (self.0 as f64) / 1024.0)
    }
}

/// Collects all tests from all files.
struct Collector {
    hashes: [HashedRefs; 2],
    tests: Vec<Test>,
    errors: Vec<TestParseError>,
    seen: FxHashMap<EcoString, (FilePos, Attrs)>,
    skipped: usize,
}

impl Collector {
    /// Creates a new test collector.
    fn new() -> Self {
        Self {
            hashes: std::array::from_fn(|_| HashedRefs::default()),
            tests: vec![],
            errors: vec![],
            seen: FxHashMap::default(),
            skipped: 0,
        }
    }

    /// Collects tests from all files.
    fn collect(
        mut self,
    ) -> Result<([HashedRefs; 2], Vec<Test>, usize), Vec<TestParseError>> {
        self.walk_files();
        self.walk_references();

        if self.errors.is_empty() {
            Ok((self.hashes, self.tests, self.skipped))
        } else {
            Err(self.errors)
        }
    }

    /// Walks through all test files and collects the tests.
    fn walk_files(&mut self) {
        for entry in walkdir::WalkDir::new(SUITE_PATH).sort_by_file_name() {
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
        for entry in walkdir::WalkDir::new(REF_PATH).sort_by_file_name() {
            let entry = entry.unwrap();
            if entry.file_type().is_dir() {
                continue;
            }

            let path = entry.path();
            let output = (path.strip_prefix(REF_PATH).ok())
                .and_then(|sub_path| sub_path.components().next())
                .and_then(|sub_dir| sub_dir.as_os_str().to_str())
                .and_then(TestOutput::from_sub_dir);
            let Some(output) = output else { continue };

            match output.kind() {
                TestOutputKind::File => self.check_dangling_file_references(path, output),
                TestOutputKind::Hash(idx) => {
                    if let Some(hashed_refs) =
                        self.check_dangling_hashed_references(path, output)
                    {
                        self.hashes[idx] = hashed_refs;
                    }
                }
            }
        }
    }

    fn check_dangling_file_references(&mut self, path: &Path, output: TestOutput) {
        let stem = path.file_stem().unwrap().to_string_lossy();
        let name = &*stem;

        let Some((pos, attrs)) = self.seen.get(name) else {
            self.errors.push(TestParseError::new(
                TestParseErrorKind::DanglingFile,
                path,
                0,
            ));
            return;
        };

        if !attrs.implied_stages().contains(output.into()) || attrs.empty {
            self.errors.push(TestParseError::new(
                TestParseErrorKind::DanglingFile,
                path,
                0,
            ));
        }

        let len = path.metadata().unwrap().len() as usize;
        if !attrs.large && len > crate::REF_LIMIT {
            let message = format!(
                "reference output size exceeds {}, but the test is not marked as `large`",
                FileSize(crate::REF_LIMIT),
            );
            self.errors
                .push(TestParseError { pos: pos.clone(), kind: message.into() });
        }
    }

    fn check_dangling_hashed_references(
        &mut self,
        path: &Path,
        output: TestOutput,
    ) -> Option<HashedRefs> {
        let path = path.to_str().unwrap().replace('\\', "/");
        let path = Path::new(&path);

        if output.hash_refs_path() != path {
            self.errors.push(TestParseError::new(
                TestParseErrorKind::DanglingFile,
                path,
                0,
            ));
            return None;
        }

        let string = std::fs::read_to_string(path).unwrap_or_default();
        let hashed_refs = HashedRefs::from_str(&string)
            .inspect_err(|err| {
                self.errors.push(TestParseError::new(
                    format!("error parsing reference hash file: {err}"),
                    path,
                    0,
                ));
            })
            .ok()?;

        for (name, line) in hashed_refs.names().zip(1..) {
            let Some((_, attrs)) = self.seen.get(name) else {
                self.errors.push(TestParseError::new(
                    TestParseErrorKind::DanglingHash(name.clone()),
                    path,
                    line,
                ));
                continue;
            };

            if !attrs.implied_stages().contains(output.into()) || attrs.empty {
                self.errors.push(TestParseError::new(
                    TestParseErrorKind::DanglingHash(name.clone()),
                    path,
                    line,
                ));
                continue;
            }
        }

        Some(hashed_refs)
    }
}

/// Parses a single test file.
struct Parser<'a> {
    collector: &'a mut Collector,
    path: Arc<PathBuf>,
    s: Scanner<'a>,
    test_start_line: usize,
    line: usize,
}

impl<'a> Parser<'a> {
    /// Creates a new parser for a file.
    fn new(collector: &'a mut Collector, path: &'a Path, source: &'a str) -> Self {
        Self {
            collector,
            path: Arc::new(path.to_owned()),
            s: Scanner::new(source),
            // Lines in files are 1-indexed.
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

            let pos = FilePos {
                path: self.path.clone(),
                line: self.test_start_line,
            };
            self.collector.seen.insert(name.clone(), (pos.clone(), attrs));

            while !self.s.done() && !self.s.at("---") {
                self.s.eat_until(is_newline);
                if self.s.eat_newline() {
                    self.line += 1;
                }
            }

            if !ARGS.implied_stages().intersects(attrs.implied_stages())
                || !selected(&name, self.path.canonicalize().unwrap())
            {
                self.collector.skipped += 1;
                continue;
            }

            let body = self.s.from(start);
            let body = parse_test_body(pos, body, &mut self.collector.errors);

            self.collector.tests.push(Test { name, attrs, body });
        }
    }

    /// Skips the preamble of a test file.
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
                "eval" => self.set_attr(attr_name, &mut stages, TestStages::EVAL),
                "paged" => self.set_attr(attr_name, &mut stages, TestStages::PAGED),
                "pdf" => self.set_attr(attr_name, &mut stages, TestStages::PDF),
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
                "empty" => self.set_attr(attr_name, &mut flags, AttrFlags::EMPTY),

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

        if stages.contains(TestStages::EVAL) {
            let others = stages.difference(TestStages::EVAL);
            if !others.is_empty() {
                self.error(format!(
                    "`eval` must be the only test stage, consider removing [{others}]"
                ));
            } else if flags.contains(AttrFlags::EMPTY) {
                self.error("specifying `empty` on an `eval` test is redundant");
            }
        }

        Attrs {
            large: flags.contains(AttrFlags::LARGE),
            empty: flags.contains(AttrFlags::EMPTY),
            pdf_standard,
            stages,
        }
    }

    /// Set an attribute flag and check for duplicates.
    fn set_attr<F: Flags + Copy>(&mut self, attr: &str, flags: &mut F, flag: F) {
        if flags.contains(flag) {
            self.error(format!("duplicate attribute `{attr}`"));
        }
        flags.insert(flag);
    }

    /// Stores a test parsing error.
    fn error(&mut self, message: impl Into<String>) {
        self.collector
            .errors
            .push(TestParseError::new(message, &self.path, self.line));
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

    let paths = &ARGS.path;
    if !paths.is_empty() && !paths.iter().any(|path| abs.starts_with(path)) {
        return false;
    }

    let exact = ARGS.exact;
    let patterns = &ARGS.pattern;
    patterns.is_empty()
        || patterns.iter().any(|pattern: &regex::Regex| {
            if exact { name == pattern.as_str() } else { pattern.is_match(name) }
        })
}

/// An error in a test file.
pub struct TestParseError {
    pub pos: FilePos,
    pub kind: TestParseErrorKind,
}

impl TestParseError {
    pub fn new(kind: impl Into<TestParseErrorKind>, path: &Path, line: usize) -> Self {
        Self {
            pos: FilePos { path: Arc::new(path.to_owned()), line },
            kind: kind.into(),
        }
    }
}

impl Display for TestParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.kind, self.pos)
    }
}

/// The kind of error that occurred when collecting tests.
pub enum TestParseErrorKind {
    DanglingFile,
    DanglingHash(EcoString),
    Other(String),
}

impl<S: Into<String>> From<S> for TestParseErrorKind {
    fn from(v: S) -> Self {
        Self::Other(v.into())
    }
}

impl Display for TestParseErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            TestParseErrorKind::DanglingFile => f.write_str("dangling reference file"),
            TestParseErrorKind::DanglingHash(name) => {
                write!(f, "dangling reference hash ({name})")
            }
            TestParseErrorKind::Other(message) => f.write_str(message),
        }
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
