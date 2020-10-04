//! Expressions.

use super::*;

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal: `true`, `1cm`, `"hi"`, `{_Hey!_}`.
    Lit(Lit),
    /// An invocation of a function: `[foo: ...]`, `foo(...)`.
    Call(ExprCall),
    /// A unary operation: `-x`.
    Unary(ExprUnary),
    /// A binary operation: `a + b`, `a / b`.
    Binary(ExprBinary),
}

/// An invocation of a function: `[foo: ...]`, `foo(...)`.
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
    pub expr: Spanned<Box<Expr>>,
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
    pub lhs: Spanned<Box<Expr>>,
    /// The operator: `+`.
    pub op: Spanned<BinOp>,
    /// The right-hand side of the operation: `b`.
    pub rhs: Spanned<Box<Expr>>,
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
