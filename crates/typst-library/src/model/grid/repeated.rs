use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::layout::{Abs, Axes, Frame, Regions};

/// A repeatable grid header. Starts at the first row.
pub struct Header {
    /// The index after the last row included in this header.
    pub end: usize,
}

/// A repeatable grid footer. Stops at the last row.
pub struct Footer {
    /// The first row included in this footer.
    pub start: usize,
}

/// A possibly repeatable grid object.
/// It still exists even when not repeatable, but must not have additional
/// considerations by grid layout, other than for consistency (such as making
/// a certain group of rows unbreakable).
pub enum Repeatable<T> {
    Repeated(T),
    NotRepeated(T),
}

impl<T> Repeatable<T> {
    /// Gets the value inside this repeatable, regardless of whether
    /// it repeats.
    pub fn unwrap(&self) -> &T {
        match self {
            Self::Repeated(repeated) => repeated,
            Self::NotRepeated(not_repeated) => not_repeated,
        }
    }

    /// Returns `Some` if the value is repeated, `None` otherwise.
    pub fn as_repeated(&self) -> Option<&T> {
        match self {
            Self::Repeated(repeated) => Some(repeated),
            Self::NotRepeated(_) => None,
        }
    }
}
