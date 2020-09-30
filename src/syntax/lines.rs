//! Conversion of byte positions to line/column locations.

use std::fmt::{self, Debug, Display, Formatter};

use super::Pos;
use crate::parse::{is_newline_char, CharParser};

/// Enables conversion of byte position to locations.
pub struct LineMap<'s> {
    src: &'s str,
    line_starts: Vec<Pos>,
}

impl<'s> LineMap<'s> {
    /// Create a new line map for a source string.
    pub fn new(src: &'s str) -> Self {
        let mut line_starts = vec![Pos::ZERO];
        let mut p = CharParser::new(src);

        while let Some(c) = p.eat_merging_crlf() {
            if is_newline_char(c) {
                line_starts.push(p.index().into());
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

/// One-indexed line-column position in source code.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Location {
    /// The one-indexed line.
    pub line: u32,
    /// The one-indexed column.
    pub column: u32,
}

impl Location {
    /// Create a new location from line and column.
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

impl Debug for Location {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
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
