//! Syntax types.

pub mod ast;
pub mod token;

mod ident;
mod span;

pub use ast::*;
pub use ident::*;
pub use span::*;
pub use token::*;

/// Decorations for semantic syntax highlighting.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
pub enum Deco {
    /// Emphasized text.
    Emph,
    /// Strong text.
    Strong,
    /// A valid, successfully resolved name.
    Resolved,
    /// An invalid, unresolved name.
    Unresolved,
    /// A key in a dictionary.
    DictKey,
}
