use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, Range};

use serde::{Deserialize, Serialize};

use crate::source::SourceId;

/// A value with the span it corresponds to in the source code.
#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Span {
    /// The id of the source file.
    pub source: SourceId,
    /// The inclusive start position.
    pub start: Pos,
    /// The inclusive end position.
    pub end: Pos,
}

impl Span {
    /// Create a new span from start and end positions.
    pub fn new(source: SourceId, start: impl Into<Pos>, end: impl Into<Pos>) -> Self {
        Self {
            source,
            start: start.into(),
            end: end.into(),
        }
    }

    /// Create a span including just a single position.
    pub fn at(source: SourceId, pos: impl Into<Pos> + Copy) -> Self {
        Self::new(source, pos, pos)
    }

    /// Create a span without real location information, usually for testing.
    pub fn detached() -> Self {
        Self {
            source: SourceId::from_raw(0),
            start: Pos::ZERO,
            end: Pos::ZERO,
        }
    }

    /// Create a span with a different start position.
    pub fn with_start(self, start: impl Into<Pos>) -> Self {
        Self { start: start.into(), ..self }
    }

    /// Create a span with a different end position.
    pub fn with_end(self, end: impl Into<Pos>) -> Self {
        Self { end: end.into(), ..self }
    }

    /// Create a new span with the earlier start and later end position.
    ///
    /// This panics if the spans come from different files.
    pub fn join(self, other: Self) -> Self {
        debug_assert_eq!(self.source, other.source);
        Self {
            source: self.source,
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
        self.source == other.source && self.start <= other.start && self.end >= other.end
    }

    /// Convert to a `Range<Pos>` for indexing.
    pub fn to_range(self) -> Range<usize> {
        self.start.to_usize() .. self.end.to_usize()
    }
}

impl Debug for Span {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}-{:?}", self.start, self.end)
    }
}

impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.source == other.source {
            Some(self.start.cmp(&other.start).then(self.end.cmp(&other.end)))
        } else {
            None
        }
    }
}

/// A byte position in source code.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Pos(pub u32);

impl Pos {
    /// The zero position.
    pub const ZERO: Self = Self(0);

    /// Convert to a usize for indexing.
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

impl Debug for Pos {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.0, f)
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

impl<T> Add<T> for Pos
where
    T: Into<Pos>,
{
    type Output = Self;

    fn add(self, rhs: T) -> Self {
        Pos(self.0 + rhs.into().0)
    }
}

/// Convert a position or range into a span.
pub trait IntoSpan {
    /// Convert into a span by providing the source id.
    fn into_span(self, source: SourceId) -> Span;
}

impl IntoSpan for Span {
    fn into_span(self, source: SourceId) -> Span {
        debug_assert_eq!(self.source, source);
        self
    }
}

impl IntoSpan for Pos {
    fn into_span(self, source: SourceId) -> Span {
        Span::new(source, self, self)
    }
}

impl IntoSpan for usize {
    fn into_span(self, source: SourceId) -> Span {
        Span::new(source, self, self)
    }
}

impl IntoSpan for Range<usize> {
    fn into_span(self, source: SourceId) -> Span {
        Span::new(source, self.start, self.end)
    }
}
