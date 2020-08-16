//! Decorations for semantic syntax highlighting.

#[cfg(feature = "serialize")]
use serde::Serialize;

use super::span::SpanVec;

/// A list of spanned decorations.
pub type Decorations = SpanVec<Decoration>;

/// Decorations for semantic syntax highlighting.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
pub enum Decoration {
    /// Text in italics.
    Italic,
    /// Text in bold.
    Bold,
    /// A valid, successfully resolved name.
    Resolved,
    /// An invalid, unresolved name.
    Unresolved,
    /// The key part of a key-value entry in a table.
    TableKey,
}
