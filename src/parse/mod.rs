//! Parsing and tokenization.

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
use crate::diag::{Deco, Pass};
use crate::eval::DictKey;
use crate::syntax::*;

/// Parse a string of source code.
pub fn parse(src: &str) -> Pass<SynTree> {
    let mut p = Parser::new(src);
    Pass::new(tree(&mut p), p.finish())
}

/// Parse a syntax tree.
fn tree(p: &mut Parser) -> SynTree {
    // We keep track of whether we are at the start of a block or paragraph
    // to know whether headings are allowed.
    let mut at_start = true;
    let mut tree = vec![];
    while !p.eof() {
        if let Some(node) = node(p, at_start) {
            if node.v == SynNode::Parbreak {
                at_start = true;
            } else if node.v != SynNode::Space {
                at_start = false;
            }
            tree.push(node);
        }
    }
    tree
}

/// Parse a syntax node.
fn node(p: &mut Parser, at_start: bool) -> Option<Spanned<SynNode>> {
    let start = p.pos();
    let node = match p.eat()? {
        // Spaces.
        Token::Space(newlines) => {
            if newlines < 2 {
                SynNode::Space
            } else {
                SynNode::Parbreak
            }
        }

        // Text.
        Token::Text(text) => SynNode::Text(text.into()),

        // Comments.
        Token::LineComment(_) | Token::BlockComment(_) => return None,

        // Markup.
        Token::Star => SynNode::Strong,
        Token::Underscore => SynNode::Emph,
        Token::Hashtag => {
            if at_start {
                SynNode::Heading(heading(p, start))
            } else {
                SynNode::Text(p.eaten_from(start).into())
            }
        }
        Token::Tilde => SynNode::Text("\u{00A0}".into()),
        Token::Backslash => SynNode::Linebreak,
        Token::UnicodeEscape(token) => SynNode::Text(unicode_escape(p, token, start)),
        Token::Raw(token) => SynNode::Raw(raw(p, token)),

        // Functions.
        Token::LeftBracket => {
            p.jump(start);
            SynNode::Expr(Expr::Call(bracket_call(p)))
        }

        // Bad tokens.
        _ => {
            p.jump(start);
            p.diag_unexpected();
            return None;
        }
    };
    Some(node.span_with(start .. p.pos()))
}

/// Parse a heading.
fn heading(p: &mut Parser, start: Pos) -> NodeHeading {
    // Parse the section depth.
    let count = p.eat_while(|c| c == Token::Hashtag);
    let span = Span::new(start, p.pos());
    let level = (count.min(5) as u8).span_with(span);
    if count > 5 {
        p.diag(warning!(span, "section depth should be at most 6"));
    }

    // Parse the heading contents.
    let mut contents = vec![];
    while p.check(|t| !matches!(t, Token::Space(n) if n >= 1)) {
        if let Some(node) = node(p, false) {
            contents.push(node);
        }
    }

    NodeHeading { level, contents }
}

/// Parse a raw block.
fn raw(p: &mut Parser, token: TokenRaw) -> NodeRaw {
    let raw = resolve::resolve_raw(token.text, token.backticks);

    if !token.terminated {
        p.diag(error!(p.pos(), "expected backtick(s)"));
    }

    raw
}

/// Parse a unicode escape sequence.
fn unicode_escape(p: &mut Parser, token: TokenUnicodeEscape, start: Pos) -> String {
    let span = Span::new(start, p.pos());
    let text = if let Some(c) = resolve::resolve_hex(token.sequence) {
        c.to_string()
    } else {
        // Print out the escape sequence verbatim if it is
        // invalid.
        p.diag(error!(span, "invalid unicode escape sequence"));
        p.get(span).into()
    };

    if !token.terminated {
        p.diag(error!(span.end, "expected closing brace"));
    }

    text
}

/// Parse a bracketed function call.
fn bracket_call(p: &mut Parser) -> ExprCall {
    p.start_group(Group::Bracket);
    p.push_mode(TokenMode::Header);

    // One header is guaranteed, but there may be more (through chaining).
    let mut outer = vec![];
    let mut inner = p.span(|p| bracket_subheader(p));

    while p.eat_if(Token::Chain) {
        outer.push(inner);
        inner = p.span(|p| bracket_subheader(p));
    }

    p.pop_mode();
    p.end_group();

    if p.peek() == Some(Token::LeftBracket) {
        let expr = p.span(|p| Expr::Lit(Lit::Content(bracket_body(p))));
        inner.span.expand(expr.span);
        inner.v.args.v.0.push(LitDictEntry { key: None, expr });
    }

    while let Some(mut top) = outer.pop() {
        let span = inner.span;
        let node = inner.map(Expr::Call).map(SynNode::Expr);
        let expr = Expr::Lit(Lit::Content(vec![node])).span_with(span);
        top.v.args.v.0.push(LitDictEntry { key: None, expr });
        inner = top;
    }

    inner.v
}

/// Parse one subheader of a bracketed function call.
fn bracket_subheader(p: &mut Parser) -> ExprCall {
    p.start_group(Group::Subheader);
    let start = p.pos();

    p.skip_white();
    let name = p.span(|p| ident(p)).transpose().unwrap_or_else(|| {
        if p.eof() {
            p.diag_expected_at("function name", start);
        } else {
            p.diag_expected("function name");
        }
        Ident(String::new()).span_with(start)
    });

    p.skip_white();
    let args = if p.eat_if(Token::Colon) {
        p.span(|p| dict_contents(p).0)
    } else {
        // Ignore the rest if there's no colon.
        p.span(|p| {
            if !p.eof() {
                p.diag_expected_at("colon", p.pos());
            }
            p.eat_while(|_| true);
            LitDict::new()
        })
    };

    p.end_group();
    ExprCall { name, args }
}

/// Parse the body of a bracketed function call.
fn bracket_body(p: &mut Parser) -> SynTree {
    p.start_group(Group::Bracket);
    p.push_mode(TokenMode::Body);
    let tree = tree(p);
    p.pop_mode();
    p.end_group();
    tree
}

/// Parse a parenthesized function call.
fn paren_call(p: &mut Parser, name: Spanned<Ident>) -> ExprCall {
    p.start_group(Group::Paren);
    let args = p.span(|p| dict_contents(p).0);
    p.end_group();
    ExprCall { name, args }
}

/// Parse the contents of a dictionary.
fn dict_contents(p: &mut Parser) -> (LitDict, bool) {
    let mut dict = LitDict::new();
    let mut comma_and_keyless = true;
    let mut expected_comma = None;

    loop {
        p.skip_white();
        if p.eof() {
            break;
        }

        let entry = if let Some(entry) = dict_entry(p) {
            entry
        } else {
            expected_comma = None;
            p.diag_unexpected();
            continue;
        };

        if let Some(pos) = expected_comma.take() {
            p.diag_expected_at("comma", pos);
        }

        if let Some(key) = &entry.key {
            comma_and_keyless = false;
            p.deco(Deco::DictKey.span_with(key.span));
        }

        let behind = entry.expr.span.end;
        dict.0.push(entry);

        p.skip_white();
        if p.eof() {
            break;
        }

        if !p.eat_if(Token::Comma) {
            expected_comma = Some(behind);
        }

        comma_and_keyless = false;
    }

    let coercible = comma_and_keyless && !dict.0.is_empty();
    (dict, coercible)
}

/// Parse a single entry in a dictionary.
fn dict_entry(p: &mut Parser) -> Option<LitDictEntry> {
    if let Some(ident) = p.span(|p| ident(p)).transpose() {
        p.skip_white();
        match p.peek() {
            // Key-value pair.
            Some(Token::Equals) => {
                p.eat_assert(Token::Equals);
                p.skip_white();
                if let Some(expr) = expr(p) {
                    Some(LitDictEntry {
                        key: Some(ident.map(|id| DictKey::Str(id.0))),
                        expr,
                    })
                } else {
                    p.diag_expected("value");
                    None
                }
            }

            // Function call.
            Some(Token::LeftParen) => Some(LitDictEntry {
                key: None,
                expr: {
                    let start = ident.span.start;
                    let call = paren_call(p, ident);
                    Expr::Call(call).span_with(start .. p.pos())
                },
            }),

            // Just an identifier.
            _ => Some(LitDictEntry {
                key: None,
                expr: ident.map(|id| Expr::Lit(Lit::Ident(id))),
            }),
        }
    } else if let Some(expr) = expr(p) {
        Some(LitDictEntry { key: None, expr })
    } else {
        None
    }
}

/// Parse an expression: `term (+ term)*`.
fn expr(p: &mut Parser) -> Option<Spanned<Expr>> {
    binops(p, "summand", term, |token| match token {
        Token::Plus => Some(BinOp::Add),
        Token::Hyphen => Some(BinOp::Sub),
        _ => None,
    })
}

/// Parse a term: `factor (* factor)*`.
fn term(p: &mut Parser) -> Option<Spanned<Expr>> {
    binops(p, "factor", factor, |token| match token {
        Token::Star => Some(BinOp::Mul),
        Token::Slash => Some(BinOp::Div),
        _ => None,
    })
}

/// Parse binary operations of the from `a (<op> b)*`.
fn binops(
    p: &mut Parser,
    operand_name: &str,
    operand: fn(&mut Parser) -> Option<Spanned<Expr>>,
    op: fn(Token) -> Option<BinOp>,
) -> Option<Spanned<Expr>> {
    let mut lhs = operand(p)?;

    loop {
        p.skip_white();
        if let Some(op) = p.span(|p| p.eat_map(op)).transpose() {
            p.skip_white();

            if let Some(rhs) = operand(p) {
                let span = lhs.span.join(rhs.span);
                let expr = Expr::Binary(ExprBinary {
                    lhs: lhs.map(Box::new),
                    op,
                    rhs: rhs.map(Box::new),
                });
                lhs = expr.span_with(span);
                p.skip_white();
            } else {
                let span = lhs.span.join(op.span);
                p.diag(error!(span, "missing right {}", operand_name));
                break;
            }
        } else {
            break;
        }
    }

    Some(lhs)
}

/// Parse a factor of the form `-?value`.
fn factor(p: &mut Parser) -> Option<Spanned<Expr>> {
    let op = |token| match token {
        Token::Hyphen => Some(UnOp::Neg),
        _ => None,
    };

    p.span(|p| {
        if let Some(op) = p.span(|p| p.eat_map(op)).transpose() {
            p.skip_white();
            if let Some(expr) = factor(p) {
                Some(Expr::Unary(ExprUnary { op, expr: expr.map(Box::new) }))
            } else {
                p.diag(error!(op.span, "missing factor"));
                None
            }
        } else {
            value(p)
        }
    })
    .transpose()
}

/// Parse a value.
fn value(p: &mut Parser) -> Option<Expr> {
    let start = p.pos();
    Some(match p.eat()? {
        // Bracketed function call.
        Token::LeftBracket => {
            p.jump(start);
            let node = p.span(|p| SynNode::Expr(Expr::Call(bracket_call(p))));
            Expr::Lit(Lit::Content(vec![node]))
        }

        // Content expression.
        Token::LeftBrace => {
            p.jump(start);
            Expr::Lit(Lit::Content(content(p)))
        }

        // Dictionary or just a parenthesized expression.
        Token::LeftParen => {
            p.jump(start);
            parenthesized(p)
        }

        // Function or just ident.
        Token::Ident(id) => {
            let ident = Ident(id.into());
            let after = p.pos();

            p.skip_white();
            if p.peek() == Some(Token::LeftParen) {
                let name = ident.span_with(start .. after);
                Expr::Call(paren_call(p, name))
            } else {
                Expr::Lit(Lit::Ident(ident))
            }
        }

        // Atomic values.
        Token::Bool(b) => Expr::Lit(Lit::Bool(b)),
        Token::Int(i) => Expr::Lit(Lit::Int(i)),
        Token::Float(f) => Expr::Lit(Lit::Float(f)),
        Token::Length(val, unit) => Expr::Lit(Lit::Length(val, unit)),
        Token::Percent(p) => Expr::Lit(Lit::Percent(p)),
        Token::Hex(hex) => Expr::Lit(Lit::Color(color(p, hex, start))),
        Token::Str(token) => Expr::Lit(Lit::Str(string(p, token))),

        // No value.
        _ => {
            p.jump(start);
            return None;
        }
    })
}

// Parse a content expression: `{...}`.
fn content(p: &mut Parser) -> SynTree {
    p.start_group(Group::Brace);
    p.push_mode(TokenMode::Body);
    let tree = tree(p);
    p.pop_mode();
    p.end_group();
    tree
}

/// Parse a parenthesized expression: `(a + b)`, `(1, key="value").
fn parenthesized(p: &mut Parser) -> Expr {
    p.start_group(Group::Paren);
    let (dict, coercible) = dict_contents(p);
    let expr = if coercible {
        dict.0.into_iter().next().expect("dict is coercible").expr.v
    } else {
        Expr::Lit(Lit::Dict(dict))
    };
    p.end_group();
    expr
}

/// Parse an identifier.
fn ident(p: &mut Parser) -> Option<Ident> {
    p.eat_map(|token| match token {
        Token::Ident(id) => Some(Ident(id.into())),
        _ => None,
    })
}

/// Parse a color.
fn color(p: &mut Parser, hex: &str, start: Pos) -> RgbaColor {
    RgbaColor::from_str(hex).unwrap_or_else(|_| {
        // Heal color by assuming black.
        p.diag(error!(start .. p.pos(), "invalid color"));
        RgbaColor::new_healed(0, 0, 0, 255)
    })
}

/// Parse a string.
fn string(p: &mut Parser, token: TokenStr) -> String {
    if !token.terminated {
        p.diag_expected_at("quote", p.pos());
    }

    resolve::resolve_string(token.string)
}

#[cfg(test)]
mod tests;
