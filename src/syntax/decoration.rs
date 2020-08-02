//! Decorations for semantic syntax highlighting.

use serde::Serialize;
use super::span::SpanVec;

/// A list of spanned decorations.
pub type Decorations = SpanVec<Decoration>;

/// Decorations for semantic syntax highlighting.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Decoration {
    /// A valid function name.
    /// ```typst
    /// [box]
    ///  ^^^
    /// ```
    ValidFuncName,
    /// An invalid function name.
    /// ```typst
    /// [blabla]
    ///  ^^^^^^
    /// ```
    InvalidFuncName,
    /// A key of a keyword argument.
    /// ```typst
    /// [box: width=5cm]
    ///       ^^^^^
    /// ```
    ArgumentKey,
    /// A key in an object.
    /// ```typst
    /// [box: padding={ left: 1cm, right: 2cm}]
    ///                 ^^^^       ^^^^^
    /// ```
    ObjectKey,
    /// An italic word.
    Italic,
    /// A bold word.
    Bold,
}
