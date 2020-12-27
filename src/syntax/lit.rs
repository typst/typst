//! Literals.

use super::*;
use crate::color::RgbaColor;
use crate::eval::DictKey;
use crate::geom::Unit;

/// A literal.
#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    /// A identifier literal: `left`.
    Ident(Ident),
    /// A boolean literal: `true`, `false`.
    Bool(bool),
    /// An integer literal: `120`.
    Int(i64),
    /// A floating-point literal: `1.2`, `10e-4`.
    Float(f64),
    /// A length literal: `12pt`, `3cm`.
    Length(f64, Unit),
    /// A percent literal: `50%`.
    ///
    /// _Note_: `50%` is stored as `50.0` here, but as `0.5` in the
    /// corresponding [value](crate::geom::Relative).
    Percent(f64),
    /// A color literal: `#ffccee`.
    Color(RgbaColor),
    /// A string literal: `"hello!"`.
    Str(String),
    /// A dictionary literal: `(false, 12cm, greeting: "hi")`.
    Dict(LitDict),
    /// A content literal: `{*Hello* there!}`.
    Content(SynTree),
}

/// A dictionary literal: `(false, 12cm, greeting: "hi")`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LitDict(pub Vec<LitDictEntry>);

/// An entry in a dictionary literal: `false` or `greeting: "hi"`.
#[derive(Debug, Clone, PartialEq)]
pub struct LitDictEntry {
    /// The key of the entry if there was one: `greeting`.
    pub key: Option<Spanned<DictKey>>,
    /// The value of the entry: `"hi"`.
    pub expr: Spanned<Expr>,
}

impl LitDict {
    /// Create an empty dict literal.
    pub fn new() -> Self {
        Self::default()
    }
}
