use std::fmt::Display;
use std::time::Duration;

use base64::Engine;
use ecow::EcoString;
use similar::{ChangeTag, InlineChange, TextDiff};
use smallvec::SmallVec;

use crate::collect::TestOutput;
use crate::output::HashedRef;

/// The diffs generated for a specific [`TestOutput`].
pub struct ReportFile {
    pub output: TestOutput,
    pub left: Option<File>,
    pub right: Option<File>,
    pub diffs: SmallVec<[DiffKind; 2]>,
}

impl ReportFile {
    pub fn new(
        output: TestOutput,
        old: Option<File>,
        new: Option<File>,
        diffs: impl IntoIterator<Item = DiffKind>,
    ) -> Self {
        Self {
            output,
            left: old,
            right: new,
            diffs: diffs.into_iter().collect(),
        }
    }
}

/// A file path and its size.
pub struct File {
    pub path: EcoString,
    /// The size of the file if it exists.
    pub size: Option<usize>,
}

/// A text or image diff.
pub enum DiffKind {
    Text(FileDiff<Lines>),
    Image(FileDiff<Image>),
}

impl DiffKind {
    pub fn missing_old(&self) -> Option<HashedRef> {
        match self {
            DiffKind::Text(diff) => diff.left().and_then(|old| old.missing()),
            DiffKind::Image(diff) => diff.left().and_then(|old| old.missing()),
        }
    }

    pub fn kind_str(&self) -> &'static str {
        match self {
            DiffKind::Text(_) => "text",
            DiffKind::Image(_) => "image",
        }
    }
}

/// A generic file diff.
pub enum FileDiff<T> {
    /// There is a diff.
    Diff(Old<T>, Result<T, ()>),
    /// There is new test output.
    Right(Result<T, ()>),
}

impl<T> FileDiff<T> {
    pub fn left(&self) -> Option<&Old<T>> {
        match self {
            FileDiff::Diff(l, _) => Some(l),
            FileDiff::Right(_) => None,
        }
    }

    pub fn right(&self) -> Option<&Result<T, ()>> {
        match self {
            FileDiff::Diff(_, r) => Some(r),
            FileDiff::Right(r) => Some(r),
        }
    }
}

/// Old reference data for a hashed reference. Contains either the data, if the
/// file is present, or the hash if it is missing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Old<T> {
    /// The expected reference data.
    Data(T),
    /// The live output for the hashed reference is missing.
    Missing(HashedRef),
}

impl<T> Old<T> {
    pub fn data(&self) -> Option<&T> {
        match self {
            Old::Data(d) => Some(d),
            Old::Missing(_) => None,
        }
    }

    pub fn missing(&self) -> Option<HashedRef> {
        match self {
            Old::Data(_) => None,
            Old::Missing(h) => Some(*h),
        }
    }

    pub fn map<V, F>(self, f: F) -> Old<V>
    where
        F: FnOnce(T) -> V,
    {
        match self {
            Old::Data(d) => Old::Data(f(d)),
            Old::Missing(h) => Old::Missing(h),
        }
    }

    pub fn as_ref(&self) -> Old<&T> {
        match self {
            Old::Data(d) => Old::Data(d),
            Old::Missing(h) => Old::Missing(*h),
        }
    }
}

pub struct Lines {
    pub lines: Vec<Line>,
}

impl Lines {
    pub fn new(lines: Vec<Line>) -> Self {
        Self { lines }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum LineKind {
    Empty,
    Del,
    Add,
    Unchanged,
    Gap,
    End,
}

impl Display for LineKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            LineKind::Empty => "empty",
            LineKind::Del => "del",
            LineKind::Add => "add",
            LineKind::Unchanged => "unchanged",
            LineKind::Gap => "gap",
            LineKind::End => "end",
        })
    }
}

pub struct Line {
    pub kind: LineKind,
    pub nr: u32,
    pub spans: SmallVec<[TextSpan; 3]>,
}

impl Line {
    pub const EMPTY: Line = Line {
        kind: LineKind::Empty,
        nr: 0,
        spans: SmallVec::new_const(),
    };
}

pub struct TextSpan {
    pub emph: bool,
    pub text: EcoString,
}

/// Create a rich HTML text diff.
pub fn text_diff(a: Option<Old<&str>>, b: Result<&str, ()>) -> FileDiff<Lines> {
    let lines = |kind| move |str| file_lines(str, kind);

    let (a, b) = match (a, b) {
        (Some(Old::Data(a)), Ok(b)) => (a, b),
        (Some(a), b) => {
            return FileDiff::Diff(
                a.map(lines(LineKind::Unchanged)),
                b.map(lines(LineKind::Unchanged)),
            );
        }
        (None, b) => return FileDiff::Right(b.map(lines(LineKind::Add))),
    };

    let diff = TextDiff::configure()
        .timeout(Duration::from_millis(500))
        .diff_lines(a, b);

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
                            left.push(Line::EMPTY);
                        }
                        while right.len() < left.len() {
                            right.push(Line::EMPTY);
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
            left.push(Line::EMPTY);
        }
        while right.len() < left.len() {
            right.push(Line::EMPTY);
        }
    }

    FileDiff::Diff(Old::Data(Lines::new(left)), Ok(Lines::new(right)))
}

fn file_lines(text: &str, kind: LineKind) -> Lines {
    let lines = text
        .lines()
        .zip(1..)
        .map(|(line, nr)| Line {
            kind,
            nr,
            spans: SmallVec::from_iter([TextSpan { emph: false, text: line.into() }]),
        })
        .collect();
    Lines { lines }
}

fn line_del(line_nr: usize, change: &InlineChange<str>) -> Line {
    diff_line(LineKind::Del, line_nr, change)
}

fn line_add(line_nr: usize, change: &InlineChange<str>) -> Line {
    diff_line(LineKind::Add, line_nr, change)
}

fn line_unchanged(line_nr: usize, change: &InlineChange<str>) -> Line {
    diff_line(LineKind::Unchanged, line_nr, change)
}

fn line_gap() -> Line {
    Line { kind: LineKind::Gap, nr: 0, spans: SmallVec::new() }
}

fn diff_line(kind: LineKind, nr: usize, change: &InlineChange<str>) -> Line {
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

pub struct Image {
    pub data_url: String,
}

impl Image {
    pub fn new(data_url: String) -> Self {
        Self { data_url }
    }
}

pub fn image_diff(
    a: Option<Old<&[u8]>>,
    b: Result<&[u8], ()>,
    format: &str,
) -> FileDiff<Image> {
    let image = |bytes| Image::new(data_url(format, bytes));
    match (a, b) {
        (Some(a), b) => FileDiff::Diff(a.map(image), b.map(image)),
        (None, b) => FileDiff::Right(b.map(image)),
    }
}

fn data_url(format: &str, data: &[u8]) -> String {
    let mut data_url = format!("data:image/{format};base64,");
    base64::engine::general_purpose::STANDARD.encode_string(data, &mut data_url);
    data_url
}
