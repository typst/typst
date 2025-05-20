use std::hash::{Hash, Hasher};
use std::iter::zip;
use std::ops::Range;
use std::str::Utf8Error;
use std::sync::Arc;

use crate::is_newline;

/// A text buffer and metadata about lines.
#[derive(Clone)]
pub struct Lines<S>(Arc<Repr<S>>);

#[derive(Clone)]
struct Repr<S> {
    lines: Vec<Line>,
    str: S,
}

/// Metadata about a line.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Line {
    /// The UTF-8 byte offset where the line starts.
    byte_idx: usize,
    /// The UTF-16 codepoint offset where the line starts.
    utf16_idx: usize,
}

impl<S: AsRef<str>> Lines<S> {
    /// TODO: memoize this?
    pub fn new(str: S) -> Self {
        let lines = lines(str.as_ref());
        Lines(Arc::new(Repr { lines, str }))
    }

    pub fn text(&self) -> &str {
        self.0.str.as_ref()
    }

    /// Get the length of the file in UTF-8 encoded bytes.
    pub fn len_bytes(&self) -> usize {
        self.0.str.as_ref().len()
    }

    /// Get the length of the file in UTF-16 code units.
    pub fn len_utf16(&self) -> usize {
        let last = self.0.lines.last().unwrap();
        last.utf16_idx + len_utf16(&self.text()[last.byte_idx..])
    }

    /// Get the length of the file in lines.
    pub fn len_lines(&self) -> usize {
        self.0.lines.len()
    }

    /// Return the index of the UTF-16 code unit at the byte index.
    pub fn byte_to_utf16(&self, byte_idx: usize) -> Option<usize> {
        let line_idx = self.byte_to_line(byte_idx)?;
        let line = self.0.lines.get(line_idx)?;
        let head = self.text().get(line.byte_idx..byte_idx)?;
        Some(line.utf16_idx + len_utf16(head))
    }

    /// Return the index of the line that contains the given byte index.
    pub fn byte_to_line(&self, byte_idx: usize) -> Option<usize> {
        (byte_idx <= self.text().len()).then(|| {
            match self.0.lines.binary_search_by_key(&byte_idx, |line| line.byte_idx) {
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
        let head = self.text().get(start..byte_idx)?;
        Some(head.chars().count())
    }

    /// Return the index of the line and column at the byte index.
    pub fn byte_to_line_column(&self, byte_idx: usize) -> Option<(usize, usize)> {
        let line = self.byte_to_line(byte_idx)?;
        let start = self.line_to_byte(line)?;
        let head = self.text().get(start..byte_idx)?;
        let col = head.chars().count();
        Some((line, col))
    }

    /// Return the byte index at the UTF-16 code unit.
    pub fn utf16_to_byte(&self, utf16_idx: usize) -> Option<usize> {
        let line = self.0.lines.get(
            match self.0.lines.binary_search_by_key(&utf16_idx, |line| line.utf16_idx) {
                Ok(i) => i,
                Err(i) => i - 1,
            },
        )?;

        let text = self.text();
        let mut k = line.utf16_idx;
        for (i, c) in text[line.byte_idx..].char_indices() {
            if k >= utf16_idx {
                return Some(line.byte_idx + i);
            }
            k += c.len_utf16();
        }

        (k == utf16_idx).then_some(text.len())
    }

    /// Return the byte position at which the given line starts.
    pub fn line_to_byte(&self, line_idx: usize) -> Option<usize> {
        self.0.lines.get(line_idx).map(|line| line.byte_idx)
    }

    /// Return the range which encloses the given line.
    pub fn line_to_range(&self, line_idx: usize) -> Option<Range<usize>> {
        let start = self.line_to_byte(line_idx)?;
        let end = self.line_to_byte(line_idx + 1).unwrap_or(self.text().len());
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
        let line = self.text().get(range.clone())?;
        let mut chars = line.chars();
        for _ in 0..column_idx {
            chars.next();
        }
        Some(range.start + (line.len() - chars.as_str().len()))
    }
}

impl Lines<String> {
    /// Tries to convert the bytes
    #[comemo::memoize]
    pub fn from_bytes(bytes: &[u8]) -> Result<Lines<String>, Utf8Error> {
        let str = std::str::from_utf8(bytes)?;
        Ok(Lines::new(str.to_string()))
    }

    /// Fully replace the source text.
    ///
    /// This performs a naive (suffix/prefix-based) diff of the old and new text
    /// to produce the smallest single edit that transforms old into new and
    /// then calls [`edit`](Self::edit) with it.
    ///
    /// Returns whether any changes were made.
    pub fn replace(&mut self, new: &str) -> bool {
        let Some((prefix, suffix)) = self.replacement_range(new) else {
            return false;
        };

        let old = self.text();
        let replace = prefix..old.len() - suffix;
        let with = &new[prefix..new.len() - suffix];
        self.edit(replace, with);

        true
    }

    /// Returns the common prefix and suffix lengths.
    /// Returns [`None`] if the old and new strings are equal.
    pub fn replacement_range(&self, new: &str) -> Option<(usize, usize)> {
        let old = self.text();

        let mut prefix =
            zip(old.bytes(), new.bytes()).take_while(|(x, y)| x == y).count();

        if prefix == old.len() && prefix == new.len() {
            return None;
        }

        while !old.is_char_boundary(prefix) || !new.is_char_boundary(prefix) {
            prefix -= 1;
        }

        let mut suffix = zip(old[prefix..].bytes().rev(), new[prefix..].bytes().rev())
            .take_while(|(x, y)| x == y)
            .count();

        while !old.is_char_boundary(old.len() - suffix)
            || !new.is_char_boundary(new.len() - suffix)
        {
            suffix += 1;
        }

        Some((prefix, suffix))
    }

    /// Edit the source file by replacing the given range.
    ///
    /// Returns the range in the new source that was ultimately reparsed.
    ///
    /// The method panics if the `replace` range is out of bounds.
    #[track_caller]
    pub fn edit(&mut self, replace: Range<usize>, with: &str) {
        let start_byte = replace.start;
        let start_utf16 = self.byte_to_utf16(start_byte).unwrap();
        let line = self.byte_to_line(start_byte).unwrap();

        let inner = Arc::make_mut(&mut self.0);

        // Update the text itself.
        inner.str.replace_range(replace.clone(), with);

        // Remove invalidated line starts.
        inner.lines.truncate(line + 1);

        // Handle adjoining of \r and \n.
        if inner.str[..start_byte].ends_with('\r') && with.starts_with('\n') {
            inner.lines.pop();
        }

        // Recalculate the line starts after the edit.
        inner
            .lines
            .extend(lines_from(start_byte, start_utf16, &inner.str[start_byte..]));
    }
}

impl<S: Hash> Hash for Lines<S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.str.hash(state);
    }
}

impl<S: AsRef<str>> AsRef<str> for Lines<S> {
    fn as_ref(&self) -> &str {
        self.0.str.as_ref()
    }
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
    let mut s = unscanny::Scanner::new(text);
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

/// The number of code units this string would use if it was encoded in
/// UTF16. This runs in linear time.
fn len_utf16(string: &str) -> usize {
    string.chars().map(char::len_utf16).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST: &str = "ä\tcde\nf💛g\r\nhi\rjkl";

    #[test]
    fn test_source_file_new() {
        let lines = Lines::new(TEST);
        assert_eq!(
            lines.0.lines,
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
        let lines = Lines::new(TEST);
        assert_eq!(lines.byte_to_line(0), Some(0));
        assert_eq!(lines.byte_to_line(2), Some(0));
        assert_eq!(lines.byte_to_line(6), Some(0));
        assert_eq!(lines.byte_to_line(7), Some(1));
        assert_eq!(lines.byte_to_line(8), Some(1));
        assert_eq!(lines.byte_to_line(12), Some(1));
        assert_eq!(lines.byte_to_line(21), Some(3));
        assert_eq!(lines.byte_to_line(22), None);
    }

    #[test]
    fn test_source_file_pos_to_column() {
        let lines = Lines::new(TEST);
        assert_eq!(lines.byte_to_column(0), Some(0));
        assert_eq!(lines.byte_to_column(2), Some(1));
        assert_eq!(lines.byte_to_column(6), Some(5));
        assert_eq!(lines.byte_to_column(7), Some(0));
        assert_eq!(lines.byte_to_column(8), Some(1));
        assert_eq!(lines.byte_to_column(12), Some(2));
    }

    #[test]
    fn test_source_file_utf16() {
        #[track_caller]
        fn roundtrip(lines: &Lines<&str>, byte_idx: usize, utf16_idx: usize) {
            let middle = lines.byte_to_utf16(byte_idx).unwrap();
            let result = lines.utf16_to_byte(middle).unwrap();
            assert_eq!(middle, utf16_idx);
            assert_eq!(result, byte_idx);
        }

        let lines = Lines::new(TEST);
        roundtrip(&lines, 0, 0);
        roundtrip(&lines, 2, 1);
        roundtrip(&lines, 3, 2);
        roundtrip(&lines, 8, 7);
        roundtrip(&lines, 12, 9);
        roundtrip(&lines, 21, 18);
        assert_eq!(lines.byte_to_utf16(22), None);
        assert_eq!(lines.utf16_to_byte(19), None);
    }

    #[test]
    fn test_source_file_roundtrip() {
        #[track_caller]
        fn roundtrip(lines: &Lines<&str>, byte_idx: usize) {
            let line = lines.byte_to_line(byte_idx).unwrap();
            let column = lines.byte_to_column(byte_idx).unwrap();
            let result = lines.line_column_to_byte(line, column).unwrap();
            assert_eq!(result, byte_idx);
        }

        let lines = Lines::new(TEST);
        roundtrip(&lines, 0);
        roundtrip(&lines, 7);
        roundtrip(&lines, 12);
        roundtrip(&lines, 21);
    }

    #[test]
    fn test_source_file_edit() {
        // This tests only the non-parser parts. The reparsing itself is
        // tested separately.
        #[track_caller]
        fn test(prev: &str, range: Range<usize>, with: &str, after: &str) {
            let reference = Lines::new(after);

            let mut edited = Lines::new(prev.to_string());
            edited.edit(range.clone(), with);
            assert_eq!(edited.text(), reference.text());
            assert_eq!(edited.0.lines, reference.0.lines);

            let mut replaced = Lines::new(prev.to_string());
            replaced.replace(&{
                let mut s = prev.to_string();
                s.replace_range(range, with);
                s
            });
            assert_eq!(replaced.text(), reference.text());
            assert_eq!(replaced.0.lines, reference.0.lines);
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
