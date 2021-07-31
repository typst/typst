use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, Range};

use serde::{Deserialize, Serialize};

/// A value with the span it corresponds to in the source code.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[derive(Serialize, Deserialize)]
pub struct Spanned<T> {
    /// The spanned value.
    pub v: T,
    /// The location in source code of the value.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Create a new instance from a value and its span.
    pub fn new(v: T, span: impl Into<Span>) -> Self {
        Self { v, span: span.into() }
    }

    /// Create a new instance from a value with the zero span.
    pub fn zero(v: T) -> Self {
        Self { v, span: Span::ZERO }
    }

    /// Convert from `&Spanned<T>` to `Spanned<&T>`
    pub fn as_ref(&self) -> Spanned<&T> {
        Spanned { v: &self.v, span: self.span }
    }

    /// Map the value using a function keeping the span.
    pub fn map<F, U>(self, f: F) -> Spanned<U>
    where
        F: FnOnce(T) -> U,
    {
        Spanned { v: f(self.v), span: self.span }
    }
}

impl<T: Debug> Debug for Spanned<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.v.fmt(f)?;
        if f.alternate() {
            f.write_str(" <")?;
            self.span.fmt(f)?;
            f.write_str(">")?;
        }
        Ok(())
    }
}

/// Bounds of a slice of source code.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[derive(Serialize, Deserialize)]
pub struct Span {
    /// The inclusive start position.
    pub start: Pos,
    /// The inclusive end position.
    pub end: Pos,
}

impl Span {
    /// The zero span.
    pub const ZERO: Self = Self { start: Pos::ZERO, end: Pos::ZERO };

    /// Create a new span from start and end positions.
    pub fn new(start: impl Into<Pos>, end: impl Into<Pos>) -> Self {
        Self { start: start.into(), end: end.into() }
    }

    /// Create a span including just a single position.
    pub fn at(pos: impl Into<Pos> + Copy) -> Self {
        Self::new(pos, pos)
    }

    /// Create a new span with the earlier start and later end position.
    pub fn join(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Expand a span by merging it with another span.
    pub fn expand(&mut self, other: Self) {
        *self = self.join(other)
    }

    /// Test whether one span complete contains the other span.
    pub fn contains(self, other: Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }

    /// Convert to a `Range<usize>` for indexing.
    pub fn to_range(self) -> Range<usize> {
        self.start.to_usize() .. self.end.to_usize()
    }
}

impl<T> From<T> for Span
where
    T: Into<Pos> + Copy,
{
    fn from(pos: T) -> Self {
        Self::at(pos)
    }
}

impl<T> From<Range<T>> for Span
where
    T: Into<Pos>,
{
    fn from(range: Range<T>) -> Self {
        Self::new(range.start, range.end)
    }
}

impl Debug for Span {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}-{:?}", self.start, self.end)
    }
}

/// A byte position in source code.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[derive(Serialize, Deserialize)]
pub struct Pos(pub u32);

impl Pos {
    /// The zero position.
    pub const ZERO: Self = Self(0);

    /// Convert to a usize for indexing.
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for Pos {
    fn from(index: u32) -> Self {
        Self(index)
    }
}

impl From<usize> for Pos {
    fn from(index: usize) -> Self {
        Self(index as u32)
    }
}

impl Debug for Pos {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<T> Add<T> for Pos
where
    T: Into<Pos>,
{
    type Output = Self;

    fn add(self, rhs: T) -> Self {
        Pos(self.0 + rhs.into().0)
    }
}
