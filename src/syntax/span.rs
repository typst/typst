//! Spans map elements to the part of source code they originate from.

use std::fmt::{self, Debug, Display, Formatter};
use std::ops::{Add, AddAssign};
use serde::Serialize;


/// Annotates a value with the part of the source code it corresponds to.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize)]
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

/// Describes a slice of source code.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize)]
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
        write!(f, "({}, {})", self.start, self.end)
    }
}

debug_display!(Span);

/// A line-column position in source code.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
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

impl AddAssign for Position {
    fn add_assign(&mut self, other: Position) {
        *self = *self + other;
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

debug_display!(Position);

/// A vector of spanned things.
pub type SpanVec<T> = Vec<Spanned<T>>;

pub fn offset_spans<T>(
    vec: SpanVec<T>,
    start: Position,
) -> impl Iterator<Item=Spanned<T>> {
    vec.into_iter()
        .map(move |mut spanned| {
            spanned.span.start += start;
            spanned.span.end += start;
            spanned
        })
}
