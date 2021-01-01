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
        if let Some(node) = p.span_if(|p| node(p, at_start)) {
            match node.v {
                SynNode::Parbreak => at_start = true,
                SynNode::Space => {}
                _ => at_start = false,
            }
            tree.push(node);
        }
    }
    tree
}

/// Parse a syntax node.
fn node(p: &mut Parser, at_start: bool) -> Option<SynNode> {
    let node = match p.peek()? {
        Token::Space(newlines) => {
            if newlines < 2 {
                SynNode::Space
            } else {
                SynNode::Parbreak
            }
        }
        Token::Text(text) => SynNode::Text(text.into()),

        Token::LineComment(_) | Token::BlockComment(_) => {
            p.eat();
            return None;
        }

        Token::Star => SynNode::Strong,
        Token::Underscore => SynNode::Emph,
        Token::Tilde => SynNode::Text("\u{00A0}".into()),
        Token::Backslash => SynNode::Linebreak,
        Token::Hashtag => {
            if at_start {
                return Some(SynNode::Heading(heading(p)));
            } else {
                SynNode::Text(p.get(p.peek_span()).into())
            }
        }
        Token::Raw(t) => SynNode::Raw(raw(p, t)),
        Token::UnicodeEscape(t) => SynNode::Text(unicode_escape(p, t)),

        Token::LeftBracket => {
            return Some(SynNode::Expr(Expr::Call(bracket_call(p))));
        }

        Token::LeftBrace => {
            return Some(SynNode::Expr(block_expr(p)?));
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
        p.eat_assert(Token::Hashtag);

        let mut level = 0u8;
        while p.eat_if(Token::Hashtag) {
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
    while p.check(|t| !matches!(t, Token::Space(n) if n >= 1)) {
        if let Some(node) = p.span_if(|p| node(p, false)) {
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

/// Parse a block expression.
fn block_expr(p: &mut Parser) -> Option<Expr> {
    p.push_mode(TokenMode::Header);
    p.start_group(Group::Brace);
    let expr = expr(p);
    p.pop_mode();
    p.end_group();
    expr
}

/// Parse a parenthesized function call.
fn paren_call(p: &mut Parser, name: Spanned<Ident>) -> ExprCall {
    p.start_group(Group::Paren);
    let args = p.span(|p| dict_contents(p).0);
    p.end_group();
    ExprCall { name, args }
}

/// Parse a bracketed function call.
fn bracket_call(p: &mut Parser) -> ExprCall {
    p.push_mode(TokenMode::Header);
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
        let expr = p.span(|p| Expr::Lit(Lit::Content(bracket_body(p))));
        inner.span.expand(expr.span);
        inner.v.args.v.0.push(LitDictEntry { key: None, expr });
    }

    while let Some(mut top) = outer.pop() {
        let span = inner.span;
        let node = inner.map(|c| SynNode::Expr(Expr::Call(c)));
        let expr = Expr::Lit(Lit::Content(vec![node])).with_span(span);
        top.v.args.v.0.push(LitDictEntry { key: None, expr });
        inner = top;
    }

    inner.v
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

    let args = p.span(|p| dict_contents(p).0);

    p.end_group();
    ExprCall { name, args }
}

/// Parse the body of a bracketed function call.
fn bracket_body(p: &mut Parser) -> SynTree {
    p.push_mode(TokenMode::Body);
    p.start_group(Group::Bracket);
    let tree = tree(p);
    p.pop_mode();
    p.end_group();
    tree
}

/// Parse the contents of a dictionary.
fn dict_contents(p: &mut Parser) -> (LitDict, bool) {
    let mut dict = LitDict::new();
    let mut missing_coma = None;
    let mut comma_and_keyless = true;

    while !p.eof() {
        if let Some(entry) = dict_entry(p) {
            let behind = entry.expr.span.end;
            if let Some(pos) = missing_coma.take() {
                p.diag_expected_at("comma", pos);
            }

            if let Some(key) = &entry.key {
                comma_and_keyless = false;
                p.deco(Deco::Name.with_span(key.span));
            }

            dict.0.push(entry);
            if p.eof() {
                break;
            }

            if p.eat_if(Token::Comma) {
                comma_and_keyless = false;
            } else {
                missing_coma = Some(behind);
            }
        }
    }

    let coercible = comma_and_keyless && !dict.0.is_empty();
    (dict, coercible)
}

/// Parse a single entry in a dictionary.
fn dict_entry(p: &mut Parser) -> Option<LitDictEntry> {
    if let Some(ident) = p.span_if(ident) {
        match p.peek() {
            // Key-value pair.
            Some(Token::Colon) => {
                p.eat_assert(Token::Colon);
                p.span_if(expr).map(|expr| LitDictEntry {
                    key: Some(ident.map(|id| DictKey::Str(id.0))),
                    expr,
                })
            }

            // Function call.
            Some(Token::LeftParen) => Some(LitDictEntry {
                key: None,
                expr: {
                    let start = ident.span.start;
                    let call = paren_call(p, ident);
                    Expr::Call(call).with_span(start .. p.last_end())
                },
            }),

            // Just an identifier.
            _ => Some(LitDictEntry {
                key: None,
                expr: ident.map(|id| Expr::Lit(Lit::Ident(id))),
            }),
        }
    } else {
        p.span_if(expr).map(|expr| LitDictEntry { key: None, expr })
    }
}

/// Parse an expression: `term (+ term)*`.
fn expr(p: &mut Parser) -> Option<Expr> {
    binops(p, term, |token| match token {
        Token::Plus => Some(BinOp::Add),
        Token::Hyphen => Some(BinOp::Sub),
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
        Token::Hyphen => Some(UnOp::Neg),
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
        // Bracketed function call.
        Some(Token::LeftBracket) => {
            let node = p.span(|p| SynNode::Expr(Expr::Call(bracket_call(p))));
            return Some(Expr::Lit(Lit::Content(vec![node])));
        }

        // Content expression.
        Some(Token::LeftBrace) => {
            return Some(Expr::Lit(Lit::Content(content(p))));
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
                return Some(Expr::Call(paren_call(p, name)));
            } else {
                return Some(Expr::Lit(Lit::Ident(ident)));
            }
        }

        // Basic values.
        Some(Token::Bool(b)) => Expr::Lit(Lit::Bool(b)),
        Some(Token::Int(i)) => Expr::Lit(Lit::Int(i)),
        Some(Token::Float(f)) => Expr::Lit(Lit::Float(f)),
        Some(Token::Length(val, unit)) => Expr::Lit(Lit::Length(val, unit)),
        Some(Token::Percent(p)) => Expr::Lit(Lit::Percent(p)),
        Some(Token::Hex(hex)) => Expr::Lit(Lit::Color(color(p, hex))),
        Some(Token::Str(token)) => Expr::Lit(Lit::Str(str(p, token))),

        // No value.
        _ => {
            p.diag_expected("expression");
            return None;
        }
    };
    p.eat();
    Some(expr)
}

// Parse a content value: `{...}`.
fn content(p: &mut Parser) -> SynTree {
    p.push_mode(TokenMode::Body);
    p.start_group(Group::Brace);
    let tree = tree(p);
    p.pop_mode();
    p.end_group();
    tree
}

/// Parse a parenthesized expression: `(a + b)`, `(1, name: "value").
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
fn color(p: &mut Parser, hex: &str) -> RgbaColor {
    RgbaColor::from_str(hex).unwrap_or_else(|_| {
        // Replace color with black.
        p.diag(error!(p.peek_span(), "invalid color"));
        RgbaColor::new(0, 0, 0, 255)
    })
}

/// Parse a string.
fn str(p: &mut Parser, token: TokenStr) -> String {
    if !token.terminated {
        p.diag_expected_at("quote", p.peek_span().end);
    }

    resolve::resolve_string(token.string)
}

#[cfg(test)]
mod tests;
