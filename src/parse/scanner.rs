use std::fmt::{self, Debug, Formatter};
use std::slice::SliceIndex;

/// A featureful char-based scanner.
#[derive(Copy, Clone)]
pub struct Scanner<'s> {
    /// The string to scan.
    src: &'s str,
    /// The index at which the peekable character starts. Must be in bounds and
    /// at a codepoint boundary to guarantee safety.
    index: usize,
}

impl<'s> Scanner<'s> {
    /// Create a new char scanner.
    pub fn new(src: &'s str) -> Self {
        Self { src, index: 0 }
    }

    /// Consume the next char.
    pub fn eat(&mut self) -> Option<char> {
        let next = self.peek();
        if let Some(c) = next {
            self.index += c.len_utf8();
        }
        next
    }

    /// Consume the next char if it is the given one.
    ///
    /// Returns whether the char was consumed.
    pub fn eat_if(&mut self, c: char) -> bool {
        let matches = self.peek() == Some(c);
        if matches {
            self.index += c.len_utf8();
        }
        matches
    }

    /// Consume the next char, debug-asserting that it is the given one.
    pub fn eat_assert(&mut self, c: char) {
        let next = self.eat();
        debug_assert_eq!(next, Some(c));
    }

    /// Consume the next char, coalescing `\r\n` to just `\n`.
    pub fn eat_merging_crlf(&mut self) -> Option<char> {
        if self.rest().starts_with("\r\n") {
            self.index += 2;
            Some('\n')
        } else {
            self.eat()
        }
    }

    /// Eat chars while the condition is true.
    pub fn eat_while<F>(&mut self, mut f: F) -> &'s str
    where
        F: FnMut(char) -> bool,
    {
        self.eat_until(|c| !f(c))
    }

    /// Eat chars until the condition is true.
    pub fn eat_until<F>(&mut self, mut f: F) -> &'s str
    where
        F: FnMut(char) -> bool,
    {
        let start = self.index;
        while let Some(c) = self.peek() {
            if f(c) {
                break;
            }
            self.index += c.len_utf8();
        }
        self.eaten_from(start)
    }

    /// Uneat the last eaten char.
    pub fn uneat(&mut self) {
        self.index = self.last_index();
    }

    /// Peek at the next char without consuming it.
    pub fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    /// Checks whether the next char fulfills a condition.
    ///
    /// Returns `false` if there is no next char.
    pub fn check<F>(&self, f: F) -> bool
    where
        F: FnOnce(char) -> bool,
    {
        self.peek().map(f).unwrap_or(false)
    }

    /// The previous index in the source string.
    pub fn last_index(&self) -> usize {
        self.eaten()
            .chars()
            .next_back()
            .map_or(0, |c| self.index - c.len_utf8())
    }

    /// The current index in the source string.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Jump to an index in the source string.
    pub fn jump(&mut self, index: usize) {
        // Make sure that the index is in bounds and on a codepoint boundary.
        self.src.get(index ..).expect("jumped to invalid index");
        self.index = index;
    }

    /// Slice a part out of the source string.
    pub fn get<I>(&self, index: I) -> &'s str
    where
        I: SliceIndex<str, Output = str>,
    {
        &self.src[index]
    }

    /// The full source string up to the current index.
    pub fn eaten(&self) -> &'s str {
        // SAFETY: The index is always in bounds and on a codepoint boundary
        // since it is:
        // - either increased by the length of a scanned character,
        // - or checked upon jumping.
        unsafe { self.src.get_unchecked(.. self.index) }
    }

    /// The source string from `start` to the current index.
    pub fn eaten_from(&self, start: usize) -> &'s str {
        &self.src[start .. self.index]
    }

    /// The remaining source string after the current index.
    pub fn rest(&self) -> &'s str {
        // SAFETY: The index is always okay, for details see `eaten()`.
        unsafe { self.src.get_unchecked(self.index ..) }
    }
}

impl Debug for Scanner<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Scanner({}|{})", self.eaten(), self.rest())
    }
}
