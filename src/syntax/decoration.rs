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
    /// A key part of a keyword argument.
    ArgumentKey,
    /// A key part of a pair in an object.
    ObjectKey,
    /// Text in italics.
    Italic,
    /// Text in bold.
    Bold,
}
