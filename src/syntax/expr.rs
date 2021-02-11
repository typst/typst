use std::rc::Rc;

use super::*;
use crate::color::RgbaColor;
use crate::geom::{AngularUnit, LengthUnit};

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal.
    Lit(Lit),
    /// An identifier: `left`.
    Ident(Ident),
    /// An array expression: `(1, "hi", 12cm)`.
    Array(ExprArray),
    /// A dictionary expression: `(color: #f79143, pattern: dashed)`.
    Dict(ExprDict),
    /// A template expression: `[*Hi* there!]`.
    Template(ExprTemplate),
    /// A grouped expression: `(1 + 2)`.
    Group(ExprGroup),
    /// A block expression: `{ #let x = 1; x + 2 }`.
    Block(ExprBlock),
    /// A unary operation: `-x`.
    Unary(ExprUnary),
    /// A binary operation: `a + b`.
    Binary(ExprBinary),
    /// An invocation of a function: `foo(...)`, `#[foo ...]`.
    Call(ExprCall),
    /// A let expression: `#let x = 1`.
    Let(ExprLet),
    /// An if expression: `#if x { y } #else { z }`.
    If(ExprIf),
    /// A for expression: `#for x #in y { z }`.
    For(ExprFor),
}

impl Expr {
    /// The source code location.
    pub fn span(&self) -> Span {
        match self {
            Self::Lit(v) => v.span,
            Self::Ident(v) => v.span,
            Self::Array(v) => v.span,
            Self::Dict(v) => v.span,
            Self::Template(v) => v.span,
            Self::Group(v) => v.span,
            Self::Block(v) => v.span,
            Self::Unary(v) => v.span,
            Self::Binary(v) => v.span,
            Self::Call(v) => v.span,
            Self::Let(v) => v.span,
            Self::If(v) => v.span,
            Self::For(v) => v.span,
        }
    }
}

/// A literal.
#[derive(Debug, Clone, PartialEq)]
pub struct Lit {
    /// The source code location.
    pub span: Span,
    /// The kind of literal.
    pub kind: LitKind,
}

/// A kind of literal.
#[derive(Debug, Clone, PartialEq)]
pub enum LitKind {
    /// The none literal: `none`.
    None,
    /// A boolean literal: `true`, `false`.
    Bool(bool),
    /// An integer literal: `120`.
    Int(i64),
    /// A floating-point literal: `1.2`, `10e-4`.
    Float(f64),
    /// A length literal: `12pt`, `3cm`.
    Length(f64, LengthUnit),
    /// An angle literal:  `1.5rad`, `90deg`.
    Angle(f64, AngularUnit),
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

/// An array expression: `(1, "hi", 12cm)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprArray {
    /// The source code location.
    pub span: Span,
    /// The entries of the array.
    pub items: Vec<Expr>,
}

/// A dictionary expression: `(color: #f79143, pattern: dashed)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprDict {
    /// The source code location.
    pub span: Span,
    /// The named dictionary entries.
    pub items: Vec<Named>,
}

/// A pair of a name and an expression: `pattern: dashed`.
#[derive(Debug, Clone, PartialEq)]
pub struct Named {
    /// The name: `pattern`.
    pub name: Ident,
    /// The right-hand side of the pair: `dashed`.
    pub expr: Expr,
}

impl Named {
    /// The source code location.
    pub fn span(&self) -> Span {
        self.name.span.join(self.expr.span())
    }
}

/// A template expression: `[*Hi* there!]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprTemplate {
    /// The source code location.
    pub span: Span,
    /// The contents of the template.
    pub tree: Rc<Tree>,
}

/// A grouped expression: `(1 + 2)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprGroup {
    /// The source code location.
    pub span: Span,
    /// The wrapped expression.
    pub expr: Box<Expr>,
}

/// A block expression: `{ #let x = 1; x + 2 }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprBlock {
    /// The source code location.
    pub span: Span,
    /// The list of expressions contained in the block.
    pub exprs: Vec<Expr>,
    /// Whether the block should create a scope.
    pub scoping: bool,
}

/// A unary operation: `-x`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprUnary {
    /// The source code location.
    pub span: Span,
    /// The operator: `-`.
    pub op: UnOp,
    /// The expression to operator on: `x`.
    pub expr: Box<Expr>,
}

/// A unary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum UnOp {
    /// The plus operator: `+`.
    Pos,
    /// The negation operator: `-`.
    Neg,
    /// The boolean `not`.
    Not,
}

impl UnOp {
    /// Try to convert the token into a unary operation.
    pub fn from_token(token: Token) -> Option<Self> {
        Some(match token {
            Token::Plus => Self::Pos,
            Token::Hyph => Self::Neg,
            Token::Not => Self::Not,
            _ => return None,
        })
    }

    /// The precedence of this operator.
    pub fn precedence(self) -> usize {
        match self {
            Self::Pos | Self::Neg => 8,
            Self::Not => 4,
        }
    }

    /// The string representation of this operation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pos => "+",
            Self::Neg => "-",
            Self::Not => "not",
        }
    }
}

/// A binary operation: `a + b`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprBinary {
    /// The source code location.
    pub span: Span,
    /// The left-hand side of the operation: `a`.
    pub lhs: Box<Expr>,
    /// The operator: `+`.
    pub op: BinOp,
    /// The right-hand side of the operation: `b`.
    pub rhs: Box<Expr>,
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
    /// The short-circuiting boolean `and`.
    And,
    /// The short-circuiting boolean `or`.
    Or,
    /// The equality operator: `==`.
    Eq,
    /// The inequality operator: `!=`.
    Neq,
    /// The less-than operator: `<`.
    Lt,
    /// The less-than or equal operator: `<=`.
    Leq,
    /// The greater-than operator: `>`.
    Gt,
    /// The greater-than or equal operator: `>=`.
    Geq,
    /// The assignment operator: `=`.
    Assign,
    /// The add-assign operator: `+=`.
    AddAssign,
    /// The subtract-assign oeprator: `-=`.
    SubAssign,
    /// The multiply-assign operator: `*=`.
    MulAssign,
    /// The divide-assign operator: `/=`.
    DivAssign,
}

impl BinOp {
    /// Try to convert the token into a binary operation.
    pub fn from_token(token: Token) -> Option<Self> {
        Some(match token {
            Token::Plus => Self::Add,
            Token::Hyph => Self::Sub,
            Token::Star => Self::Mul,
            Token::Slash => Self::Div,
            Token::And => Self::And,
            Token::Or => Self::Or,
            Token::EqEq => Self::Eq,
            Token::BangEq => Self::Neq,
            Token::Lt => Self::Lt,
            Token::LtEq => Self::Leq,
            Token::Gt => Self::Gt,
            Token::GtEq => Self::Geq,
            Token::Eq => Self::Assign,
            Token::PlusEq => Self::AddAssign,
            Token::HyphEq => Self::SubAssign,
            Token::StarEq => Self::MulAssign,
            Token::SlashEq => Self::DivAssign,
            _ => return None,
        })
    }

    /// The precedence of this operator.
    pub fn precedence(self) -> usize {
        match self {
            Self::Mul | Self::Div => 7,
            Self::Add | Self::Sub => 6,
            Self::Eq | Self::Neq | Self::Lt | Self::Leq | Self::Gt | Self::Geq => 5,
            Self::And => 3,
            Self::Or => 2,
            Self::Assign
            | Self::AddAssign
            | Self::SubAssign
            | Self::MulAssign
            | Self::DivAssign => 1,
        }
    }

    /// The associativity of this operator.
    pub fn associativity(self) -> Associativity {
        match self {
            Self::Add
            | Self::Sub
            | Self::Mul
            | Self::Div
            | Self::And
            | Self::Or
            | Self::Eq
            | Self::Neq
            | Self::Lt
            | Self::Leq
            | Self::Gt
            | Self::Geq => Associativity::Left,
            Self::Assign
            | Self::AddAssign
            | Self::SubAssign
            | Self::MulAssign
            | Self::DivAssign => Associativity::Right,
        }
    }

    /// The string representation of this operation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::And => "and",
            Self::Or => "or",
            Self::Eq => "==",
            Self::Neq => "!=",
            Self::Lt => "<",
            Self::Leq => "<=",
            Self::Gt => ">",
            Self::Geq => ">=",
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::MulAssign => "*=",
            Self::DivAssign => "/=",
        }
    }
}

/// The associativity of a binary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Associativity {
    /// Left-associative: `a + b + c` is equivalent to `(a + b) + c`.
    Left,
    /// Right-associative: `a = b = c` is equivalent to `a = (b = c)`.
    Right,
}

/// An invocation of a function: `foo(...)`, `#[foo ...]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprCall {
    /// The source code location.
    pub span: Span,
    /// The callee of the function.
    pub callee: Box<Expr>,
    /// The arguments to the function.
    pub args: ExprArgs,
}

/// The arguments to a function: `12, draw: false`.
///
/// In case of a bracketed invocation with a body, the body is _not_
/// included in the span for the sake of clearer error messages.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprArgs {
    /// The source code location.
    pub span: Span,
    /// The positional and named arguments.
    pub items: Vec<ExprArg>,
}

/// An argument to a function call: `12` or `draw: false`.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprArg {
    /// A positional argument.
    Pos(Expr),
    /// A named argument.
    Named(Named),
}

impl ExprArg {
    /// The source code location.
    pub fn span(&self) -> Span {
        match self {
            Self::Pos(expr) => expr.span(),
            Self::Named(named) => named.span(),
        }
    }
}

/// A let expression: `#let x = 1`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprLet {
    /// The source code location.
    pub span: Span,
    /// The binding to assign to.
    pub binding: Ident,
    /// The expression the pattern is initialized with.
    pub init: Option<Box<Expr>>,
}

/// An if expression: `#if x { y } #else { z }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprIf {
    /// The source code location.
    pub span: Span,
    /// The condition which selects the body to evaluate.
    pub condition: Box<Expr>,
    /// The expression to evaluate if the condition is true.
    pub if_body: Box<Expr>,
    /// The expression to evaluate if the condition is false.
    pub else_body: Option<Box<Expr>>,
}

/// A for expression: `#for x #in y { z }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprFor {
    /// The source code location.
    pub span: Span,
    /// The pattern to assign to.
    pub pattern: ForPattern,
    /// The expression to iterate over.
    pub iter: Box<Expr>,
    /// The expression to evaluate for each iteration.
    pub body: Box<Expr>,
}

/// A pattern in a for loop.
#[derive(Debug, Clone, PartialEq)]
pub enum ForPattern {
    /// A value pattern: `#for v #in array`.
    Value(Ident),
    /// A key-value pattern: `#for k, v #in dict`.
    KeyValue(Ident, Ident),
}

impl ForPattern {
    /// The source code location.
    pub fn span(&self) -> Span {
        match self {
            Self::Value(v) => v.span,
            Self::KeyValue(k, v) => k.span.join(v.span),
        }
    }
}
