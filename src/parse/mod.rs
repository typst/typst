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
use crate::eval::DictKey;
use crate::syntax::*;
use crate::Pass;

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
    let token = p.eat()?;
    let span = token.span;
    Some(match token.v {
        // Spaces.
        Token::Space(newlines) => {
            if newlines < 2 {
                SynNode::Space.span_with(span)
            } else {
                SynNode::Parbreak.span_with(span)
            }
        }
        Token::Text(text) => SynNode::Text(text.into()).span_with(span),

        // Comments.
        Token::LineComment(_) | Token::BlockComment(_) => return None,

        // Markup.
        Token::Star => SynNode::ToggleBolder.span_with(span),
        Token::Underscore => SynNode::ToggleItalic.span_with(span),
        Token::Backslash => SynNode::Linebreak.span_with(span),
        Token::Hashtag => {
            if at_start {
                heading(p, span.start).map(SynNode::Heading)
            } else {
                SynNode::Text(p.get(span).into()).span_with(span)
            }
        }
        Token::Raw(token) => raw(p, token, span).map(SynNode::Raw),
        Token::UnicodeEscape(token) => unicode_escape(p, token, span).map(SynNode::Text),

        // Functions.
        Token::LeftBracket => {
            p.jump(span.start);
            bracket_call(p).map(Expr::Call).map(SynNode::Expr)
        }

        // Bad tokens.
        _ => {
            p.diag_unexpected(token);
            return None;
        }
    })
}

/// Parse a heading.
fn heading(p: &mut Parser, start: Pos) -> Spanned<NodeHeading> {
    // Parse the section depth.
    let count = p.eat_while(|c| c == Token::Hashtag);
    let span = (start, p.pos());
    let level = (count.min(5) as u8).span_with(span);
    if count > 5 {
        p.diag(warning!(span, "section depth larger than 6 has no effect"));
    }

    // Parse the heading contents.
    p.skip_white();
    let mut contents = vec![];
    while p.check(|t| !matches!(t, Token::Space(n) if n >= 1)) {
        if let Some(node) = node(p, false) {
            contents.push(node);
        }
    }

    NodeHeading { level, contents }.span_with((start, p.pos()))
}

/// Parse a raw block.
fn raw(p: &mut Parser, token: TokenRaw, span: Span) -> Spanned<NodeRaw> {
    let raw = resolve::resolve_raw(token.text, token.backticks);

    if !token.terminated {
        p.diag(error!(span.end, "expected backtick(s)"));
    }

    raw.span_with(span)
}

/// Parse a unicode escape sequence.
fn unicode_escape(
    p: &mut Parser,
    token: TokenUnicodeEscape,
    span: Span,
) -> Spanned<String> {
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

    text.span_with(span)
}

/// Parse a bracketed function call.
fn bracket_call(p: &mut Parser) -> Spanned<ExprCall> {
    let before_bracket = p.pos();
    p.start_group(Group::Bracket);
    p.push_mode(TokenMode::Header);

    // One header is guaranteed, but there may be more (through chaining).
    let mut outer = vec![];
    let mut inner = bracket_subheader(p);

    while p.eat_if(Token::Chain).is_some() {
        outer.push(inner);
        inner = bracket_subheader(p);
    }

    p.pop_mode();
    p.end_group();

    if p.peek() == Some(Token::LeftBracket) {
        let expr = bracket_body(p).map(Lit::Content).map(Expr::Lit);
        inner.span.expand(expr.span);
        inner.v.args.0.push(LitDictEntry { key: None, expr });
    }

    while let Some(mut top) = outer.pop() {
        let span = inner.span;
        let node = inner.map(Expr::Call).map(SynNode::Expr);
        let expr = Expr::Lit(Lit::Content(vec![node])).span_with(span);
        top.v.args.0.push(LitDictEntry { key: None, expr });
        inner = top;
    }

    inner.v.span_with((before_bracket, p.pos()))
}

/// Parse one subheader of a bracketed function call.
fn bracket_subheader(p: &mut Parser) -> Spanned<ExprCall> {
    p.start_group(Group::Subheader);
    let before_name = p.pos();

    p.skip_white();
    let name = ident(p).unwrap_or_else(|| {
        if p.eof() {
            p.diag_expected_at("function name", before_name);
        } else {
            p.diag_expected("function name");
        }
        Ident(String::new()).span_with(before_name)
    });

    p.skip_white();
    let args = if p.eat_if(Token::Colon).is_some() {
        dict_contents(p).0
    } else {
        // Ignore the rest if there's no colon.
        if !p.eof() {
            p.diag_expected_at("colon", p.pos());
        }
        p.eat_while(|_| true);
        LitDict::new()
    };

    ExprCall { name, args }.span_with(p.end_group())
}

/// Parse the body of a bracketed function call.
fn bracket_body(p: &mut Parser) -> Spanned<SynTree> {
    p.start_group(Group::Bracket);
    p.push_mode(TokenMode::Body);
    let tree = tree(p);
    p.pop_mode();
    tree.span_with(p.end_group())
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
        if let Some(op) = p.eat_map(op) {
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
    if let Some(op) = p.eat_map(|token| match token {
        Token::Hyphen => Some(UnOp::Neg),
        _ => None,
    }) {
        p.skip_white();
        if let Some(expr) = factor(p) {
            let span = op.span.join(expr.span);
            let expr = Expr::Unary(ExprUnary { op, expr: expr.map(Box::new) });
            Some(expr.span_with(span))
        } else {
            p.diag(error!(op.span, "missing factor"));
            None
        }
    } else {
        value(p)
    }
}

/// Parse a value.
fn value(p: &mut Parser) -> Option<Spanned<Expr>> {
    let Spanned { v: token, span } = p.eat()?;
    Some(match token {
        // Bracketed function call.
        Token::LeftBracket => {
            p.jump(span.start);
            let call = bracket_call(p);
            let span = call.span;
            let node = call.map(Expr::Call).map(SynNode::Expr);
            Expr::Lit(Lit::Content(vec![node])).span_with(span)
        }

        // Content expression.
        Token::LeftBrace => {
            p.jump(span.start);
            content(p).map(Lit::Content).map(Expr::Lit)
        }

        // Dictionary or just a parenthesized expression.
        Token::LeftParen => {
            p.jump(span.start);
            parenthesized(p)
        }

        // Function or just ident.
        Token::Ident(id) => {
            let ident = Ident(id.into()).span_with(span);

            p.skip_white();
            if p.peek() == Some(Token::LeftParen) {
                paren_call(p, ident).map(Expr::Call)
            } else {
                ident.map(Lit::Ident).map(Expr::Lit)
            }
        }

        // Atomic values.
        Token::Bool(b) => Expr::Lit(Lit::Bool(b)).span_with(span),
        Token::Number(f) => Expr::Lit(Lit::Float(f)).span_with(span),
        Token::Length(l) => Expr::Lit(Lit::Length(l)).span_with(span),
        Token::Hex(hex) => color(p, hex, span).map(Lit::Color).map(Expr::Lit),
        Token::Str(token) => string(p, token, span).map(Lit::Str).map(Expr::Lit),

        // No value.
        _ => {
            p.jump(span.start);
            return None;
        }
    })
}

// Parse a content expression: `{...}`.
fn content(p: &mut Parser) -> Spanned<SynTree> {
    p.start_group(Group::Brace);
    p.push_mode(TokenMode::Body);
    let tree = tree(p);
    p.pop_mode();
    tree.span_with(p.end_group())
}

/// Parse a parenthesized expression: `(a + b)`, `(1, key="value").
fn parenthesized(p: &mut Parser) -> Spanned<Expr> {
    p.start_group(Group::Paren);
    let (dict, coercable) = dict_contents(p);
    let expr = if coercable {
        dict.0.into_iter().next().expect("dict is coercable").expr.v
    } else {
        Expr::Lit(Lit::Dict(dict))
    };
    expr.span_with(p.end_group())
}

/// Parse a parenthesized function call.
fn paren_call(p: &mut Parser, name: Spanned<Ident>) -> Spanned<ExprCall> {
    p.start_group(Group::Paren);
    let args = dict_contents(p).0;
    let span = name.span.join(p.end_group());
    ExprCall { name, args }.span_with(span)
}

/// Parse the contents of a dictionary.
fn dict_contents(p: &mut Parser) -> (LitDict, bool) {
    let mut dict = LitDict::new();
    let mut comma_and_keyless = true;

    loop {
        p.skip_white();
        if p.eof() {
            break;
        }

        let entry = if let Some(entry) = dict_entry(p) {
            entry
        } else {
            p.diag_expected("value");
            continue;
        };

        if let Some(key) = &entry.key {
            comma_and_keyless = false;
            p.deco(Decoration::DictKey.span_with(key.span));
        }

        let behind = entry.expr.span.end;
        dict.0.push(entry);

        p.skip_white();
        if p.eof() {
            break;
        }

        if p.eat_if(Token::Comma).is_none() {
            p.diag_expected_at("comma", behind);
        }

        comma_and_keyless = false;
    }

    let coercable = comma_and_keyless && !dict.0.is_empty();
    (dict, coercable)
}

/// Parse a single entry in a dictionary.
fn dict_entry(p: &mut Parser) -> Option<LitDictEntry> {
    if let Some(ident) = ident(p) {
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
                    None
                }
            }

            // Function call.
            Some(Token::LeftParen) => Some(LitDictEntry {
                key: None,
                expr: paren_call(p, ident).map(Expr::Call),
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

/// Parse an identifier.
fn ident(p: &mut Parser) -> Option<Spanned<Ident>> {
    p.eat_map(|token| match token {
        Token::Ident(id) => Some(Ident(id.into())),
        _ => None,
    })
}

/// Parse a color.
fn color(p: &mut Parser, hex: &str, span: Span) -> Spanned<RgbaColor> {
    RgbaColor::from_str(hex)
        .unwrap_or_else(|_| {
            // Heal color by assuming black.
            p.diag(error!(span, "invalid color"));
            RgbaColor::new_healed(0, 0, 0, 255)
        })
        .span_with(span)
}

/// Parse a string.
fn string(p: &mut Parser, token: TokenStr, span: Span) -> Spanned<String> {
    if !token.terminated {
        p.diag_expected_at("quote", span.end);
    }

    resolve::resolve_string(token.string).span_with(span)
}

#[cfg(test)]
mod tests;
