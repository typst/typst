use std::fmt::Display;
use std::path::Path;
use std::time::Duration;

use base64::Engine;
use ecow::{EcoString, eco_format};
use similar::{ChangeTag, InlineChange, TextDiff};
use smallvec::SmallVec;

pub mod html;

pub struct TestReport {
    pub name: EcoString,
    pub diffs: Vec<DiffKind>,
}

impl TestReport {
    pub fn new(name: EcoString) -> Self {
        Self { name, diffs: Vec::new() }
    }
}

pub enum DiffKind {
    Text(TextFileDiff),
    Image(ImageFileDiff),
}

impl DiffKind {
    fn left_path(&self) -> &str {
        match self {
            DiffKind::Text(diff) => &diff.left.path,
            DiffKind::Image(diff) => &diff.left.path,
        }
    }

    fn right_path(&self) -> &str {
        match self {
            DiffKind::Text(diff) => &diff.right.path,
            DiffKind::Image(diff) => &diff.right.path,
        }
    }
}

pub struct TextFileDiff {
    left: Lines,
    right: Lines,
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
    kind: LineKind,
    nr: u32,
    spans: SmallVec<[TextSpan; 3]>,
}

pub struct TextSpan {
    emph: bool,
    text: EcoString,
}

/// Create a rich HTML text diff.
pub fn text_diff((path_a, a): (&Path, &str), (path_b, b): (&Path, &str)) -> TextFileDiff {
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
        left: Lines::new(path_a, left),
        right: Lines::new(path_b, right),
    }
}

fn line_empty() -> Line {
    Line {
        kind: LineKind::Empty,
        nr: 0,
        spans: SmallVec::new(),
    }
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

pub struct ImageFileDiff {
    left: Image,
    right: Image,
}

pub struct Image {
    path: EcoString,
    data_url: String,
}

impl Image {
    pub fn new(path: &Path, data_url: String) -> Self {
        let path = eco_format!("{}", path.display());
        Self { path, data_url }
    }
}

pub fn image_diff(
    (path_a, a): (&Path, &[u8]),
    (path_b, b): (&Path, &[u8]),
    format: &str,
) -> ImageFileDiff {
    ImageFileDiff {
        left: Image::new(path_a, data_url(format, a)),
        right: Image::new(path_b, data_url(format, b)),
    }
}

fn data_url(format: &str, data: &[u8]) -> String {
    let mut data_url = format!("data:image/{format};base64,");
    base64::engine::general_purpose::STANDARD.encode_string(data, &mut data_url);
    data_url
}
