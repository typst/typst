use super::*;

/// Parse the arguments to a function call.
pub fn args(p: &mut Parser) -> ExprArgs {
    let start = p.start();
    let items = collection(p, vec![]);
    ExprArgs { span: p.span_from(start), items }
}

/// Parse a parenthesized group, which can be either of:
/// - Array literal
/// - Dictionary literal
/// - Parenthesized expression
pub fn parenthesized(p: &mut Parser) -> Expr {
    p.start_group(Group::Paren, TokenMode::Code);
    let state = if p.eat_if(Token::Colon) {
        collection(p, State::Dict(vec![]))
    } else {
        collection(p, State::Unknown)
    };
    let span = p.end_group();
    state.into_expr(span)
}

/// Parse a collection.
fn collection<T: Collection>(p: &mut Parser, mut collection: T) -> T {
    let mut missing_coma = None;

    while !p.eof() {
        if let Some(arg) = argument(p) {
            collection.push_arg(p, arg);

            if let Some(pos) = missing_coma.take() {
                p.expected_at("comma", pos);
            }

            if p.eof() {
                break;
            }

            let behind = p.end();
            if p.eat_if(Token::Comma) {
                collection.push_comma();
            } else {
                missing_coma = Some(behind);
            }
        }
    }

    collection
}

/// Parse an expression or a named pair.
fn argument(p: &mut Parser) -> Option<ExprArg> {
    let first = expr(p)?;
    if p.eat_if(Token::Colon) {
        if let Expr::Ident(name) = first {
            Some(ExprArg::Named(Named { name, expr: expr(p)? }))
        } else {
            p.diag(error!(first.span(), "expected identifier"));
            expr(p);
            None
        }
    } else {
        Some(ExprArg::Pos(first))
    }
}

/// Abstraction for comma-separated list of expression / named pairs.
trait Collection {
    fn push_arg(&mut self, p: &mut Parser, arg: ExprArg);
    fn push_comma(&mut self) {}
}

impl Collection for Vec<ExprArg> {
    fn push_arg(&mut self, _: &mut Parser, arg: ExprArg) {
        self.push(arg);
    }
}

/// State of collection parsing.
#[derive(Debug)]
enum State {
    Unknown,
    Expr(Expr),
    Array(Vec<Expr>),
    Dict(Vec<Named>),
}

impl State {
    fn into_expr(self, span: Span) -> Expr {
        match self {
            Self::Unknown => Expr::Array(ExprArray { span, items: vec![] }),
            Self::Expr(expr) => Expr::Group(ExprGroup { span, expr: Box::new(expr) }),
            Self::Array(items) => Expr::Array(ExprArray { span, items }),
            Self::Dict(items) => Expr::Dict(ExprDict { span, items }),
        }
    }
}

impl Collection for State {
    fn push_arg(&mut self, p: &mut Parser, arg: ExprArg) {
        match self {
            Self::Unknown => match arg {
                ExprArg::Pos(expr) => *self = Self::Expr(expr),
                ExprArg::Named(named) => *self = Self::Dict(vec![named]),
            },
            Self::Expr(prev) => match arg {
                ExprArg::Pos(expr) => *self = Self::Array(vec![take(prev), expr]),
                ExprArg::Named(_) => diag(p, arg),
            },
            Self::Array(array) => match arg {
                ExprArg::Pos(expr) => array.push(expr),
                ExprArg::Named(_) => diag(p, arg),
            },
            Self::Dict(dict) => match arg {
                ExprArg::Pos(_) => diag(p, arg),
                ExprArg::Named(named) => dict.push(named),
            },
        }
    }

    fn push_comma(&mut self) {
        if let Self::Expr(expr) = self {
            *self = Self::Array(vec![take(expr)]);
        }
    }
}

fn take(expr: &mut Expr) -> Expr {
    // Replace with anything, it's overwritten anyway.
    std::mem::replace(
        expr,
        Expr::Lit(Lit { span: Span::ZERO, kind: LitKind::None }),
    )
}

fn diag(p: &mut Parser, arg: ExprArg) {
    p.diag(error!(arg.span(), "{}", match arg {
        ExprArg::Pos(_) => "expected named pair, found expression",
        ExprArg::Named(_) => "expected expression, found named pair",
    }));
}
