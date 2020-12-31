//! Expressions.

use super::*;
use crate::color::RgbaColor;
use crate::eval::DictKey;
use crate::geom::Unit;

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal: `true`, `1cm`, `"hi"`, `{_Hey!_}`.
    Lit(Lit),
    /// An invocation of a function: `[foo ...]`, `foo(...)`.
    Call(ExprCall),
    /// A unary operation: `-x`.
    Unary(ExprUnary),
    /// A binary operation: `a + b`, `a / b`.
    Binary(ExprBinary),
}

/// An invocation of a function: `[foo ...]`, `foo(...)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprCall {
    /// The name of the function.
    pub name: Spanned<Ident>,
    /// The arguments to the function.
    ///
    /// In case of a bracketed invocation with a body, the body is _not_
    /// included in the span for the sake of clearer error messages.
    pub args: Spanned<LitDict>,
}

/// A unary operation: `-x`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprUnary {
    /// The operator: `-`.
    pub op: Spanned<UnOp>,
    /// The expression to operator on: `x`.
    pub expr: Box<Spanned<Expr>>,
}

/// A unary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum UnOp {
    /// The negation operator: `-`.
    Neg,
}

/// A binary operation: `a + b`, `a / b`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprBinary {
    /// The left-hand side of the operation: `a`.
    pub lhs: Box<Spanned<Expr>>,
    /// The operator: `+`.
    pub op: Spanned<BinOp>,
    /// The right-hand side of the operation: `b`.
    pub rhs: Box<Spanned<Expr>>,
}

/// A binary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BinOp {
    /// The addition operator: `+`.
    Add,
    /// The subtraction operator: `-`.
    Sub,
    /// The multiplication operator: `*`.
    Mul,
    /// The division operator: `/`.
    Div,
}

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
