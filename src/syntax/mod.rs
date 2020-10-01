//! Syntax types.

mod span;
mod token;
mod tree;

pub use span::*;
pub use token::*;
pub use tree::*;

/// Decorations for semantic syntax highlighting.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
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
    /// A key in a dictionary.
    DictKey,
}
