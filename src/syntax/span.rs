//! Spans map elements to the part of source code they originate from.

use std::fmt::{self, Display, Formatter};


/// Annotates a value with the part of the source code it corresponds to.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Spanned<T> {
    pub v: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(v: T, span: Span) -> Spanned<T> {
        Spanned { v, span }
    }

    pub fn value(self) -> T {
        self.v
    }

    pub fn map<F, V>(self, f: F) -> Spanned<V> where F: FnOnce(T) -> V {
        Spanned { v: f(self.v), span: self.span }
    }

    pub fn map_v<V>(&self, new_v: V) -> Spanned<V> {
        Spanned { v: new_v, span: self.span }
    }
}

impl<T> Display for Spanned<T> where T: std::fmt::Debug {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({:?}:{})", self.v, self.span)
    }
}

debug_display!(Spanned; T where T: std::fmt::Debug);

/// Describes a slice of source code.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub const ZERO: Span = Span { start: Position::ZERO, end: Position::ZERO };

    pub fn new(start: Position, end: Position) -> Span {
        Span { start, end }
    }

    pub fn merge(a: Span, b: Span) -> Span {
        Span {
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }

    pub fn at(pos: Position) -> Span {
        Span { start: pos, end: pos }
    }

    pub fn expand(&mut self, other: Span) {
        *self = Span::merge(*self, other)
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[{}, {}]", self.start, self.end)
    }
}

debug_display!(Span);

/// A line-column position in source code.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Position {
    /// The 0-indexed line (inclusive).
    pub line: usize,
    /// The 0-indexed column (inclusive).
    pub column: usize,
}

impl Position {
    pub const ZERO: Position = Position { line: 0, column: 0 };

    pub fn new(line: usize, column: usize) -> Position {
        Position { line, column }
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

debug_display!(Position);
