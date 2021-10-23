use super::{Ident, Markup, NodeKind, RedNode, RedTicket, Span, TypedNode};
use crate::geom::{AngularUnit, LengthUnit};
use crate::node;
use crate::util::EcoString;

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// An identifier: `left`.
    Ident(Ident),
    /// A literal: `1`, `true`, ...
    Lit(Lit),
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

    /// Return the expression's span.
    pub fn span(&self) -> Span {
        match self {
            Self::Ident(ident) => ident.span,
            Self::Lit(lit) => lit.span(),
            Self::Array(array) => array.span(),
            Self::Dict(dict) => dict.span(),
            Self::Template(template) => template.span(),
            Self::Group(group) => group.span(),
            Self::Block(block) => block.span(),
            Self::Unary(unary) => unary.span(),
            Self::Binary(binary) => binary.span(),
            Self::Call(call) => call.span(),
            Self::Closure(closure) => closure.span(),
            Self::With(with) => with.span(),
            Self::Let(let_) => let_.span(),
            Self::If(if_) => if_.span(),
            Self::While(while_) => while_.span(),
            Self::For(for_) => for_.span(),
            Self::Import(import) => import.span(),
            Self::Include(include) => include.span(),
        }
    }
}

impl TypedNode for Expr {
    fn cast_from(node: RedTicket) -> Option<Self> {
        match node.kind() {
            NodeKind::Ident(_) => Some(Self::Ident(Ident::cast_from(node).unwrap())),
            NodeKind::Array => Some(Self::Array(ArrayExpr::cast_from(node).unwrap())),
            NodeKind::Dict => Some(Self::Dict(DictExpr::cast_from(node).unwrap())),
            NodeKind::Template => {
                Some(Self::Template(TemplateExpr::cast_from(node).unwrap()))
            }
            NodeKind::Group => Some(Self::Group(GroupExpr::cast_from(node).unwrap())),
            NodeKind::Block => Some(Self::Block(BlockExpr::cast_from(node).unwrap())),
            NodeKind::Unary => Some(Self::Unary(UnaryExpr::cast_from(node).unwrap())),
            NodeKind::Binary => Some(Self::Binary(BinaryExpr::cast_from(node).unwrap())),
            NodeKind::Call => Some(Self::Call(CallExpr::cast_from(node).unwrap())),
            NodeKind::Closure => {
                Some(Self::Closure(ClosureExpr::cast_from(node).unwrap()))
            }
            NodeKind::WithExpr => Some(Self::With(WithExpr::cast_from(node).unwrap())),
            NodeKind::LetExpr => Some(Self::Let(LetExpr::cast_from(node).unwrap())),
            NodeKind::IfExpr => Some(Self::If(IfExpr::cast_from(node).unwrap())),
            NodeKind::WhileExpr => Some(Self::While(WhileExpr::cast_from(node).unwrap())),
            NodeKind::ForExpr => Some(Self::For(ForExpr::cast_from(node).unwrap())),
            NodeKind::ImportExpr => {
                Some(Self::Import(ImportExpr::cast_from(node).unwrap()))
            }
            NodeKind::IncludeExpr => {
                Some(Self::Include(IncludeExpr::cast_from(node).unwrap()))
            }
            _ => Some(Self::Lit(Lit::cast_from(node)?)),
        }
    }
}

/// A literal: `1`, `true`, ...
#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
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
}

impl TypedNode for Lit {
    fn cast_from(node: RedTicket) -> Option<Self> {
        match node.kind() {
            NodeKind::None => Some(Self::None(node.own().span())),
            NodeKind::Auto => Some(Self::Auto(node.own().span())),
            NodeKind::Bool(b) => Some(Self::Bool(node.own().span(), *b)),
            NodeKind::Int(i) => Some(Self::Int(node.own().span(), *i)),
            NodeKind::Float(f) => Some(Self::Float(node.own().span(), *f)),
            NodeKind::Length(f, unit) => Some(Self::Length(node.own().span(), *f, *unit)),
            NodeKind::Angle(f, unit) => Some(Self::Angle(node.own().span(), *f, *unit)),
            NodeKind::Percentage(f) => Some(Self::Percent(node.own().span(), *f)),
            NodeKind::Fraction(f) => Some(Self::Fractional(node.own().span(), *f)),
            NodeKind::Str(s) => Some(Self::Str(node.own().span(), s.string.clone())),
            _ => None,
        }
    }
}

impl Lit {
    pub fn span(&self) -> Span {
        match self {
            Self::None(span) => *span,
            Self::Auto(span) => *span,
            Self::Bool(span, _) => *span,
            Self::Int(span, _) => *span,
            Self::Float(span, _) => *span,
            Self::Length(span, _, _) => *span,
            Self::Angle(span, _, _) => *span,
            Self::Percent(span, _) => *span,
            Self::Fractional(span, _) => *span,
            Self::Str(span, _) => *span,
        }
    }
}

node!(
    /// An array expression: `(1, "hi", 12cm)`.
    Array => ArrayExpr
);

impl ArrayExpr {
    /// The array items.
    pub fn items(&self) -> Vec<Expr> {
        self.0.children().filter_map(RedTicket::cast).collect()
    }
}

node!(
    /// A dictionary expression: `(thickness: 3pt, pattern: dashed)`.
    Dict => DictExpr
);

impl DictExpr {
    /// The named dictionary items.
    pub fn items(&self) -> Vec<Named> {
        self.0.children().filter_map(RedTicket::cast).collect()
    }
}

node!(
    /// A pair of a name and an expression: `pattern: dashed`.
    Named
);

impl Named {
    /// The name: `pattern`.
    pub fn name(&self) -> Ident {
        self.0.cast_first_child().expect("named pair is missing name ident")
    }

    /// The right-hand side of the pair: `dashed`.
    pub fn expr(&self) -> Expr {
        self.0
            .children()
            .filter_map(RedTicket::cast)
            .nth(1)
            .expect("named pair is missing expression")
    }
}

node!(
    /// A template expression: `[*Hi* there!]`.
    Template => TemplateExpr
);

impl TemplateExpr {
    /// The contents of the template.
    pub fn body(&self) -> Markup {
        self.0
            .cast_first_child()
            .expect("template expression is missing body")
    }
}

node!(
    /// A grouped expression: `(1 + 2)`.
    Group => GroupExpr
);

impl GroupExpr {
    /// The wrapped expression.
    pub fn expr(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("group expression is missing expression")
    }
}

node!(
    /// A block expression: `{ let x = 1; x + 2 }`.
    Block => BlockExpr
);

impl BlockExpr {
    /// The list of expressions contained in the block.
    pub fn exprs(&self) -> Vec<Expr> {
        self.0.children().filter_map(RedTicket::cast).collect()
    }
}

node!(
    /// A unary operation: `-x`.
    Unary => UnaryExpr
);

impl UnaryExpr {
    /// The operator: `-`.
    pub fn op(&self) -> UnOp {
        self.0
            .cast_first_child()
            .expect("unary expression is missing operator")
    }

    /// The expression to operator on: `x`.
    pub fn expr(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("unary expression is missing expression")
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

impl TypedNode for UnOp {
    fn cast_from(node: RedTicket) -> Option<Self> {
        Self::from_token(node.kind())
    }
}

impl UnOp {
    /// Try to convert the token into a unary operation.
    pub fn from_token(token: &NodeKind) -> Option<Self> {
        Some(match token {
            NodeKind::Plus => Self::Pos,
            NodeKind::Minus => Self::Neg,
            NodeKind::Not => Self::Not,
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

node!(
    /// A binary operation: `a + b`.
    Binary => BinaryExpr
);

impl BinaryExpr {
    /// The binary operator: `+`.
    pub fn op(&self) -> BinOp {
        self.0
            .cast_first_child()
            .expect("binary expression is missing operator")
    }

    /// The left-hand side of the operation: `a`.
    pub fn lhs(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("binary expression is missing left-hand side")
    }

    /// The right-hand side of the operation: `b`.
    pub fn rhs(&self) -> Expr {
        self.0
            .children()
            .filter_map(RedTicket::cast)
            .nth(1)
            .expect("binary expression is missing right-hand side")
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

impl TypedNode for BinOp {
    fn cast_from(node: RedTicket) -> Option<Self> {
        Self::from_token(node.kind())
    }
}

impl BinOp {
    /// Try to convert the token into a binary operation.
    pub fn from_token(token: &NodeKind) -> Option<Self> {
        Some(match token {
            NodeKind::Plus => Self::Add,
            NodeKind::Minus => Self::Sub,
            NodeKind::Star => Self::Mul,
            NodeKind::Slash => Self::Div,
            NodeKind::And => Self::And,
            NodeKind::Or => Self::Or,
            NodeKind::EqEq => Self::Eq,
            NodeKind::ExclEq => Self::Neq,
            NodeKind::Lt => Self::Lt,
            NodeKind::LtEq => Self::Leq,
            NodeKind::Gt => Self::Gt,
            NodeKind::GtEq => Self::Geq,
            NodeKind::Eq => Self::Assign,
            NodeKind::PlusEq => Self::AddAssign,
            NodeKind::HyphEq => Self::SubAssign,
            NodeKind::StarEq => Self::MulAssign,
            NodeKind::SlashEq => Self::DivAssign,
            _ => return None,
        })
    }

    /// The precedence of this operator.
    pub fn precedence(self) -> usize {
        match self {
            Self::Mul | Self::Div => 6,
            Self::Add | Self::Sub => 5,
            Self::Eq | Self::Neq | Self::Lt | Self::Leq | Self::Gt | Self::Geq => 4,
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

node!(
    /// An invocation of a function: `foo(...)`.
    Call => CallExpr
);

impl CallExpr {
    /// The function to call.
    pub fn callee(&self) -> Expr {
        self.0.cast_first_child().expect("call expression is missing callee")
    }

    /// The arguments to the function.
    pub fn args(&self) -> CallArgs {
        self.0
            .cast_first_child()
            .expect("call expression is missing argument list")
    }
}

node!(
    /// The arguments to a function: `12, draw: false`.
    CallArgs
);

impl CallArgs {
    /// The positional and named arguments.
    pub fn items(&self) -> Vec<CallArg> {
        self.0.children().filter_map(RedTicket::cast).collect()
    }
}

/// An argument to a function call.
#[derive(Debug, Clone, PartialEq)]
pub enum CallArg {
    /// A positional argument: `12`.
    Pos(Expr),
    /// A named argument: `draw: false`.
    Named(Named),
    /// A spreaded argument: `..things`.
    Spread(Expr),
}

impl TypedNode for CallArg {
    fn cast_from(node: RedTicket) -> Option<Self> {
        match node.kind() {
            NodeKind::Named => Some(CallArg::Named(
                node.cast().expect("named call argument is missing name"),
            )),
            NodeKind::ParameterSink => Some(CallArg::Spread(
                node.own()
                    .cast_first_child()
                    .expect("call argument sink is missing expression"),
            )),
            _ => Some(CallArg::Pos(node.cast()?)),
        }
    }
}

impl CallArg {
    /// The name of this argument.
    pub fn span(&self) -> Span {
        match self {
            Self::Named(named) => named.span(),
            Self::Pos(expr) => expr.span(),
            Self::Spread(expr) => expr.span(),
        }
    }
}

node!(
    /// A closure expression: `(x, y) => z`.
    Closure => ClosureExpr
);

impl ClosureExpr {
    /// The name of the closure.
    ///
    /// This only exists if you use the function syntax sugar: `let f(x) = y`.
    pub fn name(&self) -> Option<Ident> {
        // `first_convert_child` does not work here because of the Option in the
        // Result.
        self.0.cast_first_child()
    }

    /// The parameter bindings.
    pub fn params(&self) -> Vec<ClosureParam> {
        self.0
            .children()
            .find(|x| x.kind() == &NodeKind::ClosureParams)
            .expect("closure is missing parameter list")
            .own()
            .children()
            .filter_map(RedTicket::cast)
            .collect()
    }

    /// The body of the closure.
    pub fn body(&self) -> Expr {
        // The filtering for the NodeKind is necessary here because otherwise,
        // `first_convert_child` will use the Ident if present.
        self.0.cast_last_child().expect("closure is missing body")
    }

    /// The ticket of the body of the closure.
    pub fn body_ticket(&self) -> RedTicket {
        self.0
            .children()
            .filter(|x| x.cast::<Expr>().is_some())
            .last()
            .unwrap()
    }
}

/// An parameter to a closure.
#[derive(Debug, Clone, PartialEq)]
pub enum ClosureParam {
    /// A positional parameter: `x`.
    Pos(Ident),
    /// A named parameter with a default value: `draw: false`.
    Named(Named),
    /// A parameter sink: `..args`.
    Sink(Ident),
}

impl TypedNode for ClosureParam {
    fn cast_from(node: RedTicket) -> Option<Self> {
        match node.kind() {
            NodeKind::Ident(i) => {
                Some(ClosureParam::Pos(Ident::new(i, node.own().span()).unwrap()))
            }
            NodeKind::Named => Some(ClosureParam::Named(
                node.cast().expect("named closure parameter is missing name"),
            )),
            NodeKind::ParameterSink => Some(ClosureParam::Sink(
                node.own()
                    .cast_first_child()
                    .expect("closure parameter sink is missing identifier"),
            )),
            _ => Some(ClosureParam::Pos(node.cast()?)),
        }
    }
}

node!(
    /// A with expression: `f with (x, y: 1)`.
    WithExpr
);

impl WithExpr {
    /// The function to apply the arguments to.
    pub fn callee(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("with expression is missing callee expression")
    }

    /// The arguments to apply to the function.
    pub fn args(&self) -> CallArgs {
        self.0
            .cast_first_child()
            .expect("with expression is missing argument list")
    }
}

node!(
    /// A let expression: `let x = 1`.
    LetExpr
);

impl LetExpr {
    /// The binding to assign to.
    pub fn binding(&self) -> Ident {
        if let Some(c) = self.0.cast_first_child() {
            c
        } else if let Some(w) = self.0.typed_child(&NodeKind::WithExpr) {
            // Can't do an `first_convert_child` here because the WithExpr's
            // callee has to be an identifier.
            w.cast_first_child()
                .expect("with expression is missing an identifier callee")
        } else if let Some(Expr::Closure(c)) = self.0.cast_last_child() {
            c.name().expect("closure is missing an identifier name")
        } else {
            panic!("let expression is missing either an identifier or a with expression")
        }
    }

    /// The expression the binding is initialized with.
    pub fn init(&self) -> Option<Expr> {
        if self.0.cast_first_child::<Ident>().is_some() {
            self.0.children().filter_map(RedTicket::cast).nth(1)
        } else {
            Some(
                self.0
                    .cast_first_child()
                    .expect("let expression is missing a with expression"),
            )
        }
    }

    /// The ticket for the expression the binding is initialized with.
    pub fn init_ticket(&self) -> RedTicket {
        if self.0.cast_first_child::<Ident>().is_some() {
            self.0.children().filter(|x| x.cast::<Expr>().is_some()).nth(1)
        } else {
            self.0.children().find(|x| x.cast::<Expr>().is_some())
        }
        .unwrap()
    }
}

node!(
    /// An import expression: `import a, b, c from "utils.typ"`.
    ImportExpr
);

impl ImportExpr {
    /// The items to be imported.
    pub fn imports(&self) -> Imports {
        self.0
            .cast_first_child()
            .expect("import expression is missing import list")
    }

    /// The location of the importable file.
    pub fn path(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("import expression is missing path expression")
    }
}

/// The items that ought to be imported from a file.
#[derive(Debug, Clone, PartialEq)]
pub enum Imports {
    /// All items in the scope of the file should be imported.
    Wildcard,
    /// The specified identifiers from the file should be imported.
    Idents(Vec<Ident>),
}

impl TypedNode for Imports {
    fn cast_from(node: RedTicket) -> Option<Self> {
        match node.kind() {
            NodeKind::Star => Some(Imports::Wildcard),
            NodeKind::ImportItems => {
                let idents = node.own().children().filter_map(RedTicket::cast).collect();
                Some(Imports::Idents(idents))
            }
            _ => None,
        }
    }
}

node!(
    /// An include expression: `include "chapter1.typ"`.
    IncludeExpr
);

impl IncludeExpr {
    /// The location of the file to be included.
    pub fn path(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("include expression is missing path expression")
    }
}

node!(
    /// An if-else expression: `if x { y } else { z }`.
    IfExpr
);

impl IfExpr {
    /// The condition which selects the body to evaluate.
    pub fn condition(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("if expression is missing condition expression")
    }

    /// The expression to evaluate if the condition is true.
    pub fn if_body(&self) -> Expr {
        self.0
            .children()
            .filter_map(RedTicket::cast)
            .nth(1)
            .expect("if expression is missing if body")
    }

    /// The expression to evaluate if the condition is false.
    pub fn else_body(&self) -> Option<Expr> {
        self.0.children().filter_map(RedTicket::cast).nth(2)
    }
}

node!(
    /// A while loop expression: `while x { y }`.
    WhileExpr
);

impl WhileExpr {
    /// The condition which selects whether to evaluate the body.
    pub fn condition(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("while loop expression is missing condition expression")
    }

    /// The expression to evaluate while the condition is true.
    pub fn body(&self) -> Expr {
        self.0
            .children()
            .filter_map(RedTicket::cast)
            .nth(1)
            .expect("while loop expression is missing body")
    }
}

node!(
    /// A for loop expression: `for x in y { z }`.
    ForExpr
);

impl ForExpr {
    /// The pattern to assign to.
    pub fn pattern(&self) -> ForPattern {
        self.0
            .cast_first_child()
            .expect("for loop expression is missing pattern")
    }

    /// The expression to iterate over.
    pub fn iter(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("for loop expression is missing iterable expression")
    }

    /// The expression to evaluate for each iteration.
    pub fn body(&self) -> Expr {
        self.0
            .children()
            .filter_map(RedTicket::cast)
            .last()
            .expect("for loop expression is missing body")
    }

    /// The ticket for the expression to evaluate for each iteration.
    pub fn body_ticket(&self) -> RedTicket {
        self.0
            .children()
            .filter(|x| x.cast::<Expr>().is_some())
            .last()
            .unwrap()
    }
}

node!(
    /// A for-in loop expression: `for x in y { z }`.
    ForPattern
);

impl ForPattern {
    pub fn key(&self) -> Option<Ident> {
        let mut items: Vec<_> = self.0.children().filter_map(RedTicket::cast).collect();
        if items.len() > 1 { Some(items.remove(0)) } else { None }
    }

    pub fn value(&self) -> Ident {
        self.0
            .cast_last_child()
            .expect("for-in loop pattern is missing value")
    }
}
