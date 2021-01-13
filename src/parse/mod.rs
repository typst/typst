//! Parsing and tokenization.

mod collection;
mod lines;
mod parser;
mod resolve;
mod scanner;
mod tokens;

pub use lines::*;
pub use parser::*;
pub use resolve::*;
pub use scanner::*;
pub use tokens::*;

use std::str::FromStr;

use crate::color::RgbaColor;
use crate::diag::Pass;
use crate::syntax::*;

use collection::{arguments, parenthesized};

/// Parse a string of source code.
pub fn parse(src: &str) -> Pass<Tree> {
    let mut p = Parser::new(src);
    Pass::new(tree(&mut p), p.finish())
}

/// Parse a syntax tree.
fn tree(p: &mut Parser) -> Tree {
    // We keep track of whether we are at the start of a block or paragraph
    // to know whether headings are allowed.
    let mut at_start = true;
    let mut tree = vec![];
    while !p.eof() {
        if let Some(node) = p.span_if(|p| node(p, &mut at_start)) {
            if !matches!(node.v, Node::Parbreak | Node::Space) {
                at_start = false;
            }
            tree.push(node);
        }
    }
    tree
}

/// Parse a syntax node.
fn node(p: &mut Parser, at_start: &mut bool) -> Option<Node> {
    let node = match p.peek()? {
        // Bracket call.
        Token::LeftBracket => {
            return Some(Node::Expr(bracket_call(p)));
        }

        // Code block.
        Token::LeftBrace => {
            return Some(Node::Expr(block(p)?));
        }

        // Markup.
        Token::Star => Node::Strong,
        Token::Underscore => Node::Emph,
        Token::Tilde => Node::Text("\u{00A0}".into()),
        Token::Hash => {
            if *at_start {
                return Some(Node::Heading(heading(p)));
            } else {
                Node::Text(p.get(p.peek_span()).into())
            }
        }
        Token::Backslash => Node::Linebreak,
        Token::Space(newlines) => {
            *at_start |= newlines > 0;
            if newlines < 2 { Node::Space } else { Node::Parbreak }
        }
        Token::Text(text) => Node::Text(text.into()),
        Token::Raw(t) => Node::Raw(raw(p, t)),
        Token::UnicodeEscape(t) => Node::Text(unicode_escape(p, t)),

        // Keywords.
        Token::Let => {
            return Some(Node::Expr(expr_let(p)?));
        }

        // Comments.
        Token::LineComment(_) | Token::BlockComment(_) => {
            p.eat();
            return None;
        }

        _ => {
            p.diag_unexpected();
            return None;
        }
    };
    p.eat();
    Some(node)
}

/// Parse a heading.
fn heading(p: &mut Parser) -> NodeHeading {
    // Count hashtags.
    let mut level = p.span(|p| {
        p.eat_assert(Token::Hash);

        let mut level = 0u8;
        while p.eat_if(Token::Hash) {
            level = level.saturating_add(1);
        }
        level
    });

    if level.v > 5 {
        p.diag(warning!(level.span, "section depth should not exceed 6"));
        level.v = 5;
    }

    // Parse the heading contents.
    let mut contents = vec![];
    while p.check(|t| !matches!(t, Token::Space(n) if n > 0)) {
        if let Some(node) = p.span_if(|p| node(p, &mut false)) {
            contents.push(node);
        }
    }

    NodeHeading { level, contents }
}

/// Handle a raw block.
fn raw(p: &mut Parser, token: TokenRaw) -> NodeRaw {
    let raw = resolve::resolve_raw(token.text, token.backticks);
    if !token.terminated {
        p.diag(error!(p.peek_span().end, "expected backtick(s)"));
    }
    raw
}

/// Handle a unicode escape sequence.
fn unicode_escape(p: &mut Parser, token: TokenUnicodeEscape) -> String {
    let span = p.peek_span();
    let text = if let Some(c) = resolve::resolve_hex(token.sequence) {
        c.to_string()
    } else {
        // Print out the escape sequence verbatim if it is invalid.
        p.diag(error!(span, "invalid unicode escape sequence"));
        p.get(span).into()
    };

    if !token.terminated {
        p.diag(error!(span.end, "expected closing brace"));
    }

    text
}

/// Parse a bracketed function call.
fn bracket_call(p: &mut Parser) -> Expr {
    p.push_mode(TokenMode::Code);
    p.start_group(Group::Bracket);

    // One header is guaranteed, but there may be more (through chaining).
    let mut outer = vec![];
    let mut inner = p.span(bracket_subheader);

    while p.eat_if(Token::Pipe) {
        outer.push(inner);
        inner = p.span(bracket_subheader);
    }

    p.pop_mode();
    p.end_group();

    if p.peek() == Some(Token::LeftBracket) {
        let body = p.span(|p| Expr::Template(bracket_body(p)));
        inner.span.expand(body.span);
        inner.v.args.v.push(Argument::Pos(body));
    }

    while let Some(mut top) = outer.pop() {
        let span = inner.span;
        let node = inner.map(|c| Node::Expr(Expr::Call(c)));
        let expr = Expr::Template(vec![node]).with_span(span);
        top.v.args.v.push(Argument::Pos(expr));
        inner = top;
    }

    Expr::Call(inner.v)
}

/// Parse one subheader of a bracketed function call.
fn bracket_subheader(p: &mut Parser) -> ExprCall {
    p.start_group(Group::Subheader);

    let start = p.next_start();
    let name = p.span_if(ident).unwrap_or_else(|| {
        let what = "function name";
        if p.eof() {
            p.diag_expected_at(what, start);
        } else {
            p.diag_expected(what);
        }
        Ident(String::new()).with_span(start)
    });

    let args = p.span(arguments);
    p.end_group();

    ExprCall { name, args }
}

/// Parse the body of a bracketed function call.
fn bracket_body(p: &mut Parser) -> Tree {
    p.push_mode(TokenMode::Markup);
    p.start_group(Group::Bracket);
    let tree = tree(p);
    p.pop_mode();
    p.end_group();
    tree
}

/// Parse a block expression: `{...}`.
fn block(p: &mut Parser) -> Option<Expr> {
    p.push_mode(TokenMode::Code);
    p.start_group(Group::Brace);
    let expr = expr(p);
    while !p.eof() {
        p.diag_unexpected();
    }
    p.pop_mode();
    p.end_group();
    Some(Expr::Block(Box::new(expr?)))
}

/// Parse an expression: `term (+ term)*`.
fn expr(p: &mut Parser) -> Option<Expr> {
    binops(p, term, |token| match token {
        Token::Plus => Some(BinOp::Add),
        Token::Hyph => Some(BinOp::Sub),
        _ => None,
    })
}

/// Parse a term: `factor (* factor)*`.
fn term(p: &mut Parser) -> Option<Expr> {
    binops(p, factor, |token| match token {
        Token::Star => Some(BinOp::Mul),
        Token::Slash => Some(BinOp::Div),
        _ => None,
    })
}

/// Parse binary operations of the from `a (<op> b)*`.
fn binops(
    p: &mut Parser,
    operand: fn(&mut Parser) -> Option<Expr>,
    op: fn(Token) -> Option<BinOp>,
) -> Option<Expr> {
    let mut lhs = p.span_if(operand)?;

    while let Some(op) = p.span_if(|p| p.eat_map(op)) {
        if let Some(rhs) = p.span_if(operand) {
            let span = lhs.span.join(rhs.span);
            let expr = Expr::Binary(ExprBinary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            });
            lhs = expr.with_span(span);
        } else {
            break;
        }
    }

    Some(lhs.v)
}

/// Parse a factor of the form `-?value`.
fn factor(p: &mut Parser) -> Option<Expr> {
    let op = |token| match token {
        Token::Plus => Some(UnOp::Pos),
        Token::Hyph => Some(UnOp::Neg),
        _ => None,
    };

    if let Some(op) = p.span_if(|p| p.eat_map(op)) {
        p.span_if(factor)
            .map(|expr| Expr::Unary(ExprUnary { op, expr: Box::new(expr) }))
    } else {
        value(p)
    }
}

/// Parse a value.
fn value(p: &mut Parser) -> Option<Expr> {
    let expr = match p.peek() {
        // Template.
        Some(Token::LeftBracket) => {
            return Some(template(p));
        }

        // Nested block.
        Some(Token::LeftBrace) => {
            return block(p);
        }

        // Dictionary or just a parenthesized expression.
        Some(Token::LeftParen) => {
            return Some(parenthesized(p));
        }

        // Function or just ident.
        Some(Token::Ident(id)) => {
            p.eat();
            let ident = Ident(id.into());
            if p.peek() == Some(Token::LeftParen) {
                let name = ident.with_span(p.peek_span());
                return Some(paren_call(p, name));
            } else {
                return Some(Expr::Ident(ident));
            }
        }

        // Basic values.
        Some(Token::None) => Expr::None,
        Some(Token::Bool(b)) => Expr::Bool(b),
        Some(Token::Int(i)) => Expr::Int(i),
        Some(Token::Float(f)) => Expr::Float(f),
        Some(Token::Length(val, unit)) => Expr::Length(val, unit),
        Some(Token::Angle(val, unit)) => Expr::Angle(val, unit),
        Some(Token::Percent(p)) => Expr::Percent(p),
        Some(Token::Hex(hex)) => Expr::Color(color(p, hex)),
        Some(Token::Str(token)) => Expr::Str(string(p, token)),

        // No value.
        _ => {
            p.diag_expected("expression");
            return None;
        }
    };
    p.eat();
    Some(expr)
}

// Parse a template value: `[...]`.
fn template(p: &mut Parser) -> Expr {
    p.push_mode(TokenMode::Markup);
    p.start_group(Group::Bracket);
    let tree = tree(p);
    p.pop_mode();
    p.end_group();
    Expr::Template(tree)
}

/// Parse a parenthesized function call.
fn paren_call(p: &mut Parser, name: Spanned<Ident>) -> Expr {
    p.start_group(Group::Paren);
    let args = p.span(arguments);
    p.end_group();
    Expr::Call(ExprCall { name, args })
}

/// Parse an identifier.
fn ident(p: &mut Parser) -> Option<Ident> {
    p.eat_map(|token| match token {
        Token::Ident(id) => Some(Ident(id.into())),
        _ => None,
    })
}

/// Parse a color.
fn color(p: &mut Parser, hex: &str) -> RgbaColor {
    RgbaColor::from_str(hex).unwrap_or_else(|_| {
        // Replace color with black.
        p.diag(error!(p.peek_span(), "invalid color"));
        RgbaColor::new(0, 0, 0, 255)
    })
}

/// Parse a string.
fn string(p: &mut Parser, token: TokenStr) -> String {
    if !token.terminated {
        p.diag_expected_at("quote", p.peek_span().end);
    }

    resolve::resolve_string(token.string)
}

/// Parse a let expresion.
fn expr_let(p: &mut Parser) -> Option<Expr> {
    p.push_mode(TokenMode::Code);
    p.start_group(Group::Terminated);
    p.eat_assert(Token::Let);

    let pat = p.span_if(ident);
    let mut rhs = None;

    if pat.is_some() {
        if p.eat_if(Token::Eq) {
            if let Some(expr) = p.span_if(expr) {
                rhs = Some(Box::new(expr));
            }
        }
    } else {
        p.diag_expected("identifier");
    }

    while !p.eof() {
        p.diag_unexpected();
    }

    p.pop_mode();
    p.end_group();
    pat.map(|pat| Expr::Let(ExprLet { pat, expr: rhs }))
}

#[cfg(test)]
mod tests;
