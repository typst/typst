// FIXME:
// Both `LineMap::location` and `search_column` can lead to quadratic compile
// times for very long lines. We probably need some smart acceleration structure
// to determine columns.

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
    pub fn location(&self, pos: Pos) -> Option<Location> {
        // Find the line which contains the position.
        let line_index = match self.line_starts.binary_search(&pos) {
            Ok(i) => i,
            Err(i) => i - 1,
        };

        let start = self.line_starts.get(line_index)?;
        let head = self.src.get(start.to_usize() .. pos.to_usize())?;

        // TODO: What about tabs?
        let column_index = head.chars().count();

        Some(Location {
            line: 1 + line_index as u32,
            column: 1 + column_index as u32,
        })
    }

    /// Convert a location to a byte position.
    pub fn pos(&self, location: Location) -> Option<Pos> {
        // Determine the boundaries of the line.
        let line_idx = location.line.checked_sub(1)? as usize;
        let line_start = *self.line_starts.get(line_idx)?;
        let line_end = self
            .line_starts
            .get(location.line as usize)
            .map_or(self.src.len(), |pos| pos.to_usize());

        let line = self.src.get(line_start.to_usize() .. line_end)?;

        // Find the index in the line. For the first column, the index is always
        // zero. For other columns, we have to look at which byte the char
        // directly before the column in question ends. We can't do
        // `nth(column_idx)` directly since the column may be behind the last
        // char.
        let column_idx = location.column.checked_sub(1)? as usize;
        let line_offset = if let Some(prev_idx) = column_idx.checked_sub(1) {
            // TODO: What about tabs?
            let (idx, prev) = line.char_indices().nth(prev_idx)?;
            idx + prev.len_utf8()
        } else {
            0
        };

        Some(line_start + line_offset)
    }
}

/// Determine the column at the end of the string.
pub fn search_column(src: &str) -> usize {
    let mut column = 0;
    for c in src.chars().rev() {
        if is_newline(c) {
            break;
        } else if c == '\t' {
            // TODO: How many columns per tab?
            column += 2;
        } else {
            column += 1;
        }
    }
    column
}

/// Whether this character denotes a newline.
#[inline]
pub fn is_newline(character: char) -> bool {
    matches!(
        character,
        // Line Feed, Vertical Tab, Form Feed, Carriage Return.
        '\n' | '\x0B' | '\x0C' | '\r' |
        // Next Line, Line Separator, Paragraph Separator.
        '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
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
        assert_eq!(map.location(Pos(0)), Some(Location::new(1, 1)));
        assert_eq!(map.location(Pos(2)), Some(Location::new(1, 2)));
        assert_eq!(map.location(Pos(6)), Some(Location::new(1, 6)));
        assert_eq!(map.location(Pos(7)), Some(Location::new(2, 1)));
        assert_eq!(map.location(Pos(8)), Some(Location::new(2, 2)));
        assert_eq!(map.location(Pos(12)), Some(Location::new(2, 3)));
        assert_eq!(map.location(Pos(21)), Some(Location::new(4, 4)));
        assert_eq!(map.location(Pos(22)), None);
    }

    #[test]
    fn test_line_map_pos() {
        fn assert_round_trip(map: &LineMap, pos: Pos) {
            assert_eq!(map.location(pos).and_then(|loc| map.pos(loc)), Some(pos));
        }

        let map = LineMap::new(TEST);
        assert_round_trip(&map, Pos(0));
        assert_round_trip(&map, Pos(7));
        assert_round_trip(&map, Pos(12));
        assert_round_trip(&map, Pos(21));
    }
}
