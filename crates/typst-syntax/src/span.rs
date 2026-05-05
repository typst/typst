use std::fmt::{self, Debug, Formatter};
use std::num::{NonZeroU16, NonZeroU64};
use std::ops::Range;

use ecow::{EcoString, eco_format};

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
/// Span numbers are ordered in the syntax tree to enable quickly finding the
/// node of a known span:
/// - The span number of a parent node is always smaller than the number of any
///   of its children
/// - The span numbers of sibling nodes always increase from left to right
///
/// Combining those guarantees, we have that for siblings in order [A, B, C],
/// the span numbers for node A and _all of A's children_ are less than node B's
/// span number, and the numbers for node C and all of C's children are greater
/// than B's span number.
///
/// # Raw range spans
/// Non Typst-files use raw ranges instead of numbered spans. The maximum
/// encodable value for start and end is 2^23-1. Larger values will be
/// saturated.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Span(NonZeroU64);

/// The unique number of a span within its [`Source`](crate::Source). Known to
/// be within the range of `Span::FULL`.
///
/// This is mainly used externally as an input to the
/// [`Source::range`](crate::Source::range) method for efficiently finding the
/// byte range of a span.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SpanNumber(pub(crate) u64);

/// The possible kinds of span.
#[derive(Debug)]
pub enum SpanKind {
    /// A span that does not point into any file.
    Detached,
    /// A numbered span.
    Number { id: FileId, num: SpanNumber },
    /// A raw byte range in a file.
    Range { id: FileId, range: Range<usize> },
}

impl Span {
    /// The full range of numbers available for source file span numbering.
    pub(crate) const FULL: Range<u64> = 2..(1 << 47);

    /// The value reserved for the detached span.
    const DETACHED: Self = Self(NonZeroU64::new(1).unwrap());

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
    const RANGE_VALUE_BITS: usize = 23;
    const RANGE_VALUE_MAX: u64 = (1 << Self::RANGE_VALUE_BITS) - 1;

    /// Create a span that does not point into any file.
    pub const fn detached() -> Self {
        Self::DETACHED
    }

    /// Create a new span from a [`FileId`] and a [`SpanNumber`].
    pub(crate) const fn from_number(id: FileId, SpanNumber(number): SpanNumber) -> Self {
        debug_assert!(Self::FULL.start <= number);
        debug_assert!(number < Self::FULL.end);
        Self::pack(id, number)
    }

    /// Create a new span from a raw byte range instead of a span number.
    ///
    /// If one of the range's parts exceeds the maximum value of `2^23-1`, it is
    /// saturated.
    pub const fn from_range(id: FileId, range: Range<usize>) -> Self {
        let start = saturate(range.start, Self::RANGE_VALUE_MAX);
        let end = saturate(range.end, Self::RANGE_VALUE_MAX);
        let number = (start << Self::RANGE_VALUE_BITS) | end;
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
        self.0.get() == Self::DETACHED.0.get()
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

    /// Unpack the span into the variants of a [`SpanKind`] for easier use.
    ///
    /// To access a range, you may want to use `WorldExt::range` instead.
    pub const fn get(self) -> SpanKind {
        let Some(id) = self.id() else { return SpanKind::Detached };
        let num = self.number();
        if let Some(packed_range) = num.checked_sub(Self::RANGE_BASE) {
            let start = (packed_range >> Self::RANGE_VALUE_BITS) as usize;
            let end = (packed_range & Self::RANGE_VALUE_MAX) as usize;
            SpanKind::Range { id, range: start..end }
        } else {
            SpanKind::Number { id, num: SpanNumber(num) }
        }
    }

    /// Extract the raw underlying number.
    pub const fn into_raw(self) -> NonZeroU64 {
        self.0
    }

    /// Return `other` if `self` is detached and `self` otherwise.
    pub fn or(self, other: Self) -> Self {
        if self.is_detached() { other } else { self }
    }

    /// Find the first non-detached span in the iterator.
    pub fn find(iter: impl IntoIterator<Item = Self>) -> Self {
        iter.into_iter()
            .find(|span| !span.is_detached())
            .unwrap_or(Span::detached())
    }
}

/// Saturate a value at a given maximum. Can't use `.min()` since it isn't
/// stable in const :/
const fn saturate(value: usize, max: u64) -> u64 {
    if value as u64 > max { max } else { value as u64 }
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
    pub const fn new(v: T, span: Span) -> Self {
        Self { v, span }
    }

    /// Create a new instance with a span that does not point into any file.
    pub const fn detached(v: T) -> Self {
        Self { v, span: Span::detached() }
    }

    /// Convert from `&Spanned<T>` to `Spanned<&T>`
    pub const fn as_ref(&self) -> Spanned<&T> {
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

/// Remaps ranges.
///
/// Useful in combination with
/// [`SyntaxNode::synthesize_mapped`](super::SyntaxNode::synthesize_mapped) to
/// have accurate error spans for source text that is non-consecutive in its
/// source file (for instance, Typst code in a doc comment with start-of-line
/// slashes).
#[derive(Hash)]
pub struct RangeMapper {
    vec: Vec<Mapping>,
    total: usize,
}

/// A mapping from an old index to a new one, guarantees that `old <= new`.
#[derive(Hash, Clone, Copy)]
struct Mapping {
    old: usize,
    new: usize,
}

impl RangeMapper {
    /// Creates a new range mapper.
    ///
    /// The iterator should returns ranges in the original text that will be
    /// consecutively concatenated to produce the derived text.
    ///
    /// Segments should be in order. (The start of a later range must not
    /// precede the end of an earlier range.)
    ///
    /// Note that this representation implies that ranges can only ever increase
    /// in their start position and length when mapped.
    pub fn new(
        segments: impl IntoIterator<Item = Range<usize>>,
    ) -> Result<Self, EcoString> {
        let mut map = Mapping { old: 0, new: 0 };
        let vec = segments
            .into_iter()
            .map(|Range { start, end }| {
                if start > end || map.new > start {
                    return Err(eco_format!("invalid mapper segment: ({start}, {end})"));
                }
                map.new = start;
                let segment_map = map;
                map.old += end - start;
                Ok(segment_map)
            })
            .collect::<Result<Vec<Mapping>, EcoString>>()?;

        if vec.is_empty() {
            Ok(Self { vec: vec![map], total: 0 })
        } else {
            Ok(Self { vec, total: map.old })
        }
    }

    /// The total length of the original text.
    pub(crate) fn total_len(&self) -> usize {
        self.total
    }

    /// Maps a range in the derived text back to a range in the original text.
    /// If the range spans over multiple segments, the gap between the two
    /// segments will be included in the resulting range.
    ///
    /// Input ranges must have  `start <= end`, and the caller should have
    /// verified that `end <= self.total`.
    pub(crate) fn map(&self, range: Range<usize>) -> Range<usize> {
        debug_assert!(range.start <= range.end);
        if range.end == 0 {
            // Handles the panic case of `map_end`.
            let offset = self.vec[0].new;
            offset..offset
        } else if range.start == range.end {
            // If start/end are at a boundary, map them to the first position,
            // not the second.
            let offset = self.map_end(range.start);
            offset..offset
        } else {
            // We now know that `start < end`, so the values from `map_start`
            // and `map_end` must be non-overlapping.
            let start = self.map_start(range.start);
            let end = self.map_end(range.end);
            start..end
        }
    }

    /// Map a single offset, preferring the second index if at a boundary.
    fn map_start(&self, offset: usize) -> usize {
        let idx = self.vec.partition_point(|&Mapping { old, new: _ }| old <= offset);
        // Subtracting by 1 is valid: vec is non-empty, index 0 has `old == 0`,
        // and `partition_point` returns the index of the first item to fail the
        // predicate (or the length), which is not index 0, since `0 <= usize`
        // is true for all usize.
        let Mapping { old, new } = &self.vec[idx - 1];
        new + (offset - old)
    }

    /// Map a single offset, preferring the first index if at a boundary.
    ///
    /// This will panic if `offset` is 0.
    fn map_end(&self, offset: usize) -> usize {
        debug_assert_ne!(offset, 0);
        let idx = self.vec.partition_point(|&Mapping { old, new: _ }| old < offset);
        // Unlike `map_start`, this can yield index 0 when `offset == 0`, making
        // `idx - 1` potentially panicking.
        let Mapping { old, new } = &self.vec[idx - 1];
        new + (offset - old)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_detached() {
        let span = Span::detached();
        assert!(span.is_detached());
        assert_eq!(span.id(), None);
    }

    #[test]
    fn test_span_number_encoding() {
        let id = FileId::from_raw(NonZeroU16::new(5).unwrap());
        let span = Span::from_number(id, SpanNumber(10));
        assert_eq!(span.id(), Some(id));
        assert_eq!(span.number(), 10);
    }

    #[test]
    fn test_span_range_encoding() {
        let file_id = FileId::from_raw(NonZeroU16::new(u16::MAX).unwrap());
        let roundtrip = |range: Range<usize>| {
            let span = Span::from_range(file_id, range.clone());
            let SpanKind::Range { id, range: actual } = span.get() else {
                panic!("bad span kind")
            };
            assert_eq!(id, file_id);
            assert_eq!(actual, range);
        };

        roundtrip(0..0);
        roundtrip(177..233);
        roundtrip(0..8388607);
        roundtrip(8388606..8388607); // 2^23-2 .. 2^23-1
    }

    #[test]
    fn test_range_mapper() {
        let base = "-- Hello\n-- world\n";
        let ranges = [(3..9), (12..18)];
        let mapped = ranges.iter().map(|r| &base[r.clone()]).collect::<String>();
        let m = RangeMapper::new(ranges).unwrap();

        assert_eq!(mapped, "Hello\nworld\n");
        assert_eq!(m.map(2..3), 5..6); // l -> l
        assert_eq!(m.map(4..6), (7..9)); // o\n -> o\n
        assert_eq!(m.map(6..8), (12..14)); // wo -> wo
        assert_eq!(m.map(8..11), (14..17)); // rld -> rld
        assert_eq!(m.map(2..12), (5..18)); // llo\n-- world\n -> llo\n-- world\n

        // Empty ranges on boundaries:
        assert_eq!(m.map(0..0), (3..3));
        assert_eq!(m.map(6..6), (9..9)); // maps to the left of the boundary
        assert_eq!(m.map(12..12), (18..18));
    }

    /// Small exhaustive edge case tests for the range mapper
    #[test]
    fn test_range_mapper_exhaustive() {
        let empty = RangeMapper::new([]).unwrap();
        assert_eq!(empty.map(0..0), 0..0);

        let exact = RangeMapper::new(Some(0..1)).unwrap();
        assert_eq!(exact.map(0..0), 0..0);
        assert_eq!(exact.map(0..1), 0..1);
        assert_eq!(exact.map(1..1), 1..1);

        let plus = RangeMapper::new(Some(10..11)).unwrap();
        assert_eq!(plus.map(0..0), 10..10);
        assert_eq!(plus.map(0..1), 10..11);
        assert_eq!(plus.map(1..1), 11..11);

        let disjoint = RangeMapper::new([(10..11), (21..22)]).unwrap();
        assert_eq!(disjoint.map(0..0), 10..10);
        assert_eq!(disjoint.map(0..1), 10..11);
        assert_eq!(disjoint.map(0..2), 10..22);
        assert_eq!(disjoint.map(1..1), 11..11);
        assert_eq!(disjoint.map(1..2), 21..22);
        assert_eq!(disjoint.map(2..2), 22..22);

        // disjoint with interspersed empty ranges.
        let with_empty = RangeMapper::new([
            (10..10),
            (10..11),
            (11..11),
            (16..16),
            (21..21),
            (21..22),
            (22..22),
        ])
        .unwrap();
        assert_eq!(with_empty.map(0..0), 10..10);
        assert_eq!(with_empty.map(0..1), 10..11);
        assert_eq!(with_empty.map(0..2), 10..22);
        assert_eq!(with_empty.map(1..1), 11..11);
        assert_eq!(with_empty.map(1..2), 21..22);
        assert_eq!(with_empty.map(2..2), 22..22);
    }
}
