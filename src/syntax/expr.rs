use super::*;
use crate::color::RgbaColor;
use crate::geom::{AngularUnit, LengthUnit};

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// The none literal: `none`.
    None,
    /// A identifier literal: `left`.
    Ident(Ident),
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
    /// An invocation of a function: `foo(...)`, `[foo ...]`.
    Call(ExprCall),
    /// A let expression: `#let x = 1`.
    Let(ExprLet),
    /// An if expression: `#if x { y } #else { z }`.
    If(ExprIf),
    /// A for expression: `#for x #in y { z }`.
    For(ExprFor),
}

impl Pretty for Expr {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::None => p.push_str("none"),
            Self::Ident(v) => p.push_str(&v),
            Self::Bool(v) => write!(p, "{}", v).unwrap(),
            Self::Int(v) => p.push_str(itoa::Buffer::new().format(*v)),
            Self::Float(v) => p.push_str(ryu::Buffer::new().format(*v)),
            Self::Length(v, u) => write!(p, "{}{}", v, u).unwrap(),
            Self::Angle(v, u) => write!(p, "{}{}", v, u).unwrap(),
            Self::Percent(v) => write!(p, "{}%", v).unwrap(),
            Self::Color(v) => write!(p, "{}", v).unwrap(),
            // TODO: Debug escapes a bit more than we want (e.g. apostrophes).
            // We probably need to do the escaping ourselves.
            Self::Str(v) => write!(p, "{:?}", &v).unwrap(),
            Self::Array(v) => v.pretty(p),
            Self::Dict(v) => v.pretty(p),
            Self::Template(v) => {
                p.push_str("[");
                v.pretty(p);
                p.push_str("]");
            }
            Self::Group(v) => {
                p.push_str("(");
                v.v.pretty(p);
                p.push_str(")");
            }
            Self::Block(v) => v.pretty(p),
            Self::Unary(v) => v.pretty(p),
            Self::Binary(v) => v.pretty(p),
            Self::Call(v) => v.pretty(p),
            Self::Let(v) => v.pretty(p),
            Self::If(v) => v.pretty(p),
            Self::For(v) => v.pretty(p),
        }
    }
}

/// An array expression: `(1, "hi", 12cm)`.
pub type ExprArray = SpanVec<Expr>;

impl Pretty for ExprArray {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("(");
        p.join(self, ", ", |item, p| item.v.pretty(p));
        if self.len() == 1 {
            p.push_str(",");
        }
        p.push_str(")");
    }
}

/// A dictionary expression: `(color: #f79143, pattern: dashed)`.
pub type ExprDict = Vec<Named>;

impl Pretty for ExprDict {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("(");
        if self.is_empty() {
            p.push_str(":");
        } else {
            p.join(self, ", ", |named, p| named.pretty(p));
        }
        p.push_str(")");
    }
}

/// A pair of a name and an expression: `pattern: dashed`.
#[derive(Debug, Clone, PartialEq)]
pub struct Named {
    /// The name: `pattern`.
    pub name: Spanned<Ident>,
    /// The right-hand side of the pair: `dashed`.
    pub expr: Spanned<Expr>,
}

impl Pretty for Named {
    fn pretty(&self, p: &mut Printer) {
        p.push_str(&self.name.v);
        p.push_str(": ");
        self.expr.v.pretty(p);
    }
}

/// A template expression: `[*Hi* there!]`.
pub type ExprTemplate = Tree;

/// A grouped expression: `(1 + 2)`.
pub type ExprGroup = SpanBox<Expr>;

/// A block expression: `{ #let x = 1; x + 2 }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprBlock {
    /// The list of expressions contained in the block.
    pub exprs: SpanVec<Expr>,
}

impl Pretty for ExprBlock {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("{");
        if self.exprs.len() > 1 {
            p.push_str(" ");
        }
        p.join(&self.exprs, "; ", |expr, p| expr.v.pretty(p));
        if self.exprs.len() > 1 {
            p.push_str(" ");
        }
        p.push_str("}");
    }
}

/// A unary operation: `-x`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprUnary {
    /// The operator: `-`.
    pub op: Spanned<UnOp>,
    /// The expression to operator on: `x`.
    pub expr: SpanBox<Expr>,
}

impl Pretty for ExprUnary {
    fn pretty(&self, p: &mut Printer) {
        self.op.v.pretty(p);
        if self.op.v == UnOp::Not {
            p.push_str(" ");
        }
        self.expr.v.pretty(p);
    }
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

impl Pretty for UnOp {
    fn pretty(&self, p: &mut Printer) {
        p.push_str(self.as_str());
    }
}

/// A binary operation: `a + b`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprBinary {
    /// The left-hand side of the operation: `a`.
    pub lhs: SpanBox<Expr>,
    /// The operator: `+`.
    pub op: Spanned<BinOp>,
    /// The right-hand side of the operation: `b`.
    pub rhs: SpanBox<Expr>,
}

impl Pretty for ExprBinary {
    fn pretty(&self, p: &mut Printer) {
        self.lhs.v.pretty(p);
        p.push_str(" ");
        self.op.v.pretty(p);
        p.push_str(" ");
        self.rhs.v.pretty(p);
    }
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

impl Pretty for BinOp {
    fn pretty(&self, p: &mut Printer) {
        p.push_str(self.as_str());
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

/// An invocation of a function: `foo(...)`, `[foo ...]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprCall {
    /// The callee of the function.
    pub callee: SpanBox<Expr>,
    /// The arguments to the function.
    pub args: Spanned<ExprArgs>,
}

impl Pretty for ExprCall {
    fn pretty(&self, p: &mut Printer) {
        self.callee.v.pretty(p);
        p.push_str("(");
        self.args.v.pretty(p);
        p.push_str(")");
    }
}

/// Pretty print a bracketed function call, with body or chaining when possible.
pub fn pretty_bracket_call(call: &ExprCall, p: &mut Printer, chained: bool) {
    if chained {
        p.push_str(" | ");
    } else {
        p.push_str("[");
    }

    // Function name.
    call.callee.v.pretty(p);

    // Find out whether this can be written with a body or as a chain.
    //
    // Example: Transforms "[v [Hi]]" => "[v][Hi]".
    if let [head @ .., Argument::Pos(Spanned { v: Expr::Template(template), .. })] =
        call.args.v.as_slice()
    {
        // Previous arguments.
        if !head.is_empty() {
            p.push_str(" ");
            p.join(head, ", ", |item, p| item.pretty(p));
        }

        // Find out whether this can written as a chain.
        //
        // Example: Transforms "[v][[f]]" => "[v | f]".
        if let [Spanned { v: Node::Expr(Expr::Call(call)), .. }] = template.as_slice() {
            return pretty_bracket_call(call, p, true);
        } else {
            p.push_str("][");
            template.pretty(p);
        }
    } else if !call.args.v.is_empty() {
        p.push_str(" ");
        call.args.v.pretty(p);
    }

    // Either end of header or end of body.
    p.push_str("]");
}

/// The arguments to a function: `12, draw: false`.
///
/// In case of a bracketed invocation with a body, the body is _not_
/// included in the span for the sake of clearer error messages.
pub type ExprArgs = Vec<Argument>;

impl Pretty for Vec<Argument> {
    fn pretty(&self, p: &mut Printer) {
        p.join(self, ", ", |item, p| item.pretty(p));
    }
}

/// An argument to a function call: `12` or `draw: false`.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// A positional arguments.
    Pos(Spanned<Expr>),
    /// A named argument.
    Named(Named),
}

impl Pretty for Argument {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::Pos(expr) => expr.v.pretty(p),
            Self::Named(named) => named.pretty(p),
        }
    }
}

/// A let expression: `#let x = 1`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprLet {
    /// The pattern to assign to.
    pub pat: Spanned<Ident>,
    /// The expression the pattern is initialized with.
    pub init: Option<SpanBox<Expr>>,
}

impl Pretty for ExprLet {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("#let ");
        p.push_str(&self.pat.v);
        if let Some(init) = &self.init {
            p.push_str(" = ");
            init.v.pretty(p);
        }
    }
}

/// An if expression: `#if x { y } #else { z }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprIf {
    /// The condition which selects the body to evaluate.
    pub condition: SpanBox<Expr>,
    /// The expression to evaluate if the condition is true.
    pub if_body: SpanBox<Expr>,
    /// The expression to evaluate if the condition is false.
    pub else_body: Option<SpanBox<Expr>>,
}

impl Pretty for ExprIf {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("#if ");
        self.condition.v.pretty(p);
        p.push_str(" ");
        self.if_body.v.pretty(p);
        if let Some(expr) = &self.else_body {
            p.push_str(" #else ");
            expr.v.pretty(p);
        }
    }
}

/// A for expression: `#for x #in y { z }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprFor {
    /// The pattern to assign to.
    pub pat: Spanned<Ident>,
    /// The expression to iterate over.
    pub iter: SpanBox<Expr>,
    /// The expression to evaluate for each iteration.
    pub body: SpanBox<Expr>,
}

impl Pretty for ExprFor {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("#for ");
        p.push_str(&self.pat.v);
        p.push_str(" #in ");
        self.iter.v.pretty(p);
        p.push_str(" ");
        self.body.v.pretty(p);
    }
}
