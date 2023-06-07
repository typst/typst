//! Source file management.

use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::path::{Path, PathBuf};

use comemo::Prehashed;
use unscanny::Scanner;

use super::ast::Markup;
use super::reparser::reparse;
use super::{is_newline, parse, LinkedNode, Span, SyntaxNode};
use crate::diag::SourceResult;
use crate::util::{PathExt, StrExt};

/// A source file.
///
/// All line and column indices start at zero, just like byte indices. Only for
/// user-facing display, you should add 1 to them.
#[derive(Clone)]
pub struct Source {
    id: SourceId,
    path: PathBuf,
    lines: Vec<Line>,
    text: Prehashed<String>,
    root: Prehashed<SyntaxNode>,
}

impl Source {
    /// Create a new source file.
    #[tracing::instrument(skip_all)]
    pub fn new(id: SourceId, path: &Path, text: String) -> Self {
        let mut root = parse(&text);
        root.numberize(id, Span::FULL).unwrap();
        Self {
            id,
            path: path.normalize(),
            lines: lines(&text),
            text: Prehashed::new(text),
            root: Prehashed::new(root),
        }
    }

    /// Create a source file without a real id and path, usually for testing.
    pub fn detached(text: impl Into<String>) -> Self {
        Self::new(SourceId::detached(), Path::new(""), text.into())
    }

    /// Create a source file with the same synthetic span for all nodes.
    pub fn synthesized(text: String, span: Span) -> Self {
        let mut root = parse(&text);
        root.synthesize(span);
        Self {
            id: SourceId::detached(),
            path: PathBuf::new(),
            lines: lines(&text),
            text: Prehashed::new(text),
            root: Prehashed::new(root),
        }
    }

    /// The root node of the file's untyped syntax tree.
    pub fn root(&self) -> &SyntaxNode {
        &self.root
    }

    /// The root node of the file's typed abstract syntax tree.
    pub fn ast(&self) -> SourceResult<Markup> {
        let errors = self.root.errors();
        if errors.is_empty() {
            Ok(self.root.cast().expect("root node must be markup"))
        } else {
            Err(Box::new(errors))
        }
    }

    /// The id of the source file.
    pub fn id(&self) -> SourceId {
        self.id
    }

    /// The normalized path to the source file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The whole source as a string slice.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Slice out the part of the source code enclosed by the range.
    pub fn get(&self, range: Range<usize>) -> Option<&str> {
        self.text.get(range)
    }

    /// Fully replace the source text.
    pub fn replace(&mut self, text: String) {
        self.text = Prehashed::new(text);
        self.lines = lines(&self.text);
        let mut root = parse(&self.text);
        root.numberize(self.id, Span::FULL).unwrap();
        self.root = Prehashed::new(root);
    }

    /// Edit the source file by replacing the given range.
    ///
    /// Returns the range in the new source that was ultimately reparsed.
    ///
    /// The method panics if the `replace` range is out of bounds.
    #[track_caller]
    pub fn edit(&mut self, replace: Range<usize>, with: &str) -> Range<usize> {
        let start_byte = replace.start;
        let start_utf16 = self.byte_to_utf16(replace.start).unwrap();
        self.text.update(|text| text.replace_range(replace.clone(), with));

        // Remove invalidated line starts.
        let line = self.byte_to_line(start_byte).unwrap();
        self.lines.truncate(line + 1);

        // Handle adjoining of \r and \n.
        if self.text[..start_byte].ends_with('\r') && with.starts_with('\n') {
            self.lines.pop();
        }

        // Recalculate the line starts after the edit.
        self.lines
            .extend(lines_from(start_byte, start_utf16, &self.text[start_byte..]));

        // Incrementally reparse the replaced range.
        self.root
            .update(|root| reparse(root, &self.text, replace, with.len()))
    }

    /// Get the length of the file in UTF-8 encoded bytes.
    pub fn len_bytes(&self) -> usize {
        self.text.len()
    }

    /// Get the length of the file in UTF-16 code units.
    pub fn len_utf16(&self) -> usize {
        let last = self.lines.last().unwrap();
        last.utf16_idx + self.text[last.byte_idx..].len_utf16()
    }

    /// Get the length of the file in lines.
    pub fn len_lines(&self) -> usize {
        self.lines.len()
    }

    /// Find the node with the given span.
    ///
    /// Returns `None` if the span does not point into this source file.
    pub fn find(&self, span: Span) -> Option<LinkedNode<'_>> {
        LinkedNode::new(&self.root).find(span)
    }

    /// Map a span that points into this source file to a byte range.
    ///
    /// Panics if the span does not point into this source file.
    #[track_caller]
    pub fn range(&self, span: Span) -> Range<usize> {
        self.find(span)
            .expect("span does not point into this source file")
            .range()
    }

    /// Return the index of the UTF-16 code unit at the byte index.
    pub fn byte_to_utf16(&self, byte_idx: usize) -> Option<usize> {
        let line_idx = self.byte_to_line(byte_idx)?;
        let line = self.lines.get(line_idx)?;
        let head = self.text.get(line.byte_idx..byte_idx)?;
        Some(line.utf16_idx + head.len_utf16())
    }

    /// Return the index of the line that contains the given byte index.
    pub fn byte_to_line(&self, byte_idx: usize) -> Option<usize> {
        (byte_idx <= self.text.len()).then(|| {
            match self.lines.binary_search_by_key(&byte_idx, |line| line.byte_idx) {
                Ok(i) => i,
                Err(i) => i - 1,
            }
        })
    }

    /// Return the index of the column at the byte index.
    ///
    /// The column is defined as the number of characters in the line before the
    /// byte index.
    pub fn byte_to_column(&self, byte_idx: usize) -> Option<usize> {
        let line = self.byte_to_line(byte_idx)?;
        let start = self.line_to_byte(line)?;
        let head = self.get(start..byte_idx)?;
        Some(head.chars().count())
    }

    /// Return the byte index at the UTF-16 code unit.
    pub fn utf16_to_byte(&self, utf16_idx: usize) -> Option<usize> {
        let line = self.lines.get(
            match self.lines.binary_search_by_key(&utf16_idx, |line| line.utf16_idx) {
                Ok(i) => i,
                Err(i) => i - 1,
            },
        )?;

        let mut k = line.utf16_idx;
        for (i, c) in self.text[line.byte_idx..].char_indices() {
            if k >= utf16_idx {
                return Some(line.byte_idx + i);
            }
            k += c.len_utf16();
        }

        (k == utf16_idx).then_some(self.text.len())
    }

    /// Return the byte position at which the given line starts.
    pub fn line_to_byte(&self, line_idx: usize) -> Option<usize> {
        self.lines.get(line_idx).map(|line| line.byte_idx)
    }

    /// Return the range which encloses the given line.
    pub fn line_to_range(&self, line_idx: usize) -> Option<Range<usize>> {
        let start = self.line_to_byte(line_idx)?;
        let end = self.line_to_byte(line_idx + 1).unwrap_or(self.text.len());
        Some(start..end)
    }

    /// Return the byte index of the given (line, column) pair.
    ///
    /// The column defines the number of characters to go beyond the start of
    /// the line.
    pub fn line_column_to_byte(
        &self,
        line_idx: usize,
        column_idx: usize,
    ) -> Option<usize> {
        let range = self.line_to_range(line_idx)?;
        let line = self.get(range.clone())?;
        let mut chars = line.chars();
        for _ in 0..column_idx {
            chars.next();
        }
        Some(range.start + (line.len() - chars.as_str().len()))
    }
}

impl Debug for Source {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Source({})", self.path.display())
    }
}

impl Hash for Source {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.path.hash(state);
        self.text.hash(state);
        self.root.hash(state);
    }
}

/// A unique identifier for a loaded source file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SourceId(u16);

impl SourceId {
    /// Create a new source id for a file that is not part of the world.
    pub const fn detached() -> Self {
        Self(u16::MAX)
    }

    /// Whether the source id is the detached.
    pub const fn is_detached(self) -> bool {
        self.0 == Self::detached().0
    }

    /// Create a source id from a number.
    pub const fn from_u16(v: u16) -> Self {
        Self(v)
    }

    /// Extract the underlying number.
    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

/// Metadata about a line.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Line {
    /// The UTF-8 byte offset where the line starts.
    byte_idx: usize,
    /// The UTF-16 codepoint offset where the line starts.
    utf16_idx: usize,
}

/// Create a line vector.
fn lines(text: &str) -> Vec<Line> {
    std::iter::once(Line { byte_idx: 0, utf16_idx: 0 })
        .chain(lines_from(0, 0, text))
        .collect()
}

/// Compute a line iterator from an offset.
fn lines_from(
    byte_offset: usize,
    utf16_offset: usize,
    text: &str,
) -> impl Iterator<Item = Line> + '_ {
    let mut s = Scanner::new(text);
    let mut utf16_idx = utf16_offset;

    std::iter::from_fn(move || {
        s.eat_until(|c: char| {
            utf16_idx += c.len_utf16();
            is_newline(c)
        });

        if s.done() {
            return None;
        }

        if s.eat() == Some('\r') && s.eat_if('\n') {
            utf16_idx += 1;
        }

        Some(Line { byte_idx: byte_offset + s.cursor(), utf16_idx })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST: &str = "ä\tcde\nf💛g\r\nhi\rjkl";

    #[test]
    fn test_source_file_new() {
        let source = Source::detached(TEST);
        assert_eq!(
            source.lines,
            [
                Line { byte_idx: 0, utf16_idx: 0 },
                Line { byte_idx: 7, utf16_idx: 6 },
                Line { byte_idx: 15, utf16_idx: 12 },
                Line { byte_idx: 18, utf16_idx: 15 },
            ]
        );
    }

    #[test]
    fn test_source_file_pos_to_line() {
        let source = Source::detached(TEST);
        assert_eq!(source.byte_to_line(0), Some(0));
        assert_eq!(source.byte_to_line(2), Some(0));
        assert_eq!(source.byte_to_line(6), Some(0));
        assert_eq!(source.byte_to_line(7), Some(1));
        assert_eq!(source.byte_to_line(8), Some(1));
        assert_eq!(source.byte_to_line(12), Some(1));
        assert_eq!(source.byte_to_line(21), Some(3));
        assert_eq!(source.byte_to_line(22), None);
    }

    #[test]
    fn test_source_file_pos_to_column() {
        let source = Source::detached(TEST);
        assert_eq!(source.byte_to_column(0), Some(0));
        assert_eq!(source.byte_to_column(2), Some(1));
        assert_eq!(source.byte_to_column(6), Some(5));
        assert_eq!(source.byte_to_column(7), Some(0));
        assert_eq!(source.byte_to_column(8), Some(1));
        assert_eq!(source.byte_to_column(12), Some(2));
    }

    #[test]
    fn test_source_file_utf16() {
        #[track_caller]
        fn roundtrip(source: &Source, byte_idx: usize, utf16_idx: usize) {
            let middle = source.byte_to_utf16(byte_idx).unwrap();
            let result = source.utf16_to_byte(middle).unwrap();
            assert_eq!(middle, utf16_idx);
            assert_eq!(result, byte_idx);
        }

        let source = Source::detached(TEST);
        roundtrip(&source, 0, 0);
        roundtrip(&source, 2, 1);
        roundtrip(&source, 3, 2);
        roundtrip(&source, 8, 7);
        roundtrip(&source, 12, 9);
        roundtrip(&source, 21, 18);
        assert_eq!(source.byte_to_utf16(22), None);
        assert_eq!(source.utf16_to_byte(19), None);
    }

    #[test]
    fn test_source_file_roundtrip() {
        #[track_caller]
        fn roundtrip(source: &Source, byte_idx: usize) {
            let line = source.byte_to_line(byte_idx).unwrap();
            let column = source.byte_to_column(byte_idx).unwrap();
            let result = source.line_column_to_byte(line, column).unwrap();
            assert_eq!(result, byte_idx);
        }

        let source = Source::detached(TEST);
        roundtrip(&source, 0);
        roundtrip(&source, 7);
        roundtrip(&source, 12);
        roundtrip(&source, 21);
    }

    #[test]
    fn test_source_file_edit() {
        // This tests only the non-parser parts. The reparsing itself is
        // tested separately.
        #[track_caller]
        fn test(prev: &str, range: Range<usize>, with: &str, after: &str) {
            let mut source = Source::detached(prev);
            let result = Source::detached(after);
            source.edit(range, with);
            assert_eq!(source.text, result.text);
            assert_eq!(source.lines, result.lines);
        }

        // Test inserting at the beginning.
        test("abc\n", 0..0, "hi\n", "hi\nabc\n");
        test("\nabc", 0..0, "hi\r", "hi\r\nabc");

        // Test editing in the middle.
        test(TEST, 4..16, "❌", "ä\tc❌i\rjkl");

        // Test appending.
        test("abc\ndef", 7..7, "hi", "abc\ndefhi");
        test("abc\ndef\n", 8..8, "hi", "abc\ndef\nhi");

        // Test appending with adjoining \r and \n.
        test("abc\ndef\r", 8..8, "\nghi", "abc\ndef\r\nghi");

        // Test removing everything.
        test(TEST, 0..21, "", "");
    }
}
