//! Source file management.

use std::collections::HashMap;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use unscanny::Scanner;

use crate::diag::TypResult;
use crate::loading::{FileHash, Loader};
use crate::parse::{is_newline, parse, reparse};
use crate::syntax::ast::Markup;
use crate::syntax::{Span, SyntaxNode};
use crate::util::{PathExt, StrExt};

#[cfg(feature = "codespan-reporting")]
use codespan_reporting::files::{self, Files};

/// A unique identifier for a loaded source file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SourceId(u16);

impl SourceId {
    /// Create a new source id for a file that is not part of a store.
    pub const fn detached() -> Self {
        Self(u16::MAX)
    }

    /// Create a source id from the raw underlying value.
    ///
    /// This should only be called with values returned by
    /// [`into_raw`](Self::into_raw).
    pub const fn from_raw(v: u16) -> Self {
        Self(v)
    }

    /// Convert into the raw underlying value.
    pub const fn into_raw(self) -> u16 {
        self.0
    }
}

/// Storage for loaded source files.
pub struct SourceStore {
    loader: Arc<dyn Loader>,
    files: HashMap<FileHash, SourceId>,
    sources: Vec<SourceFile>,
}

impl SourceStore {
    /// Create a new, empty source store.
    pub fn new(loader: Arc<dyn Loader>) -> Self {
        Self {
            loader,
            files: HashMap::new(),
            sources: vec![],
        }
    }

    /// Get a reference to a loaded source file.
    ///
    /// This panics if no source file with this `id` exists. This function
    /// should only be called with ids returned by this store's
    /// [`load()`](Self::load) and [`provide()`](Self::provide) methods.
    #[track_caller]
    pub fn get(&self, id: SourceId) -> &SourceFile {
        &self.sources[id.0 as usize]
    }

    /// Load a source file from a path relative to the compilation environment's
    /// root.
    ///
    /// If there already exists a source file for this path, it is
    /// [replaced](SourceFile::replace).
    pub fn load(&mut self, path: impl AsRef<Path>) -> io::Result<SourceId> {
        let path = path.as_ref();
        let hash = self.loader.resolve(path)?;
        if let Some(&id) = self.files.get(&hash) {
            return Ok(id);
        }

        let data = self.loader.load(path)?;
        let src = String::from_utf8(data).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "file is not valid utf-8")
        })?;

        Ok(self.provide(path, src))
    }

    /// Directly provide a source file.
    ///
    /// The `path` does not need to be [resolvable](Loader::resolve) through the
    /// `loader`. If it is though, imports that resolve to the same file hash
    /// will use the inserted file instead of going through [`Loader::load`].
    ///
    /// If the path is resolvable and points to an existing source file, it is
    /// [replaced](SourceFile::replace).
    pub fn provide(&mut self, path: impl AsRef<Path>, src: String) -> SourceId {
        let path = path.as_ref();
        let hash = self.loader.resolve(path).ok();

        // Check for existing file and replace if one exists.
        if let Some(&id) = hash.and_then(|hash| self.files.get(&hash)) {
            self.replace(id, src);
            return id;
        }

        // No existing file yet.
        let id = SourceId(self.sources.len() as u16);
        self.sources.push(SourceFile::new(id, path, src));

        // Register in file map if the path was known to the loader.
        if let Some(hash) = hash {
            self.files.insert(hash, id);
        }

        id
    }

    /// Fully [replace](SourceFile::replace) the source text of a file.
    ///
    /// This panics if no source file with this `id` exists.
    #[track_caller]
    pub fn replace(&mut self, id: SourceId, src: String) {
        self.sources[id.0 as usize].replace(src)
    }

    /// [Edit](SourceFile::edit) a source file by replacing the given range.
    ///
    /// This panics if no source file with this `id` exists or if the `replace`
    /// range is out of bounds.
    #[track_caller]
    pub fn edit(
        &mut self,
        id: SourceId,
        replace: Range<usize>,
        with: &str,
    ) -> Range<usize> {
        self.sources[id.0 as usize].edit(replace, with)
    }

    /// Map a span that points into a [file](SourceFile::range) stored in this
    /// source store to a byte range.
    ///
    /// Panics if the span does not point into this source store.
    pub fn range(&self, span: Span) -> Range<usize> {
        self.get(span.source()).range(span)
    }
}

/// A single source file.
///
/// _Note_: All line and column indices start at zero, just like byte indices.
/// Only for user-facing display, you should add 1 to them.
pub struct SourceFile {
    id: SourceId,
    path: PathBuf,
    src: String,
    lines: Vec<Line>,
    root: SyntaxNode,
    rev: usize,
}

impl SourceFile {
    /// Create a new source file.
    pub fn new(id: SourceId, path: &Path, src: String) -> Self {
        let mut lines = vec![Line { byte_idx: 0, utf16_idx: 0 }];
        lines.extend(Line::iter(0, 0, &src));

        let mut root = parse(&src);
        root.numberize(id, Span::FULL).unwrap();

        Self {
            id,
            path: path.normalize(),
            root,
            src,
            lines,
            rev: 0,
        }
    }

    /// Create a source file without a real id and path, usually for testing.
    pub fn detached(src: impl Into<String>) -> Self {
        Self::new(SourceId::detached(), Path::new(""), src.into())
    }

    /// Create a source file with the same synthetic span for all nodes.
    pub fn synthesized(src: impl Into<String>, span: Span) -> Self {
        let mut file = Self::detached(src);
        file.root.synthesize(span);
        file.id = span.source();
        file
    }

    /// The root node of the file's untyped syntax tree.
    pub fn root(&self) -> &SyntaxNode {
        &self.root
    }

    /// The root node of the file's typed abstract syntax tree.
    pub fn ast(&self) -> TypResult<Markup> {
        let errors = self.root.errors();
        if errors.is_empty() {
            Ok(self.root.cast().unwrap())
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
    pub fn src(&self) -> &str {
        &self.src
    }

    /// The revision number of the file.
    ///
    /// This is increased on [replacements](Self::replace) and
    /// [edits](Self::edit).
    pub fn rev(&self) -> usize {
        self.rev
    }

    /// Slice out the part of the source code enclosed by the range.
    pub fn get(&self, range: Range<usize>) -> Option<&str> {
        self.src.get(range)
    }

    /// Fully replace the source text and increase the revision number.
    pub fn replace(&mut self, src: String) {
        self.src = src;
        self.lines = vec![Line { byte_idx: 0, utf16_idx: 0 }];
        self.lines.extend(Line::iter(0, 0, &self.src));
        self.root = parse(&self.src);
        self.root.numberize(self.id(), Span::FULL).unwrap();
        self.rev = self.rev.wrapping_add(1);
    }

    /// Edit the source file by replacing the given range and increase the
    /// revision number.
    ///
    /// Returns the range in the new source that was ultimately reparsed.
    ///
    /// The method panics if the `replace` range is out of bounds.
    pub fn edit(&mut self, replace: Range<usize>, with: &str) -> Range<usize> {
        self.rev = self.rev.wrapping_add(1);

        let start_byte = replace.start;
        let start_utf16 = self.byte_to_utf16(replace.start).unwrap();
        self.src.replace_range(replace.clone(), with);

        // Remove invalidated line starts.
        let line = self.byte_to_line(start_byte).unwrap();
        self.lines.truncate(line + 1);

        // Handle adjoining of \r and \n.
        if self.src[.. start_byte].ends_with('\r') && with.starts_with('\n') {
            self.lines.pop();
        }

        // Recalculate the line starts after the edit.
        self.lines.extend(Line::iter(
            start_byte,
            start_utf16,
            &self.src[start_byte ..],
        ));

        // Incrementally reparse the replaced range.
        reparse(&mut self.root, &self.src, replace, with.len())
    }

    /// Get the length of the file in bytes.
    pub fn len_bytes(&self) -> usize {
        self.src.len()
    }

    /// Get the length of the file in UTF16 code units.
    pub fn len_utf16(&self) -> usize {
        let last = self.lines.last().unwrap();
        last.utf16_idx + self.src[last.byte_idx ..].len_utf16()
    }

    /// Get the length of the file in lines.
    pub fn len_lines(&self) -> usize {
        self.lines.len()
    }

    /// Map a span that points into this source file to a byte range.
    ///
    /// Panics if the span does not point into this source file.
    pub fn range(&self, span: Span) -> Range<usize> {
        self.root
            .range(span, 0)
            .expect("span does not point into this source file")
    }

    /// Return the index of the UTF-16 code unit at the byte index.
    pub fn byte_to_utf16(&self, byte_idx: usize) -> Option<usize> {
        let line_idx = self.byte_to_line(byte_idx)?;
        let line = self.lines.get(line_idx)?;
        let head = self.src.get(line.byte_idx .. byte_idx)?;
        Some(line.utf16_idx + head.len_utf16())
    }

    /// Return the index of the line that contains the given byte index.
    pub fn byte_to_line(&self, byte_idx: usize) -> Option<usize> {
        (byte_idx <= self.src.len()).then(|| {
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
        let head = self.get(start .. byte_idx)?;
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
        for (i, c) in self.src[line.byte_idx ..].char_indices() {
            if k >= utf16_idx {
                return Some(line.byte_idx + i);
            }
            k += c.len_utf16();
        }

        (k == utf16_idx).then(|| self.src.len())
    }


    /// Return the byte position at which the given line starts.
    pub fn line_to_byte(&self, line_idx: usize) -> Option<usize> {
        self.lines.get(line_idx).map(|line| line.byte_idx)
    }

    /// Return the range which encloses the given line.
    pub fn line_to_range(&self, line_idx: usize) -> Option<Range<usize>> {
        let start = self.line_to_byte(line_idx)?;
        let end = self.line_to_byte(line_idx + 1).unwrap_or(self.src.len());
        Some(start .. end)
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
        for _ in 0 .. column_idx {
            chars.next();
        }
        Some(range.start + (line.len() - chars.as_str().len()))
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

impl Line {
    /// Iterate over the lines in the string.
    fn iter(
        byte_offset: usize,
        utf16_offset: usize,
        string: &str,
    ) -> impl Iterator<Item = Line> + '_ {
        let mut s = Scanner::new(string);
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

            Some(Line {
                byte_idx: byte_offset + s.cursor(),
                utf16_idx,
            })
        })
    }
}

impl AsRef<str> for SourceFile {
    fn as_ref(&self) -> &str {
        &self.src
    }
}

#[cfg(feature = "codespan-reporting")]
impl<'a> Files<'a> for SourceStore {
    type FileId = SourceId;
    type Name = std::path::Display<'a>;
    type Source = &'a SourceFile;

    fn name(&'a self, id: SourceId) -> Result<Self::Name, files::Error> {
        Ok(self.get(id).path().display())
    }

    fn source(&'a self, id: SourceId) -> Result<Self::Source, files::Error> {
        Ok(self.get(id))
    }

    fn line_index(&'a self, id: SourceId, given: usize) -> Result<usize, files::Error> {
        let source = self.get(id);
        source
            .byte_to_line(given)
            .ok_or_else(|| files::Error::IndexTooLarge { given, max: source.len_bytes() })
    }

    fn line_range(
        &'a self,
        id: SourceId,
        given: usize,
    ) -> Result<std::ops::Range<usize>, files::Error> {
        let source = self.get(id);
        source
            .line_to_range(given)
            .ok_or_else(|| files::Error::LineTooLarge { given, max: source.len_lines() })
    }

    fn column_number(
        &'a self,
        id: SourceId,
        _: usize,
        given: usize,
    ) -> Result<usize, files::Error> {
        let source = self.get(id);
        source.byte_to_column(given).ok_or_else(|| {
            let max = source.len_bytes();
            if given <= max {
                files::Error::InvalidCharBoundary { given }
            } else {
                files::Error::IndexTooLarge { given, max }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST: &str = "ä\tcde\nf💛g\r\nhi\rjkl";

    #[test]
    fn test_source_file_new() {
        let source = SourceFile::detached(TEST);
        assert_eq!(source.lines, [
            Line { byte_idx: 0, utf16_idx: 0 },
            Line { byte_idx: 7, utf16_idx: 6 },
            Line { byte_idx: 15, utf16_idx: 12 },
            Line { byte_idx: 18, utf16_idx: 15 },
        ]);
    }

    #[test]
    fn test_source_file_pos_to_line() {
        let source = SourceFile::detached(TEST);
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
        let source = SourceFile::detached(TEST);
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
        fn roundtrip(source: &SourceFile, byte_idx: usize, utf16_idx: usize) {
            let middle = source.byte_to_utf16(byte_idx).unwrap();
            let result = source.utf16_to_byte(middle).unwrap();
            assert_eq!(middle, utf16_idx);
            assert_eq!(result, byte_idx);
        }

        let source = SourceFile::detached(TEST);
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
        fn roundtrip(source: &SourceFile, byte_idx: usize) {
            let line = source.byte_to_line(byte_idx).unwrap();
            let column = source.byte_to_column(byte_idx).unwrap();
            let result = source.line_column_to_byte(line, column).unwrap();
            assert_eq!(result, byte_idx);
        }

        let source = SourceFile::detached(TEST);
        roundtrip(&source, 0);
        roundtrip(&source, 7);
        roundtrip(&source, 12);
        roundtrip(&source, 21);
    }

    #[test]
    fn test_source_file_edit() {
        #[track_caller]
        fn test(prev: &str, range: Range<usize>, with: &str, after: &str) {
            let mut source = SourceFile::detached(prev);
            let result = SourceFile::detached(after);
            source.edit(range, with);
            assert_eq!(source.src, result.src);
            assert_eq!(source.lines, result.lines);
        }

        // Test inserting at the begining.
        test("abc\n", 0 .. 0, "hi\n", "hi\nabc\n");
        test("\nabc", 0 .. 0, "hi\r", "hi\r\nabc");

        // Test editing in the middle.
        test(TEST, 4 .. 16, "❌", "ä\tc❌i\rjkl");

        // Test appending.
        test("abc\ndef", 7 .. 7, "hi", "abc\ndefhi");
        test("abc\ndef\n", 8 .. 8, "hi", "abc\ndef\nhi");

        // Test appending with adjoining \r and \n.
        test("abc\ndef\r", 8 .. 8, "\nghi", "abc\ndef\r\nghi");

        // Test removing everything.
        test(TEST, 0 .. 21, "", "");
    }
}
