//! Decorations for semantic syntax highlighting.

#[cfg(feature = "serialize")]
use serde::Serialize;

use super::span::SpanVec;

/// A list of spanned decorations.
pub type Decorations = SpanVec<Decoration>;

/// Decorations for semantic syntax highlighting.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
pub enum Decoration {
    /// A valid, successfully resolved function name.
    ResolvedFunc,
    /// An invalid, unresolved function name.
    UnresolvedFunc,
    /// The key part of a key-value entry in a table.
    TableKey,
    /// Text in italics.
    Italic,
    /// Text in bold.
    Bold,
}
