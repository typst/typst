use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroU64;
use std::ops::Range;

use super::SourceId;

/// A unique identifier for a syntax node.
///
/// This is used throughout the compiler to track which source section an error
/// or element stems from. Can be [mapped back](super::Source::range) to a byte
/// range for user facing display.
///
/// During editing, the span values stay mostly stable, even for nodes behind an
/// insertion. This is not true for simple ranges as they would shift. Spans can
/// be used as inputs to memoized functions without hurting cache performance
/// when text is inserted somewhere in the document other than the end.
///
/// Span ids are ordered in the syntax tree to enable quickly finding the node
/// with some id:
/// - The id of a parent is always smaller than the ids of any of its children.
/// - The id of a node is always greater than any id in the subtrees of any left
///   sibling and smaller than any id in the subtrees of any right sibling.
///
/// This type takes up 8 bytes and is null-optimized (i.e. `Option<Span>` also
/// takes 8 bytes).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Span(NonZeroU64);

impl Span {
    /// The full range of numbers available for span numbering.
    pub const FULL: Range<u64> = 2..(1 << Self::BITS);
    const DETACHED: u64 = 1;

    // Data layout:
    // | 16 bits source id | 48 bits number |
    const BITS: usize = 48;

    /// Create a new span from a source id and a unique number.
    ///
    /// Panics if the `number` is not contained in `FULL`.
    #[track_caller]
    pub const fn new(id: SourceId, number: u64) -> Self {
        assert!(
            Self::FULL.start <= number && number < Self::FULL.end,
            "span number outside valid range"
        );

        Self::pack(id, number)
    }

    /// A span that does not point into any source file.
    pub const fn detached() -> Self {
        Self::pack(SourceId::detached(), Self::DETACHED)
    }

    /// Pack the components into a span.
    #[track_caller]
    const fn pack(id: SourceId, number: u64) -> Span {
        let bits = ((id.as_u16() as u64) << Self::BITS) | number;
        match NonZeroU64::new(bits) {
            Some(v) => Self(v),
            None => panic!("span encoding is zero"),
        }
    }

    /// Whether the span is detached.
    pub const fn is_detached(self) -> bool {
        self.source().is_detached()
    }

    /// The id of the source file the span points into.
    pub const fn source(self) -> SourceId {
        SourceId::from_u16((self.0.get() >> Self::BITS) as u16)
    }

    /// The unique number of the span within its source file.
    pub const fn number(self) -> u64 {
        self.0.get() & ((1 << Self::BITS) - 1)
    }
}

/// A value with a span locating it in the source code.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Spanned<T> {
    /// The spanned value.
    pub v: T,
    /// The value's location in source code.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Create a new instance from a value and its span.
    pub fn new(v: T, span: Span) -> Self {
        Self { v, span }
    }

    /// Convert from `&Spanned<T>` to `Spanned<&T>`
    pub fn as_ref(&self) -> Spanned<&T> {
        Spanned { v: &self.v, span: self.span }
    }

    /// Map the value using a function.
    pub fn map<F, U>(self, f: F) -> Spanned<U>
    where
        F: FnOnce(T) -> U,
    {
        Spanned { v: f(self.v), span: self.span }
    }
}

impl<T: Debug> Debug for Spanned<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.v.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::{SourceId, Span};

    #[test]
    fn test_span_encoding() {
        let id = SourceId::from_u16(5);
        let span = Span::new(id, 10);
        assert_eq!(span.source(), id);
        assert_eq!(span.number(), 10);
    }
}
