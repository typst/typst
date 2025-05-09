use std::fmt::{self, Debug, Formatter};
use std::num::{NonZeroU16, NonZeroU64};
use std::ops::Range;

use ecow::EcoString;

use crate::FileId;

/// Defines a range in a file.
///
/// This is used throughout the compiler to track which source section an
/// element stems from or an error applies to.
///
/// - The [`.id()`](Self::id) function can be used to get the `FileId` for the
///   span and, by extension, its file system path.
/// - The `WorldExt::range` function can be used to map the span to a
///   `Range<usize>`.
///
/// This type takes up 8 bytes and is copyable and null-optimized (i.e.
/// `Option<Span>` also takes 8 bytes).
///
/// Spans come in two flavors: Numbered spans and raw range spans. The
/// `WorldExt::range` function automatically handles both cases, yielding a
/// `Range<usize>`.
///
/// # Numbered spans
/// Typst source files use _numbered spans._ Rather than using byte ranges,
/// which shift a lot as you type, each AST node gets a unique number.
///
/// During editing, the span numbers stay mostly stable, even for nodes behind
/// an insertion. This is not true for simple ranges as they would shift. Spans
/// can be used as inputs to memoized functions without hurting cache
/// performance when text is inserted somewhere in the document other than the
/// end.
///
/// Span ids are ordered in the syntax tree to enable quickly finding the node
/// with some id:
/// - The id of a parent is always smaller than the ids of any of its children.
/// - The id of a node is always greater than any id in the subtrees of any left
///   sibling and smaller than any id in the subtrees of any right sibling.
///
/// # Raw range spans
/// Non Typst-files use raw ranges instead of numbered spans. The maximum
/// encodable value for start and end is 2^23. Larger values will be saturated.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Span(NonZeroU64);

impl Span {
    /// The full range of numbers available for source file span numbering.
    pub(crate) const FULL: Range<u64> = 2..(1 << 47);

    /// The value reserved for the detached span.
    const DETACHED: u64 = 1;

    /// Data layout:
    /// | 16 bits file id | 48 bits number |
    ///
    /// Number =
    /// - 1 means detached
    /// - 2..2^47-1 is a numbered span
    /// - 2^47..2^48-1 is a raw range span. To retrieve it, you must subtract
    ///   `RANGE_BASE` and then use shifting/bitmasking to extract the
    ///   components.
    const NUMBER_BITS: usize = 48;
    const FILE_ID_SHIFT: usize = Self::NUMBER_BITS;
    const NUMBER_MASK: u64 = (1 << Self::NUMBER_BITS) - 1;
    const RANGE_BASE: u64 = Self::FULL.end;
    const RANGE_PART_BITS: usize = 23;
    const RANGE_PART_SHIFT: usize = Self::RANGE_PART_BITS;
    const RANGE_PART_MASK: u64 = (1 << Self::RANGE_PART_BITS) - 1;

    /// Create a span that does not point into any file.
    pub const fn detached() -> Self {
        Self(NonZeroU64::new(Self::DETACHED).unwrap())
    }

    /// Create a new span from a file id and a number.
    ///
    /// Returns `None` if `number` is not contained in `FULL`.
    pub(crate) const fn from_number(id: FileId, number: u64) -> Option<Self> {
        if number < Self::FULL.start || number >= Self::FULL.end {
            return None;
        }
        Some(Self::pack(id, number))
    }

    /// Create a new span from a raw byte range instead of a span number.
    ///
    /// If one of the range's parts exceeds the maximum value (2^23), it is
    /// saturated.
    pub const fn from_range(id: FileId, range: Range<usize>) -> Self {
        let max = 1 << Self::RANGE_PART_BITS;
        let start = if range.start > max { max } else { range.start } as u64;
        let end = if range.end > max { max } else { range.end } as u64;
        let number = (start << Self::RANGE_PART_SHIFT) | end;
        Self::pack(id, Self::RANGE_BASE + number)
    }

    /// Construct from a raw number.
    ///
    /// Should only be used with numbers retrieved via
    /// [`into_raw`](Self::into_raw). Misuse may results in panics, but no
    /// unsafety.
    pub const fn from_raw(v: NonZeroU64) -> Self {
        Self(v)
    }

    /// Pack a file ID and the low bits into a span.
    const fn pack(id: FileId, low: u64) -> Self {
        let bits = ((id.into_raw().get() as u64) << Self::FILE_ID_SHIFT) | low;

        // The file ID is non-zero.
        Self(NonZeroU64::new(bits).unwrap())
    }

    /// Whether the span is detached.
    pub const fn is_detached(self) -> bool {
        self.0.get() == Self::DETACHED
    }

    /// The id of the file the span points into.
    ///
    /// Returns `None` if the span is detached.
    pub const fn id(self) -> Option<FileId> {
        // Detached span has only zero high bits, so it will trigger the
        // `None` case.
        match NonZeroU16::new((self.0.get() >> Self::FILE_ID_SHIFT) as u16) {
            Some(v) => Some(FileId::from_raw(v)),
            None => None,
        }
    }

    /// The unique number of the span within its [`Source`](crate::Source).
    pub(crate) const fn number(self) -> u64 {
        self.0.get() & Self::NUMBER_MASK
    }

    /// Extract a raw byte range from the span, if it is a raw range span.
    ///
    /// Typically, you should use `WorldExt::range` instead.
    pub const fn range(self) -> Option<Range<usize>> {
        let Some(number) = self.number().checked_sub(Self::RANGE_BASE) else {
            return None;
        };

        let start = (number >> Self::RANGE_PART_SHIFT) as usize;
        let end = (number & Self::RANGE_PART_MASK) as usize;
        Some(start..end)
    }

    /// Extract the raw underlying number.
    pub const fn into_raw(self) -> NonZeroU64 {
        self.0
    }

    /// Return `other` if `self` is detached and `self` otherwise.
    pub fn or(self, other: Self) -> Self {
        if self.is_detached() {
            other
        } else {
            self
        }
    }

    /// Find the first non-detached span in the iterator.
    pub fn find(iter: impl IntoIterator<Item = Self>) -> Self {
        iter.into_iter()
            .find(|span| !span.is_detached())
            .unwrap_or(Span::detached())
    }

    /// Resolve a file location relative to this span's source.
    pub fn resolve_path(self, path: &str) -> Result<FileId, EcoString> {
        let Some(file) = self.id() else {
            return Err("cannot access file system from here".into());
        };
        Ok(file.join(path))
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
    use std::num::NonZeroU16;
    use std::ops::Range;

    use crate::{FileId, Span};

    #[test]
    fn test_span_detached() {
        let span = Span::detached();
        assert!(span.is_detached());
        assert_eq!(span.id(), None);
        assert_eq!(span.range(), None);
    }

    #[test]
    fn test_span_number_encoding() {
        let id = FileId::from_raw(NonZeroU16::new(5).unwrap());
        let span = Span::from_number(id, 10).unwrap();
        assert_eq!(span.id(), Some(id));
        assert_eq!(span.number(), 10);
        assert_eq!(span.range(), None);
    }

    #[test]
    fn test_span_range_encoding() {
        let id = FileId::from_raw(NonZeroU16::new(u16::MAX).unwrap());
        let roundtrip = |range: Range<usize>| {
            let span = Span::from_range(id, range.clone());
            assert_eq!(span.id(), Some(id));
            assert_eq!(span.range(), Some(range));
        };

        roundtrip(0..0);
        roundtrip(177..233);
        roundtrip(0..8388607);
        roundtrip(8388606..8388607);
    }
}
