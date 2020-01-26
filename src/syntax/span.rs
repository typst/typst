//! Spans map elements to the part of source code they originate from.

use std::fmt::{self, Debug, Display, Formatter};
use std::ops::{Add, Sub};
use serde::Serialize;


/// Zero-indexed line-column position in source code.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct Position {
    /// The zero-indexed line.
    pub line: usize,
    /// The zero-indexed column.
    pub column: usize,
}

impl Position {
    /// The line 0, column 0 position.
    pub const ZERO: Position = Position { line: 0, column: 0 };

    /// Crete a new instance from line and column.
    pub fn new(line: usize, column: usize) -> Position {
        Position { line, column }
    }
}

impl Add for Position {
    type Output = Position;

    fn add(self, rhs: Position) -> Position {
        if rhs.line == 0 {
            Position {
                line: self.line,
                column: self.column + rhs.column
            }
        } else {
            Position {
                line: self.line + rhs.line,
                column: rhs.column,
            }
        }
    }
}

impl Sub for Position {
    type Output = Position;

    fn sub(self, rhs: Position) -> Position {
        if self.line == rhs.line {
            Position {
                line: 0,
                column: self.column - rhs.column
            }
        } else {
            Position {
                line: self.line - rhs.line,
                column: self.column,
            }
        }
    }
}

/// Locates a slice of source code.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize)]
pub struct Span {
    /// The inclusive start position.
    pub start: Position,
    /// The inclusive end position.
    pub end: Position,
}

impl Span {
    /// A dummy span.
    pub const ZERO: Span = Span { start: Position::ZERO, end: Position::ZERO };

    /// Create a new span from start and end positions.
    pub fn new(start: Position, end: Position) -> Span {
        Span { start, end }
    }

    /// Create a new span with the earlier start and later end position.
    pub fn merge(a: Span, b: Span) -> Span {
        Span {
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }

    /// Create a span including just a single position.
    pub fn at(pos: Position) -> Span {
        Span { start: pos, end: pos }
    }

    /// Offset a span by a start position.
    ///
    /// This is, for example, used to translate error spans from function local
    /// to global.
    pub fn offset(self, start: Position) -> Span {
        Span {
            start: start + self.start,
            end: start + self.end,
        }
    }
}

/// A value with the span it corresponds to in the source code.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize)]
pub struct Spanned<T> {
    /// The value.
    pub v: T,
    /// The corresponding span.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Create a new instance from a value and its span.
    pub fn new(v: T, span: Span) -> Spanned<T> {
        Spanned { v, span }
    }

    /// Access the value.
    pub fn value(self) -> T {
        self.v
    }

    /// Map the value using a function while keeping the span.
    pub fn map<V, F>(self, f: F) -> Spanned<V> where F: FnOnce(T) -> V {
        Spanned { v: f(self.v), span: self.span }
    }

    /// Maps the span while keeping the value.
    pub fn map_span<F>(mut self, f: F) -> Spanned<T> where F: FnOnce(Span) -> Span {
        self.span = f(self.span);
        self
    }
}

/// A vector of spanned things.
pub type SpanVec<T> = Vec<Spanned<T>>;

/// [Offset](Span::offset) all spans in a vector of spanned things by a start
/// position.
pub fn offset_spans<T>(vec: SpanVec<T>, start: Position) -> impl Iterator<Item=Spanned<T>> {
    vec.into_iter().map(move |s| s.map_span(|span| span.offset(start)))
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.start, self.end)
    }
}

impl<T> Display for Spanned<T> where T: std::fmt::Display {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}, {}, ", self.span.start, self.span.end)?;
        self.v.fmt(f)?;
        write!(f, ")")
    }
}

impl<T> Debug for Spanned<T> where T: std::fmt::Debug {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}, {}, ", self.span.start, self.span.end)?;
        self.v.fmt(f)?;
        write!(f, ")")
    }
}

debug_display!(Position);
debug_display!(Span);
