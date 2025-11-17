use std::path::Path;

use ecow::{EcoString, eco_format};
use similar::{ChangeTag, InlineChange, TextDiff};
use smallvec::SmallVec;
use typst::diag::{FileError, FileResult, Severity, SourceDiagnostic, Warned};
use typst::foundations::{Bytes, Cast, Datetime, Dict, IntoValue, Value, dict};
use typst::text::{Font, FontBook};
use typst::{Feature, Features, Library, LibraryExt, World};
use typst_html::HtmlDocument;
use typst_syntax::{FileId, Source, Span, Spanned, VirtualPath};
use typst_utils::LazyHash;

static DIFF_STYLE: &str = include_str!("report.css");
static REPORT_TEMPLATE: &str = include_str!("report.typ");

const ANSII_RED: &str = "\x1b[91m";
const ANSII_YELLOW: &str = "\x1b[93m";
const ANSII_BLUE: &str = "\x1b[34m";
const ANSII_CLEAR: &str = "\x1b[0m";

struct ReportWorld {
    source: Source,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

impl ReportWorld {
    pub fn new(source: Source, inputs: Dict) -> Self {
        let library = Library::builder()
            .with_features(Features::from_iter([Feature::Html]))
            .with_inputs(inputs)
            .build();

        // For HTML fonts aren't required.
        let book = FontBook::new();
        let fonts = Vec::new();
        Self {
            source,
            library: LazyHash::new(library),
            book: LazyHash::new(book),
            fonts,
        }
    }
}

impl World for ReportWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::Other(Some("Opening other files is not supported".into())))
        }
    }

    fn file(&self, _: FileId) -> FileResult<Bytes> {
        Err(FileError::Other(Some("Opening other files is not supported".into())))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _: Option<i64>) -> Option<Datetime> {
        None
    }
}

pub struct TextFileDiff {
    name: EcoString,
    left: Lines,
    right: Lines,
}

impl IntoValue for TextFileDiff {
    fn into_value(self) -> typst::foundations::Value {
        Value::Dict(dict! {
            "name" => self.name.into_value(),
            "left" => self.left.into_value(),
            "right" => self.right.into_value(),
        })
    }
}

pub struct Lines {
    path: EcoString,
    lines: Vec<Line>,
}

impl Lines {
    pub fn new(path: &Path, lines: Vec<Line>) -> Self {
        let path = eco_format!("{}", path.display());
        Self { path, lines }
    }
}

impl IntoValue for Lines {
    fn into_value(self) -> typst::foundations::Value {
        Value::Dict(dict! {
            "path" => self.path.into_value(),
            "lines" => self.lines.into_value(),
        })
    }
}

#[derive(Copy, Clone, Cast)]
pub enum Kind {
    Empty,
    Del,
    Add,
    Unchanged,
    Gap,
}

pub struct Line {
    kind: Kind,
    nr: u32,
    spans: SmallVec<[TextSpan; 3]>,
}

impl IntoValue for Line {
    fn into_value(self) -> typst::foundations::Value {
        Value::Dict(dict! {
            "kind" => self.kind,
            "nr" => self.nr,
            "spans" => self.spans,
        })
    }
}

pub struct TextSpan {
    emph: bool,
    text: EcoString,
}

impl IntoValue for TextSpan {
    fn into_value(self) -> typst::foundations::Value {
        Value::Dict(dict! {
            "emph" => self.emph,
            "text" => self.text,
        })
    }
}

pub fn generate(mut diffs: Vec<TextFileDiff>) -> Option<String> {
    diffs.sort_by(|a, b| a.name.cmp(&b.name));

    let inputs = dict! {
        "diffs" => diffs.into_value(),
        "style" => DIFF_STYLE.into_value(),
    };
    let vpath = VirtualPath::new("report.typ");
    let source = Source::new(FileId::new(None, vpath), REPORT_TEMPLATE.into());
    let world = ReportWorld::new(source, inputs);
    let Warned { output, warnings } = typst::compile::<HtmlDocument>(&world);
    print_diagnostics(&world, &warnings);

    let doc = output.inspect_err(|errors| print_diagnostics(&world, errors)).ok()?;
    typst_html::html(&doc)
        .inspect_err(|errors| print_diagnostics(&world, errors))
        .ok()
}

fn print_diagnostics(world: &ReportWorld, diags: &[SourceDiagnostic]) {
    for diag in diags {
        let SourceDiagnostic { severity, span, message, trace: _, hints } = diag;
        if diag.message == "html export is under active development and incomplete" {
            continue;
        }

        let severity = typst_utils::display(|f| match severity {
            Severity::Error => write!(f, "{ANSII_RED}error{ANSII_CLEAR}"),
            Severity::Warning => write!(f, "{ANSII_YELLOW}warning{ANSII_CLEAR}"),
        });
        eprintln!("{severity}: {message}");

        print_code_lines(world, *span);
        for Spanned { v: message, span } in hints {
            eprintln!("{ANSII_BLUE}hint{ANSII_CLEAR}: {message}");
            if !span.is_detached() {
                print_code_lines(world, *span);
            }
        }
        eprintln!();
    }
}

fn print_code_lines(world: &ReportWorld, span: Span) {
    let lines = world.source.lines();
    if let Some(range) = span.range().or_else(|| world.source.range(span)) {
        let (line_idx, col_idx) = lines.byte_to_line_column(range.start).unwrap();
        let end_line_idx = lines.byte_to_line(range.start).unwrap();

        let line_nr = line_idx + 1;
        let col_nr = col_idx + 1;
        eprintln!(
            "     {ANSII_BLUE}┌─{ANSII_CLEAR} tests/src/report.typ:{line_nr}:{col_nr}"
        );
        for line_idx in line_idx..=end_line_idx {
            let line_range = lines.line_to_range(line_idx).unwrap();
            let line = &lines.text()[line_range].trim_end();
            let line_nr = line_idx + 1;
            eprintln!("{ANSII_BLUE}{line_nr:>4} │{ANSII_CLEAR} {line}");
        }
        eprintln!("     {ANSII_BLUE}│{ANSII_CLEAR}");
    }
}

/// Create a rich HTML text diff.
pub fn text_diff(
    name: EcoString,
    (path_a, a): (&Path, &str),
    (path_b, b): (&Path, &str),
) -> TextFileDiff {
    let diff = TextDiff::from_lines(a, b);

    let mut left = Vec::new();
    let mut right = Vec::new();

    for (i, group) in diff.grouped_ops(3).iter().enumerate() {
        if i != 0 {
            left.push(line_gap());
            right.push(line_gap());
        }

        for op in group.iter() {
            for change in diff.iter_inline_changes(op) {
                match change.tag() {
                    ChangeTag::Equal => {
                        while left.len() < right.len() {
                            left.push(line_empty());
                        }
                        while right.len() < left.len() {
                            right.push(line_empty());
                        }

                        let left_line_nr = change.old_index().unwrap();
                        left.push(line_unchanged(left_line_nr, &change));

                        let right_line_nr = change.new_index().unwrap();
                        right.push(line_unchanged(right_line_nr, &change));
                    }
                    ChangeTag::Delete => {
                        let left_line_nr = change.old_index().unwrap();
                        left.push(line_del(left_line_nr, &change));
                    }
                    ChangeTag::Insert => {
                        let right_line_nr = change.new_index().unwrap();
                        right.push(line_add(right_line_nr, &change));
                    }
                }
            }
        }

        while left.len() < right.len() {
            left.push(line_empty());
        }
        while right.len() < left.len() {
            right.push(line_empty());
        }
    }

    TextFileDiff {
        name,
        left: Lines::new(path_a, left),
        right: Lines::new(path_b, right),
    }
}

fn line_empty() -> Line {
    Line { kind: Kind::Empty, nr: 0, spans: SmallVec::new() }
}

fn line_del(line_nr: usize, change: &InlineChange<str>) -> Line {
    diff_line(Kind::Del, line_nr, change)
}

fn line_add(line_nr: usize, change: &InlineChange<str>) -> Line {
    diff_line(Kind::Add, line_nr, change)
}

fn line_unchanged(line_nr: usize, change: &InlineChange<str>) -> Line {
    diff_line(Kind::Unchanged, line_nr, change)
}

fn line_gap() -> Line {
    Line { kind: Kind::Gap, nr: 0, spans: SmallVec::new() }
}

fn diff_line(kind: Kind, nr: usize, change: &InlineChange<str>) -> Line {
    let spans = line_spans(change);
    Line { kind, nr: nr as u32, spans }
}

fn line_spans(change: &InlineChange<str>) -> SmallVec<[TextSpan; 3]> {
    change
        .iter_strings_lossy()
        .map(|(emph, span)| {
            let span = span.trim_end_matches('\n');
            TextSpan { emph, text: span.into() }
        })
        .collect()
}
