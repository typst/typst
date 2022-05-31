use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroU64;

use crate::syntax::SourceId;

/// A value with the span it corresponds to in the source code.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
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
        self.v.fmt(f)
    }
}

/// A unique identifier for a syntax node.
///
/// This is used throughout the compiler to track which source section an error
/// or element stems from. Can be mapped back to a source id + byte range for
/// user facing display.
///
/// Node ids are ordered in the tree to enable quickly finding the node with
/// some id:
/// - The id of a parent is always smaller than the ids of any of its children.
/// - The id of a node is always greater than any id in the subtrees of any left
///   sibling and smaller than any id the subtrees of any right sibling.
///
/// Node ids stay mostly stable, even for nodes behind an insertion. This is not
/// true for simple spans/ranges as they shift. Node ids can be used as inputs
/// to memoized functions without hurting cache performance when text is
/// inserted somewhere in the document other than the end.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Span(NonZeroU64);

impl Span {
    /// Create a new span from a source id and a unique number.
    pub const fn new(id: SourceId, number: u64) -> Self {
        assert!(number > 0 && number < (1 << 48));
        let bits = ((id.into_raw() as u64) << 48) | number;
        Self(nonzero(bits))
    }

    /// A node that does not belong to any source file.
    pub const fn detached() -> Self {
        Self(nonzero(1))
    }

    /// The id of the source file the span points into.
    pub const fn source(self) -> SourceId {
        SourceId::from_raw((self.0.get() >> 48) as u16)
    }
}

/// Convert to a non zero u64.
const fn nonzero(v: u64) -> NonZeroU64 {
    match NonZeroU64::new(v) {
        Some(v) => v,
        None => unreachable!(),
    }
}
