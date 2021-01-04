use super::*;
use crate::color::RgbaColor;
use crate::geom::Unit;

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal: `true`, `1cm`, `"hi"`.
    Lit(Lit),
    /// An invocation of a function: `[foo ...]`, `foo(...)`.
    Call(ExprCall),
    /// A unary operation: `-x`.
    Unary(ExprUnary),
    /// A binary operation: `a + b`, `a / b`.
    Binary(ExprBinary),
    /// An array expression: `(1, "hi", 12cm)`.
    Array(ExprArray),
    /// A dictionary expression: `(color: #f79143, pattern: dashed)`.
    Dict(ExprDict),
    /// A content expression: `{*Hello* there!}`.
    Content(ExprContent),
}

/// An invocation of a function: `[foo ...]`, `foo(...)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprCall {
    /// The name of the function.
    pub name: Spanned<Ident>,
    /// The arguments to the function.
    pub args: Spanned<ExprArgs>,
}

/// The arguments to a function: `12, draw: false`.
///
/// In case of a bracketed invocation with a body, the body is _not_
/// included in the span for the sake of clearer error messages.
pub type ExprArgs = Vec<Argument>;

/// An argument to a function call: `12` or `draw: false`.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// A positional arguments.
    Pos(Spanned<Expr>),
    /// A named argument.
    Named(Named),
}

/// A pair of a name and an expression: `pattern: dashed`.
#[derive(Debug, Clone, PartialEq)]
pub struct Named {
    /// The name: `pattern`.
    pub name: Spanned<Ident>,
    /// The right-hand side of the pair: `dashed`.
    pub expr: Spanned<Expr>,
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

/// An array expression: `(1, "hi", 12cm)`.
pub type ExprArray = SpanVec<Expr>;

/// A dictionary expression: `(color: #f79143, pattern: dashed)`.
pub type ExprDict = Vec<Named>;

/// A content expression: `{*Hello* there!}`.
pub type ExprContent = Tree;

/// A literal.
#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    /// A identifier literal: `left`.
    Ident(Ident),
    /// The none literal: `none`.
    None,
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
}
