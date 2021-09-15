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
    #[inline]
    pub fn new(src: &'s str) -> Self {
        Self { src, index: 0 }
    }

    /// Consume the next char.
    #[inline]
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
    #[inline]
    pub fn eat_if(&mut self, c: char) -> bool {
        let matches = self.peek() == Some(c);
        if matches {
            self.index += c.len_utf8();
        }
        matches
    }

    /// Consume the next char, debug-asserting that it is the given one.
    #[inline]
    pub fn eat_assert(&mut self, c: char) {
        let next = self.eat();
        debug_assert_eq!(next, Some(c));
    }

    /// Eat chars while the condition is true.
    #[inline]
    pub fn eat_while<F>(&mut self, mut f: F) -> &'s str
    where
        F: FnMut(char) -> bool,
    {
        self.eat_until(|c| !f(c))
    }

    /// Eat chars until the condition is true.
    #[inline]
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
    #[inline]
    pub fn uneat(&mut self) {
        self.index = self.last_index();
    }

    /// Peek at the next char without consuming it.
    #[inline]
    pub fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    /// Checks whether the next char fulfills a condition.
    ///
    /// Returns `default` if there is no next char.
    #[inline]
    pub fn check_or<F>(&self, default: bool, f: F) -> bool
    where
        F: FnOnce(char) -> bool,
    {
        self.peek().map_or(default, f)
    }

    /// The previous index in the source string.
    #[inline]
    pub fn last_index(&self) -> usize {
        self.eaten().chars().last().map_or(0, |c| self.index - c.len_utf8())
    }

    /// The current index in the source string.
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    /// Jump to an index in the source string.
    #[inline]
    pub fn jump(&mut self, index: usize) {
        // Make sure that the index is in bounds and on a codepoint boundary.
        self.src.get(index ..).expect("jumped to invalid index");
        self.index = index;
    }

    /// Slice out part of the source string.
    #[inline]
    pub fn get<I>(&self, index: I) -> &'s str
    where
        I: SliceIndex<str, Output = str>,
    {
        // See `eaten_from` for details about `unwrap_or_default`.
        self.src.get(index).unwrap_or_default()
    }

    /// The remaining source string after the current index.
    #[inline]
    pub fn rest(&self) -> &'s str {
        // Safety: The index is always in bounds and on a codepoint boundary
        // since it starts at zero and is is:
        // - either increased by the length of a scanned character, advacing
        //   from one codepoint boundary to the next,
        // - or checked upon jumping.
        unsafe { self.src.get_unchecked(self.index ..) }
    }

    /// The full source string up to the current index.
    #[inline]
    pub fn eaten(&self) -> &'s str {
        // Safety: The index is always okay, for details see `rest()`.
        unsafe { self.src.get_unchecked(.. self.index) }
    }

    /// The source string from `start` to the current index.
    #[inline]
    pub fn eaten_from(&self, start: usize) -> &'s str {
        // Using `unwrap_or_default` is much faster than unwrap, probably
        // because then the whole call to `eaten_from` is pure and can be
        // optimized away in some cases.
        self.src.get(start .. self.index).unwrap_or_default()
    }
}
