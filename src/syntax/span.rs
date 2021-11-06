use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::ops::Range;

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
    pub start: usize,
    /// The inclusive end position.
    pub end: usize,
}

impl Span {
    /// Create a new span from start and end positions.
    pub fn new(source: SourceId, start: usize, end: usize) -> Self {
        Self { source, start, end }
    }

    /// Create a span including just a single position.
    pub fn at(source: SourceId, pos: usize) -> Self {
        Self::new(source, pos, pos)
    }

    /// Create a span without real location information, usually for testing.
    pub fn detached() -> Self {
        Self {
            source: SourceId::from_raw(0),
            start: 0,
            end: 0,
        }
    }

    /// Create a span with a different start position.
    pub fn with_start(self, start: usize) -> Self {
        Self { start, ..self }
    }

    /// Create a span with a different end position.
    pub fn with_end(self, end: usize) -> Self {
        Self { end, ..self }
    }

    /// Whether the span is a single point.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// The byte length of the spanned region.
    pub fn len(self) -> usize {
        self.end - self.start
    }

    /// A new span at the position of this span's start.
    pub fn at_start(&self) -> Span {
        Self::at(self.source, self.start)
    }

    /// A new span at the position of this span's end.
    pub fn at_end(&self) -> Span {
        Self::at(self.source, self.end)
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

    /// Create a new span by specifying a span in which a modification happened
    /// and how many characters are now in that span.
    pub fn inserted(mut self, other: Self, n: usize) -> Self {
        if !self.surrounds(other) {
            panic!();
        }

        let len_change = n as isize - other.len() as isize;
        self.end += len_change as usize;
        self
    }

    /// Test whether a position is within the span.
    pub fn contains(&self, pos: usize) -> bool {
        self.start <= pos && self.end >= pos
    }

    /// Test whether one span complete contains the other span.
    pub fn surrounds(self, other: Self) -> bool {
        self.source == other.source && self.start <= other.start && self.end >= other.end
    }

    /// Convert to a `Range<usize>` for indexing.
    pub fn to_range(self) -> Range<usize> {
        self.start .. self.end
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
