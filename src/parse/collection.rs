use super::*;
use crate::diag::Deco;

/// Parse the arguments to a function call.
pub fn arguments(p: &mut Parser) -> ExprArgs {
    collection(p, vec![])
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
    p.end_group();
    state.into_expr()
}

/// Parse a collection.
fn collection<T: Collection>(p: &mut Parser, mut collection: T) -> T {
    let mut missing_coma = None;

    while !p.eof() {
        if let Some(arg) = p.span_if(argument) {
            collection.push_arg(p, arg);

            if let Some(pos) = missing_coma.take() {
                p.expected_at("comma", pos);
            }

            if p.eof() {
                break;
            }

            let behind = p.last_end();
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
fn argument(p: &mut Parser) -> Option<Argument> {
    let first = p.span_if(expr)?;
    if p.eat_if(Token::Colon) {
        if let Expr::Ident(ident) = first.v {
            let expr = p.span_if(expr)?;
            let name = ident.with_span(first.span);
            p.deco(Deco::Name.with_span(name.span));
            Some(Argument::Named(Named { name, expr }))
        } else {
            p.diag(error!(first.span, "expected identifier"));
            expr(p);
            None
        }
    } else {
        Some(Argument::Pos(first))
    }
}

/// Abstraction for comma-separated list of expression / named pairs.
trait Collection {
    fn push_arg(&mut self, p: &mut Parser, arg: Spanned<Argument>);
    fn push_comma(&mut self) {}
}

impl Collection for ExprArgs {
    fn push_arg(&mut self, _: &mut Parser, arg: Spanned<Argument>) {
        self.push(arg.v);
    }
}

/// State of collection parsing.
#[derive(Debug)]
enum State {
    Unknown,
    Expr(Spanned<Expr>),
    Array(ExprArray),
    Dict(ExprDict),
}

impl State {
    fn into_expr(self) -> Expr {
        match self {
            Self::Unknown => Expr::Array(vec![]),
            Self::Expr(expr) => Expr::Group(Box::new(expr)),
            Self::Array(array) => Expr::Array(array),
            Self::Dict(dict) => Expr::Dict(dict),
        }
    }
}

impl Collection for State {
    fn push_arg(&mut self, p: &mut Parser, arg: Spanned<Argument>) {
        match self {
            Self::Unknown => match arg.v {
                Argument::Pos(expr) => *self = Self::Expr(expr),
                Argument::Named(named) => *self = Self::Dict(vec![named]),
            },
            Self::Expr(prev) => match arg.v {
                Argument::Pos(expr) => *self = Self::Array(vec![take(prev), expr]),
                Argument::Named(_) => diag(p, arg),
            },
            Self::Array(array) => match arg.v {
                Argument::Pos(expr) => array.push(expr),
                Argument::Named(_) => diag(p, arg),
            },
            Self::Dict(dict) => match arg.v {
                Argument::Pos(_) => diag(p, arg),
                Argument::Named(named) => dict.push(named),
            },
        }
    }

    fn push_comma(&mut self) {
        if let Self::Expr(expr) = self {
            *self = Self::Array(vec![take(expr)]);
        }
    }
}

fn take(expr: &mut Spanned<Expr>) -> Spanned<Expr> {
    // Replace with anything, it's overwritten anyway.
    std::mem::replace(expr, Spanned::zero(Expr::Bool(false)))
}

fn diag(p: &mut Parser, arg: Spanned<Argument>) {
    p.diag(error!(arg.span, "{}", match arg.v {
        Argument::Pos(_) => "expected named pair, found expression",
        Argument::Named(_) => "expected expression, found named pair",
    }));
}
