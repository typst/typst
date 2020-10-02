//! Low-level char-based scanner.

use std::fmt::{self, Debug, Formatter};
use std::slice::SliceIndex;
use std::str::Chars;

/// A low-level featureful char-based scanner.
#[derive(Clone)]
pub struct Scanner<'s> {
    src: &'s str,
    iter: Chars<'s>,
    index: usize,
}

impl<'s> Scanner<'s> {
    /// Create a new char scanner.
    pub fn new(src: &'s str) -> Self {
        Self { src, iter: src.chars(), index: 0 }
    }

    /// Consume the next char.
    pub fn eat(&mut self) -> Option<char> {
        let next = self.iter.next();
        if let Some(c) = next {
            self.index += c.len_utf8();
        }
        next
    }

    /// Consume the next char if it is the given one.
    ///
    /// Returns whether the char was consumed.
    pub fn eat_if(&mut self, c: char) -> bool {
        // Don't decode the char twice through peek() and eat().
        if self.iter.next() == Some(c) {
            self.index += c.len_utf8();
            true
        } else {
            self.reset();
            false
        }
    }

    /// Consume the next char, debug-asserting that it is the given one.
    pub fn eat_assert(&mut self, c: char) {
        let next = self.eat();
        debug_assert_eq!(next, Some(c));
    }

    /// Consume the next char, coalescing `\r\n` to just `\n`.
    pub fn eat_merging_crlf(&mut self) -> Option<char> {
        let c = self.eat();
        if c == Some('\r') && self.eat_if('\n') {
            Some('\n')
        } else {
            c
        }
    }

    /// Eat chars while the condition is true.
    pub fn eat_while(&mut self, mut f: impl FnMut(char) -> bool) -> &'s str {
        self.eat_until(|c| !f(c))
    }

    /// Eat chars until the condition is true.
    pub fn eat_until(&mut self, mut f: impl FnMut(char) -> bool) -> &'s str {
        let start = self.index;
        while let Some(c) = self.iter.next() {
            if f(c) {
                // Undo the previous `next()` without peeking all the time
                // during iteration.
                self.reset();
                break;
            }
            self.index += c.len_utf8();
        }
        &self.src[start .. self.index]
    }

    /// Uneat the last eaten char.
    pub fn uneat(&mut self) {
        self.index = self.last_index();
        self.reset();
    }

    /// Peek at the next char without consuming it.
    pub fn peek(&self) -> Option<char> {
        self.iter.clone().next()
    }

    /// Peek at the nth-next char without consuming anything.
    pub fn peek_nth(&self, n: usize) -> Option<char> {
        self.iter.clone().nth(n)
    }

    /// Checks whether the next char fulfills a condition.
    ///
    /// Returns `false` if there is no next char.
    pub fn check(&self, f: impl FnOnce(char) -> bool) -> bool {
        self.peek().map(f).unwrap_or(false)
    }

    /// Whether the end of the source string is reached.
    pub fn eof(&self) -> bool {
        self.iter.as_str().is_empty()
    }

    /// The previous index in the source string.
    pub fn last_index(&self) -> usize {
        self.src[.. self.index]
            .chars()
            .next_back()
            .map(|c| self.index - c.len_utf8())
            .unwrap_or(0)
    }

    /// The current index in the source string.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Jump to an index in the source string.
    pub fn jump(&mut self, index: usize) {
        self.index = index;
        self.reset();
    }

    /// The full source string.
    pub fn src(&self) -> &'s str {
        self.src
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
        &self.src[.. self.index]
    }

    /// The source string from `start` to the current index.
    pub fn eaten_from(&self, start: usize) -> &'s str {
        &self.src[start .. self.index]
    }

    /// The remaining source string after the current index.
    pub fn rest(&self) -> &'s str {
        &self.src[self.index ..]
    }

    /// Go back to the where the index says.
    fn reset(&mut self) {
        self.iter = self.src[self.index ..].chars();
    }
}

impl Debug for Scanner<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Scanner({}|{})", self.eaten(), self.rest())
    }
}
