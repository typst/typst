//! Source files.

use std::collections::{hash_map::Entry, HashMap};

use crate::loading::FileId;
use crate::parse::{is_newline, Scanner};
use crate::syntax::{Pos, Span};

/// A store for loaded source files.
#[derive(Default)]
pub struct SourceMap {
    sources: HashMap<FileId, SourceFile>,
}

impl SourceMap {
    /// Create a new, empty source map
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a source file by id.
    pub fn get(&self, file: FileId) -> Option<&SourceFile> {
        self.sources.get(&file)
    }

    /// Insert a sources.
    pub fn insert(&mut self, source: SourceFile) -> &SourceFile {
        match self.sources.entry(source.file) {
            Entry::Occupied(mut entry) => {
                entry.insert(source);
                entry.into_mut()
            }
            Entry::Vacant(entry) => entry.insert(source),
        }
    }

    /// Remove all sources.
    pub fn clear(&mut self) {
        self.sources.clear();
    }
}

/// A single source file.
pub struct SourceFile {
    file: FileId,
    src: String,
    line_starts: Vec<Pos>,
}

impl SourceFile {
    /// Create a new source file from  string.
    pub fn new(file: FileId, src: String) -> Self {
        let mut line_starts = vec![Pos::ZERO];
        let mut s = Scanner::new(&src);

        while let Some(c) = s.eat() {
            if is_newline(c) {
                if c == '\r' {
                    s.eat_if('\n');
                }
                line_starts.push(s.index().into());
            }
        }

        Self { file, src, line_starts }
    }

    /// The file id.
    pub fn file(&self) -> FileId {
        self.file
    }

    /// The whole source as a string slice.
    pub fn src(&self) -> &str {
        &self.src
    }

    /// Get the length of the file in bytes.
    pub fn len_bytes(&self) -> usize {
        self.src.len()
    }

    /// Get the length of the file in lines.
    pub fn len_lines(&self) -> usize {
        self.line_starts.len()
    }

    /// Slice out the part of the source code enclosed by the span.
    pub fn get(&self, span: Span) -> Option<&str> {
        self.src.get(span.to_range())
    }

    /// Return the index of the line that contains the given byte position.
    pub fn pos_to_line(&self, byte_pos: Pos) -> Option<usize> {
        (byte_pos.to_usize() <= self.src.len()).then(|| {
            match self.line_starts.binary_search(&byte_pos) {
                Ok(i) => i,
                Err(i) => i - 1,
            }
        })
    }

    /// Return the column of the byte index.
    ///
    /// Tabs are counted as occupying two columns.
    pub fn pos_to_column(&self, byte_pos: Pos) -> Option<usize> {
        let line = self.pos_to_line(byte_pos)?;
        let start = self.line_to_pos(line)?;
        let head = self.get(Span::new(start, byte_pos))?;
        Some(head.chars().map(width).sum())
    }

    /// Return the byte position at which the given line starts.
    pub fn line_to_pos(&self, line_idx: usize) -> Option<Pos> {
        self.line_starts.get(line_idx).copied()
    }

    /// Return the span which encloses the given line.
    pub fn line_to_span(&self, line_idx: usize) -> Option<Span> {
        let start = self.line_to_pos(line_idx)?;
        let end = self.line_to_pos(line_idx + 1).unwrap_or(self.src.len().into());
        Some(Span::new(start, end))
    }

    /// Return the byte position of the given (line, column) pair.
    ///
    /// Tabs are counted as occupying two columns.
    pub fn line_column_to_pos(&self, line_idx: usize, column_idx: usize) -> Option<Pos> {
        let span = self.line_to_span(line_idx)?;
        let line = self.get(span)?;

        if column_idx == 0 {
            return Some(span.start);
        }

        let mut column = 0;
        for (i, c) in line.char_indices() {
            column += width(c);
            if column >= column_idx {
                return Some(span.start + Pos::from(i + c.len_utf8()));
            }
        }

        None
    }
}

/// The display width of the character.
fn width(c: char) -> usize {
    if c == '\t' { 2 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: FileId = FileId::from_raw(0);
    const TEST: &str = "äbcde\nf💛g\r\nhi\rjkl";

    #[test]
    fn test_source_file_new() {
        let source = SourceFile::new(ID, TEST.into());
        assert_eq!(source.line_starts, vec![Pos(0), Pos(7), Pos(15), Pos(18)]);
    }

    #[test]
    fn test_source_file_pos_to_line() {
        let source = SourceFile::new(ID, TEST.into());
        assert_eq!(source.pos_to_line(Pos(0)), Some(0));
        assert_eq!(source.pos_to_line(Pos(2)), Some(0));
        assert_eq!(source.pos_to_line(Pos(6)), Some(0));
        assert_eq!(source.pos_to_line(Pos(7)), Some(1));
        assert_eq!(source.pos_to_line(Pos(8)), Some(1));
        assert_eq!(source.pos_to_line(Pos(12)), Some(1));
        assert_eq!(source.pos_to_line(Pos(21)), Some(3));
        assert_eq!(source.pos_to_line(Pos(22)), None);
    }

    #[test]
    fn test_source_file_roundtrip() {
        #[track_caller]
        fn roundtrip(source: &SourceFile, byte_pos: Pos) {
            let line = source.pos_to_line(byte_pos).unwrap();
            let column = source.pos_to_column(byte_pos).unwrap();
            let result = source.line_column_to_pos(line, column).unwrap();
            assert_eq!(result, byte_pos);
        }

        let source = SourceFile::new(ID, TEST.into());
        roundtrip(&source, Pos(0));
        roundtrip(&source, Pos(7));
        roundtrip(&source, Pos(12));
        roundtrip(&source, Pos(21));
    }
}
