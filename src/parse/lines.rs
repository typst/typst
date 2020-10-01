//! Conversion of byte positions to line/column locations.

use super::Scanner;
use crate::syntax::{Location, Pos};

/// Enables conversion of byte position to locations.
pub struct LineMap<'s> {
    src: &'s str,
    line_starts: Vec<Pos>,
}

impl<'s> LineMap<'s> {
    /// Create a new line map for a source string.
    pub fn new(src: &'s str) -> Self {
        let mut line_starts = vec![Pos::ZERO];
        let mut s = Scanner::new(src);

        while let Some(c) = s.eat_merging_crlf() {
            if is_newline(c) {
                line_starts.push(s.index().into());
            }
        }

        Self { src, line_starts }
    }

    /// Convert a byte position to a location.
    ///
    /// # Panics
    /// This panics if the position is out of bounds.
    pub fn location(&self, pos: Pos) -> Location {
        let line_index = match self.line_starts.binary_search(&pos) {
            Ok(i) => i,
            Err(i) => i - 1,
        };

        let line_start = self.line_starts[line_index];
        let head = &self.src[line_start.to_usize() .. pos.to_usize()];
        let column_index = head.chars().count();

        Location {
            line: 1 + line_index as u32,
            column: 1 + column_index as u32,
        }
    }
}

/// Whether this character denotes a newline.
pub fn is_newline(character: char) -> bool {
    match character {
        // Line Feed, Vertical Tab, Form Feed, Carriage Return.
        '\n' | '\x0B' | '\x0C' | '\r' |
        // Next Line, Line Separator, Paragraph Separator.
        '\u{0085}' | '\u{2028}' | '\u{2029}' => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST: &str = "äbcde\nf💛g\r\nhi\rjkl";

    #[test]
    fn test_line_map_new() {
        let map = LineMap::new(TEST);
        assert_eq!(map.line_starts, vec![Pos(0), Pos(7), Pos(15), Pos(18)]);
    }

    #[test]
    fn test_line_map_location() {
        let map = LineMap::new(TEST);
        assert_eq!(map.location(Pos(0)), Location::new(1, 1));
        assert_eq!(map.location(Pos(2)), Location::new(1, 2));
        assert_eq!(map.location(Pos(6)), Location::new(1, 6));
        assert_eq!(map.location(Pos(7)), Location::new(2, 1));
        assert_eq!(map.location(Pos(8)), Location::new(2, 2));
        assert_eq!(map.location(Pos(12)), Location::new(2, 3));
        assert_eq!(map.location(Pos(21)), Location::new(4, 4));
    }

    #[test]
    #[should_panic]
    fn test_line_map_panics_out_of_bounds() {
        LineMap::new(TEST).location(Pos(22));
    }
}
