use std::rc::Rc;

use super::*;
use crate::geom::{AngularUnit, LengthUnit};

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// The none literal: `none`.
    None(Span),
    /// The auto literal: `auto`.
    Auto(Span),
    /// A boolean literal: `true`, `false`.
    Bool(Span, bool),
    /// An integer literal: `120`.
    Int(Span, i64),
    /// A floating-point literal: `1.2`, `10e-4`.
    Float(Span, f64),
    /// A length literal: `12pt`, `3cm`.
    Length(Span, f64, LengthUnit),
    /// An angle literal:  `1.5rad`, `90deg`.
    Angle(Span, f64, AngularUnit),
    /// A percent literal: `50%`.
    ///
    /// _Note_: `50%` is stored as `50.0` here, but as `0.5` in the
    /// corresponding [value](crate::geom::Relative).
    Percent(Span, f64),
    /// A fraction unit literal: `1fr`.
    Fractional(Span, f64),
    /// A string literal: `"hello!"`.
    Str(Span, EcoString),
    /// An identifier: `left`.
    Ident(Ident),
    /// An array expression: `(1, "hi", 12cm)`.
    Array(ArrayExpr),
    /// A dictionary expression: `(thickness: 3pt, pattern: dashed)`.
    Dict(DictExpr),
    /// A template expression: `[*Hi* there!]`.
    Template(TemplateExpr),
    /// A grouped expression: `(1 + 2)`.
    Group(GroupExpr),
    /// A block expression: `{ let x = 1; x + 2 }`.
    Block(BlockExpr),
    /// A unary operation: `-x`.
    Unary(UnaryExpr),
    /// A binary operation: `a + b`.
    Binary(BinaryExpr),
    /// An invocation of a function: `f(x, y)`.
    Call(CallExpr),
    /// A closure expression: `(x, y) => z`.
    Closure(ClosureExpr),
    /// A with expression: `f with (x, y: 1)`.
    With(WithExpr),
    /// A let expression: `let x = 1`.
    Let(LetExpr),
    /// An if-else expression: `if x { y } else { z }`.
    If(IfExpr),
    /// A while loop expression: `while x { y }`.
    While(WhileExpr),
    /// A for loop expression: `for x in y { z }`.
    For(ForExpr),
    /// An import expression: `import a, b, c from "utils.typ"`.
    Import(ImportExpr),
    /// An include expression: `include "chapter1.typ"`.
    Include(IncludeExpr),
}

impl Expr {
    /// The source code location.
    pub fn span(&self) -> Span {
        match *self {
            Self::None(span) => span,
            Self::Auto(span) => span,
            Self::Bool(span, _) => span,
            Self::Int(span, _) => span,
            Self::Float(span, _) => span,
            Self::Length(span, _, _) => span,
            Self::Angle(span, _, _) => span,
            Self::Percent(span, _) => span,
            Self::Fractional(span, _) => span,
            Self::Str(span, _) => span,
            Self::Ident(ref v) => v.span,
            Self::Array(ref v) => v.span,
            Self::Dict(ref v) => v.span,
            Self::Template(ref v) => v.span,
            Self::Group(ref v) => v.span,
            Self::Block(ref v) => v.span,
            Self::Unary(ref v) => v.span,
            Self::Binary(ref v) => v.span,
            Self::Call(ref v) => v.span,
            Self::Closure(ref v) => v.span,
            Self::With(ref v) => v.span,
            Self::Let(ref v) => v.span,
            Self::If(ref v) => v.span,
            Self::While(ref v) => v.span,
            Self::For(ref v) => v.span,
            Self::Import(ref v) => v.span,
            Self::Include(ref v) => v.span,
        }
    }

    /// Whether the expression can be shortened in markup with a hashtag.
    pub fn has_short_form(&self) -> bool {
        matches!(self,
            Self::Ident(_)
            | Self::Call(_)
            | Self::Let(_)
            | Self::If(_)
            | Self::While(_)
            | Self::For(_)
            | Self::Import(_)
            | Self::Include(_)
        )
    }
}

/// An array expression: `(1, "hi", 12cm)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayExpr {
    /// The source code location.
    pub span: Span,
    /// The entries of the array.
    pub items: Vec<Expr>,
}

/// A dictionary expression: `(thickness: 3pt, pattern: dashed)`.
#[derive(Debug, Clone, PartialEq)]
pub struct DictExpr {
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
pub struct TemplateExpr {
    /// The source code location.
    pub span: Span,
    /// The contents of the template.
    pub tree: Rc<SyntaxTree>,
}

/// A grouped expression: `(1 + 2)`.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupExpr {
    /// The source code location.
    pub span: Span,
    /// The wrapped expression.
    pub expr: Box<Expr>,
}

/// A block expression: `{ let x = 1; x + 2 }`.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpr {
    /// The source code location.
    pub span: Span,
    /// The list of expressions contained in the block.
    pub exprs: Vec<Expr>,
    /// Whether the block should create a scope.
    pub scoping: bool,
}

/// A unary operation: `-x`.
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpr {
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
pub struct BinaryExpr {
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
    /// The inclusive range operator: `..`.
    Range,
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
            Token::ExclEq => Self::Neq,
            Token::Lt => Self::Lt,
            Token::LtEq => Self::Leq,
            Token::Gt => Self::Gt,
            Token::GtEq => Self::Geq,
            Token::Eq => Self::Assign,
            Token::PlusEq => Self::AddAssign,
            Token::HyphEq => Self::SubAssign,
            Token::StarEq => Self::MulAssign,
            Token::SlashEq => Self::DivAssign,
            Token::Dots => Self::Range,
            _ => return None,
        })
    }

    /// The precedence of this operator.
    pub fn precedence(self) -> usize {
        match self {
            Self::Mul | Self::Div => 7,
            Self::Add | Self::Sub => 6,
            Self::Eq | Self::Neq | Self::Lt | Self::Leq | Self::Gt | Self::Geq => 5,
            Self::And => 4,
            Self::Or => 3,
            Self::Range => 2,
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
            | Self::Geq
            | Self::Range => Associativity::Left,
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
            Self::Range => "..",
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

/// An invocation of a function: `foo(...)`.
#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    /// The source code location.
    pub span: Span,
    /// The function to call.
    pub callee: Box<Expr>,
    /// Whether the call is wide, that is, capturing the template behind it.
    pub wide: bool,
    /// The arguments to the function.
    pub args: CallArgs,
}

/// The arguments to a function: `12, draw: false`.
///
/// In case of a bracketed invocation with a body, the body is _not_
/// included in the span for the sake of clearer error messages.
#[derive(Debug, Clone, PartialEq)]
pub struct CallArgs {
    /// The source code location.
    pub span: Span,
    /// The positional and named arguments.
    pub items: Vec<CallArg>,
}

/// An argument to a function call: `12` or `draw: false`.
#[derive(Debug, Clone, PartialEq)]
pub enum CallArg {
    /// A positional argument.
    Pos(Expr),
    /// A named argument.
    Named(Named),
}

impl CallArg {
    /// The source code location.
    pub fn span(&self) -> Span {
        match self {
            Self::Pos(expr) => expr.span(),
            Self::Named(named) => named.span(),
        }
    }
}

/// A closure expression: `(x, y) => z`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosureExpr {
    /// The source code location.
    pub span: Span,
    /// The name of the closure.
    ///
    /// This only exists if you use the function syntax sugar: `let f(x) = y`.
    pub name: Option<Ident>,
    /// The parameter bindings.
    pub params: Rc<Vec<Ident>>,
    /// The body of the closure.
    pub body: Rc<Expr>,
}

/// A with expression: `f with (x, y: 1)`.
///
/// Applies arguments to a function.
#[derive(Debug, Clone, PartialEq)]
pub struct WithExpr {
    /// The source code location.
    pub span: Span,
    /// The function to apply the arguments to.
    pub callee: Box<Expr>,
    /// The arguments to apply to the function.
    pub args: CallArgs,
}

/// A let expression: `let x = 1`.
#[derive(Debug, Clone, PartialEq)]
pub struct LetExpr {
    /// The source code location.
    pub span: Span,
    /// The binding to assign to.
    pub binding: Ident,
    /// The expression the binding is initialized with.
    pub init: Option<Box<Expr>>,
}

/// An import expression: `import a, b, c from "utils.typ"`.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportExpr {
    /// The source code location.
    pub span: Span,
    /// The items to be imported.
    pub imports: Imports,
    /// The location of the importable file.
    pub path: Box<Expr>,
}

/// The items that ought to be imported from a file.
#[derive(Debug, Clone, PartialEq)]
pub enum Imports {
    /// All items in the scope of the file should be imported.
    Wildcard,
    /// The specified identifiers from the file should be imported.
    Idents(Vec<Ident>),
}

/// An include expression: `include "chapter1.typ"`.
#[derive(Debug, Clone, PartialEq)]
pub struct IncludeExpr {
    /// The source code location.
    pub span: Span,
    /// The location of the file to be included.
    pub path: Box<Expr>,
}

/// An if-else expression: `if x { y } else { z }`.
#[derive(Debug, Clone, PartialEq)]
pub struct IfExpr {
    /// The source code location.
    pub span: Span,
    /// The condition which selects the body to evaluate.
    pub condition: Box<Expr>,
    /// The expression to evaluate if the condition is true.
    pub if_body: Box<Expr>,
    /// The expression to evaluate if the condition is false.
    pub else_body: Option<Box<Expr>>,
}

/// A while loop expression: `while x { y }`.
#[derive(Debug, Clone, PartialEq)]
pub struct WhileExpr {
    /// The source code location.
    pub span: Span,
    /// The condition which selects whether to evaluate the body.
    pub condition: Box<Expr>,
    /// The expression to evaluate while the condition is true.
    pub body: Box<Expr>,
}

/// A for loop expression: `for x in y { z }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ForExpr {
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
    /// A value pattern: `for v in array`.
    Value(Ident),
    /// A key-value pattern: `for k, v in dict`.
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
