//! Source file management.

use std::collections::HashMap;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::diag::TypResult;
use crate::loading::{FileHash, Loader};
use crate::parse::{is_newline, parse, Reparser, Scanner};
use crate::syntax::ast::Markup;
use crate::syntax::{self, Category, GreenNode, RedNode};
use crate::util::PathExt;

#[cfg(feature = "codespan-reporting")]
use codespan_reporting::files::{self, Files};

/// A unique identifier for a loaded source file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SourceId(u32);

impl SourceId {
    /// Create a source id from the raw underlying value.
    ///
    /// This should only be called with values returned by
    /// [`into_raw`](Self::into_raw).
    pub const fn from_raw(v: u32) -> Self {
        Self(v)
    }

    /// Convert into the raw underlying value.
    pub const fn into_raw(self) -> u32 {
        self.0
    }
}

/// Storage for loaded source files.
pub struct SourceStore {
    loader: Rc<dyn Loader>,
    files: HashMap<FileHash, SourceId>,
    sources: Vec<SourceFile>,
}

impl SourceStore {
    /// Create a new, empty source store.
    pub fn new(loader: Rc<dyn Loader>) -> Self {
        Self {
            loader,
            files: HashMap::new(),
            sources: vec![],
        }
    }

    /// Load a source file from a path using the `loader`.
    pub fn load(&mut self, path: &Path) -> io::Result<SourceId> {
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
    /// overwritten.
    pub fn provide(&mut self, path: &Path, src: String) -> SourceId {
        let hash = self.loader.resolve(path).ok();

        // Check for existing file and replace if one exists.
        if let Some(&id) = hash.and_then(|hash| self.files.get(&hash)) {
            self.sources[id.0 as usize] = SourceFile::new(id, path, src);
            return id;
        }

        // No existing file yet.
        let id = SourceId(self.sources.len() as u32);
        self.sources.push(SourceFile::new(id, path, src));

        // Register in file map if the path was known to the loader.
        if let Some(hash) = hash {
            self.files.insert(hash, id);
        }

        id
    }

    /// Edit a source file by replacing the given range.
    ///
    /// This panics if no source file with this `id` exists or if the `replace`
    /// range is out of bounds for the source file identified by `id`.
    #[track_caller]
    pub fn edit(&mut self, id: SourceId, replace: Range<usize>, with: &str) {
        self.sources[id.0 as usize].edit(replace, with);
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
}

/// A single source file.
///
/// _Note_: All line and column indices start at zero, just like byte indices.
/// Only for user-facing display, you should add 1 to them.
pub struct SourceFile {
    id: SourceId,
    path: PathBuf,
    src: String,
    line_starts: Vec<usize>,
    root: Rc<GreenNode>,
}

impl SourceFile {
    /// Create a new source file.
    pub fn new(id: SourceId, path: &Path, src: String) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(newlines(&src));
        Self {
            id,
            path: path.normalize(),
            root: parse(&src),
            src,
            line_starts,
        }
    }

    /// Create a source file without a real id and path, usually for testing.
    pub fn detached(src: impl Into<String>) -> Self {
        Self::new(SourceId(0), Path::new(""), src.into())
    }

    /// The root node of the file's untyped green tree.
    pub fn root(&self) -> &Rc<GreenNode> {
        &self.root
    }

    /// The root node of the file's typed abstract syntax tree.
    pub fn ast(&self) -> TypResult<Markup> {
        let red = RedNode::from_root(self.root.clone(), self.id);
        let errors = red.errors();
        if errors.is_empty() {
            Ok(red.cast().unwrap())
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

    /// Slice out the part of the source code enclosed by the span.
    pub fn get(&self, range: Range<usize>) -> Option<&str> {
        self.src.get(range)
    }

    /// Get the length of the file in bytes.
    pub fn len_bytes(&self) -> usize {
        self.src.len()
    }

    /// Get the length of the file in lines.
    pub fn len_lines(&self) -> usize {
        self.line_starts.len()
    }

    /// Return the index of the UTF-16 code unit at the byte index.
    pub fn byte_to_utf16(&self, byte_idx: usize) -> Option<usize> {
        Some(self.src.get(.. byte_idx)?.chars().map(char::len_utf16).sum())
    }

    /// Return the index of the line that contains the given byte index.
    pub fn byte_to_line(&self, byte_idx: usize) -> Option<usize> {
        (byte_idx <= self.src.len()).then(|| {
            match self.line_starts.binary_search(&byte_idx) {
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

    /// Return the index of the UTF-16 code unit at the byte index.
    pub fn utf16_to_byte(&self, utf16_idx: usize) -> Option<usize> {
        let mut k = 0;
        for (i, c) in self.src.char_indices() {
            if k >= utf16_idx {
                return Some(i);
            }
            k += c.len_utf16();
        }
        (k == utf16_idx).then(|| self.src.len())
    }

    /// Return the byte position at which the given line starts.
    pub fn line_to_byte(&self, line_idx: usize) -> Option<usize> {
        self.line_starts.get(line_idx).copied()
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

    /// Edit the source file by replacing the given range.
    ///
    /// Returns the range of the section in the new source that was ultimately
    /// reparsed. The method panics if the `replace` range is out of bounds.
    pub fn edit(&mut self, replace: Range<usize>, with: &str) -> Range<usize> {
        let start = replace.start;
        self.src.replace_range(replace.clone(), with);

        // Remove invalidated line starts.
        let line = self.byte_to_line(start).unwrap();
        self.line_starts.truncate(line + 1);

        // Handle adjoining of \r and \n.
        if self.src[.. start].ends_with('\r') && with.starts_with('\n') {
            self.line_starts.pop();
        }

        // Recalculate the line starts after the edit.
        self.line_starts
            .extend(newlines(&self.src[start ..]).map(|idx| start + idx));

        // Update the root node.
        let reparser = Reparser::new(&self.src, replace, with.len());
        if let Some(range) = reparser.reparse(Rc::make_mut(&mut self.root)) {
            range
        } else {
            self.root = parse(&self.src);
            0 .. self.src.len()
        }
    }

    /// Provide highlighting categories for the given range of the source file.
    pub fn highlight<F>(&self, range: Range<usize>, mut f: F)
    where
        F: FnMut(Range<usize>, Category),
    {
        let red = RedNode::from_root(self.root.clone(), self.id);
        syntax::highlight(red.as_ref(), range, &mut f)
    }
}

/// The indices at which lines start (right behind newlines).
///
/// The beginning of the string (index 0) is not returned.
fn newlines(string: &str) -> impl Iterator<Item = usize> + '_ {
    let mut s = Scanner::new(string);
    std::iter::from_fn(move || {
        while let Some(c) = s.eat() {
            if is_newline(c) {
                if c == '\r' {
                    s.eat_if('\n');
                }
                return Some(s.index());
            }
        }
        None
    })
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
        assert_eq!(source.line_starts, [0, 7, 15, 18]);
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
            assert_eq!(source.line_starts, result.line_starts);
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
