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

use std::rc::Rc;

use crate::diag::Pass;
use crate::syntax::*;
use collection::{args, parenthesized};

/// Parse a string of source code.
pub fn parse(src: &str) -> Pass<Tree> {
    let mut p = Parser::new(src);
    Pass::new(tree(&mut p), p.diags)
}

/// Parse a syntax tree.
fn tree(p: &mut Parser) -> Tree {
    // We keep track of whether we are at the start of a block or paragraph
    // to know whether headings are allowed.
    let mut at_start = true;
    let mut tree = vec![];
    while !p.eof() {
        if let Some(node) = node(p, &mut at_start) {
            if !matches!(node, Node::Parbreak | Node::Space) {
                at_start = false;
            }
            tree.push(node);
        }
    }
    tree
}

/// Parse a syntax node.
fn node(p: &mut Parser, at_start: &mut bool) -> Option<Node> {
    let token = p.peek()?;
    let node = match token {
        // Whitespace.
        Token::Space(newlines) => {
            *at_start |= newlines > 0;
            if newlines < 2 { Node::Space } else { Node::Parbreak }
        }

        // Text.
        Token::Text(text) => Node::Text(text.into()),

        // Markup.
        Token::Star => Node::Strong,
        Token::Underscore => Node::Emph,
        Token::Eq => {
            if *at_start {
                return Some(heading(p));
            } else {
                Node::Text(p.peek_src().into())
            }
        }
        Token::Tilde => Node::Text("\u{00A0}".into()),
        Token::Backslash => Node::Linebreak,
        Token::Raw(t) => raw(p, t),
        Token::UnicodeEscape(t) => Node::Text(unicode_escape(p, t)),

        // Keywords.
        Token::Let | Token::If | Token::For => {
            *at_start = false;
            let stmt = token == Token::Let;
            let group = if stmt { Group::Stmt } else { Group::Expr };

            p.start_group(group, TokenMode::Code);
            let expr = primary(p);
            if stmt && expr.is_some() && !p.eof() {
                p.expected_at("semicolon or line break", p.end());
            }
            p.end_group();

            // Uneat spaces we might have eaten eagerly.
            p.jump(p.end());
            return expr.map(Node::Expr);
        }

        // Block.
        Token::LeftBrace => {
            *at_start = false;
            return Some(Node::Expr(block(p, false)?));
        }

        // Template.
        Token::LeftBracket => {
            *at_start = false;
            return Some(Node::Expr(template(p)));
        }

        // Function template.
        Token::HashBracket => {
            *at_start = false;
            return Some(Node::Expr(bracket_call(p)?));
        }

        // Comments.
        Token::LineComment(_) | Token::BlockComment(_) => {
            p.eat();
            return None;
        }

        _ => {
            *at_start = false;
            p.unexpected();
            return None;
        }
    };
    p.eat();
    Some(node)
}

/// Parse a heading.
fn heading(p: &mut Parser) -> Node {
    let start = p.start();
    p.assert(&[Token::Eq]);

    // Count depth.
    let mut level: usize = 0;
    while p.eat_if(Token::Eq) {
        level += 1;
    }

    if level > 5 {
        p.diag(warning!(start .. p.end(), "should not exceed depth 6"));
        level = 5;
    }

    // Parse the heading contents.
    let mut contents = vec![];
    while p.check(|t| !matches!(t, Token::Space(n) if n >= 1)) {
        contents.extend(node(p, &mut false));
    }

    Node::Heading(NodeHeading { level, contents })
}

/// Handle a raw block.
fn raw(p: &mut Parser, token: TokenRaw) -> Node {
    let raw = resolve::resolve_raw(token.text, token.backticks, p.start());
    if !token.terminated {
        p.diag(error!(p.peek_span().end, "expected backtick(s)"));
    }
    Node::Raw(raw)
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
fn bracket_call(p: &mut Parser) -> Option<Expr> {
    p.start_group(Group::Bracket, TokenMode::Code);

    // One header is guaranteed, but there may be more (through chaining).
    let mut outer = vec![];
    let mut inner = bracket_subheader(p);
    while p.eat_if(Token::Pipe) {
        if let Some(new) = bracket_subheader(p) {
            outer.extend(inner);
            inner = Some(new);
        }
    }

    p.end_group();

    let body = match p.peek() {
        Some(Token::LeftBracket) => Some(bracket_body(p)),
        _ => None,
    };

    let mut inner = inner?;
    if let Some(body) = body {
        inner.span.expand(body.span());
        inner.args.items.push(ExprArg::Pos(body));
    }

    while let Some(mut top) = outer.pop() {
        top.args.items.push(ExprArg::Pos(Expr::Call(inner)));
        inner = top;
    }

    Some(Expr::Call(inner))
}

/// Parse one subheader of a bracketed function call.
fn bracket_subheader(p: &mut Parser) -> Option<ExprCall> {
    p.start_group(Group::Subheader, TokenMode::Code);
    let name = ident(p);
    let args = args(p);
    let span = p.end_group();
    Some(ExprCall {
        span,
        callee: Box::new(Expr::Ident(name?)),
        args,
    })
}

/// Parse the body of a bracketed function call.
fn bracket_body(p: &mut Parser) -> Expr {
    p.start_group(Group::Bracket, TokenMode::Markup);
    let tree = Rc::new(tree(p));
    let span = p.end_group();
    Expr::Template(ExprTemplate { span, tree })
}

/// Parse an expression.
fn expr(p: &mut Parser) -> Option<Expr> {
    expr_with(p, 0)
}

/// Parse an expression with operators having at least the minimum precedence.
fn expr_with(p: &mut Parser, min_prec: usize) -> Option<Expr> {
    let start = p.start();
    let mut lhs = match p.eat_map(UnOp::from_token) {
        Some(op) => {
            let prec = op.precedence();
            let expr = Box::new(expr_with(p, prec)?);
            Expr::Unary(ExprUnary { span: p.span_from(start), op, expr })
        }
        None => primary(p)?,
    };

    loop {
        let op = match p.peek().and_then(BinOp::from_token) {
            Some(binop) => binop,
            None => break,
        };

        let mut prec = op.precedence();
        if prec < min_prec {
            break;
        }

        p.eat();
        match op.associativity() {
            Associativity::Left => prec += 1,
            Associativity::Right => {}
        }

        let rhs = match expr_with(p, prec) {
            Some(rhs) => Box::new(rhs),
            None => break,
        };

        let span = lhs.span().join(rhs.span());
        lhs = Expr::Binary(ExprBinary { span, lhs: Box::new(lhs), op, rhs });
    }

    Some(lhs)
}

/// Parse a primary expression.
fn primary(p: &mut Parser) -> Option<Expr> {
    if let Some(expr) = literal(p) {
        return Some(expr);
    }

    match p.peek() {
        // Function or identifier.
        Some(Token::Ident(string)) => {
            let ident = Ident {
                span: p.eat_span(),
                string: string.into(),
            };
            if p.peek() == Some(Token::LeftParen) {
                Some(paren_call(p, ident))
            } else {
                Some(Expr::Ident(ident))
            }
        }

        // Keywords.
        Some(Token::Let) => expr_let(p),
        Some(Token::If) => expr_if(p),
        Some(Token::For) => expr_for(p),

        // Structures.
        Some(Token::LeftBrace) => block(p, true),
        Some(Token::LeftBracket) => Some(template(p)),
        Some(Token::HashBracket) => bracket_call(p),
        Some(Token::LeftParen) => Some(parenthesized(p)),

        // Nothing.
        _ => {
            p.expected("expression");
            None
        }
    }
}

/// Parse a literal.
fn literal(p: &mut Parser) -> Option<Expr> {
    let kind = match p.peek()? {
        // Basic values.
        Token::None => LitKind::None,
        Token::Bool(b) => LitKind::Bool(b),
        Token::Int(i) => LitKind::Int(i),
        Token::Float(f) => LitKind::Float(f),
        Token::Length(val, unit) => LitKind::Length(val, unit),
        Token::Angle(val, unit) => LitKind::Angle(val, unit),
        Token::Percent(p) => LitKind::Percent(p),
        Token::Color(color) => LitKind::Color(color),
        Token::Str(token) => LitKind::Str(string(p, token)),
        _ => return None,
    };
    Some(Expr::Lit(Lit { span: p.eat_span(), kind }))
}

// Parse a template value: `[...]`.
fn template(p: &mut Parser) -> Expr {
    p.start_group(Group::Bracket, TokenMode::Markup);
    let tree = Rc::new(tree(p));
    let span = p.end_group();
    Expr::Template(ExprTemplate { span, tree })
}

/// Parse a block expression: `{...}`.
fn block(p: &mut Parser, scopes: bool) -> Option<Expr> {
    p.start_group(Group::Brace, TokenMode::Code);
    let mut exprs = vec![];
    while !p.eof() {
        p.start_group(Group::Stmt, TokenMode::Code);
        if let Some(expr) = expr(p) {
            exprs.push(expr);
            if !p.eof() {
                p.expected_at("semicolon or line break", p.end());
            }
        }
        p.end_group();
        p.skip_white();
    }
    let span = p.end_group();
    Some(Expr::Block(ExprBlock { span, exprs, scoping: scopes }))
}

/// Parse a parenthesized function call.
fn paren_call(p: &mut Parser, name: Ident) -> Expr {
    p.start_group(Group::Paren, TokenMode::Code);
    let args = args(p);
    p.end_group();
    Expr::Call(ExprCall {
        span: p.span_from(name.span.start),
        callee: Box::new(Expr::Ident(name)),
        args,
    })
}

/// Parse a string.
fn string(p: &mut Parser, token: TokenStr) -> String {
    if !token.terminated {
        p.expected_at("quote", p.peek_span().end);
    }
    resolve::resolve_string(token.string)
}

/// Parse a let expression.
fn expr_let(p: &mut Parser) -> Option<Expr> {
    let start = p.start();
    p.assert(&[Token::Let]);

    let mut expr_let = None;
    if let Some(binding) = ident(p) {
        let mut init = None;
        if p.eat_if(Token::Eq) {
            init = expr(p);
        }

        expr_let = Some(Expr::Let(ExprLet {
            span: p.span_from(start),
            binding,
            init: init.map(Box::new),
        }))
    }

    expr_let
}

/// Parse an if expresion.
fn expr_if(p: &mut Parser) -> Option<Expr> {
    let start = p.start();
    p.assert(&[Token::If]);

    let mut expr_if = None;
    if let Some(condition) = expr(p) {
        if let Some(if_body) = body(p) {
            let mut else_body = None;
            if p.eat_if(Token::Else) {
                else_body = body(p);
            }

            expr_if = Some(Expr::If(ExprIf {
                span: p.span_from(start),
                condition: Box::new(condition),
                if_body: Box::new(if_body),
                else_body: else_body.map(Box::new),
            }));
        }
    }

    expr_if
}

/// Parse a for expression.
fn expr_for(p: &mut Parser) -> Option<Expr> {
    let start = p.start();
    p.assert(&[Token::For]);

    let mut expr_for = None;
    if let Some(pattern) = for_pattern(p) {
        if p.expect(Token::In) {
            if let Some(iter) = expr(p) {
                if let Some(body) = body(p) {
                    expr_for = Some(Expr::For(ExprFor {
                        span: p.span_from(start),
                        pattern,
                        iter: Box::new(iter),
                        body: Box::new(body),
                    }));
                }
            }
        }
    }

    expr_for
}

/// Parse a for loop pattern.
fn for_pattern(p: &mut Parser) -> Option<ForPattern> {
    let first = ident(p)?;
    if p.eat_if(Token::Comma) {
        if let Some(second) = ident(p) {
            return Some(ForPattern::KeyValue(first, second));
        }
    }
    Some(ForPattern::Value(first))
}

/// Parse an identifier.
fn ident(p: &mut Parser) -> Option<Ident> {
    if let Some(Token::Ident(string)) = p.peek() {
        Some(Ident {
            span: p.eat_span(),
            string: string.to_string(),
        })
    } else {
        p.expected("identifier");
        None
    }
}

/// Parse a control flow body.
fn body(p: &mut Parser) -> Option<Expr> {
    match p.peek() {
        Some(Token::LeftBracket) => Some(template(p)),
        Some(Token::LeftBrace) => block(p, true),
        _ => {
            p.expected_at("body", p.end());
            None
        }
    }
}
