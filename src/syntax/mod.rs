//! Syntax types.

mod expr;
mod ident;
mod lit;
mod span;
mod token;
mod tree;

/// Abstract syntax tree definition.
pub mod ast {
    use super::*;
    pub use expr::*;
    pub use lit::*;
    pub use tree::*;
}

pub use ast::*;
pub use ident::*;
pub use span::*;
pub use token::*;

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
