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
    let token = p.peek()?;
    let node = match token {
        // Bracket call.
        Token::LeftBracket => {
            return Some(Node::Expr(bracket_call(p)?));
        }

        // Code block.
        Token::LeftBrace => {
            return Some(Node::Expr(block(p, false)?));
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
        Token::Let | Token::If | Token::For => {
            let stmt = token == Token::Let;
            let group = if stmt { Group::Stmt } else { Group::Expr };

            p.start_group(group, TokenMode::Code);
            let expr = primary(p);
            if stmt && expr.is_some() && !p.eof() {
                p.expected_at("semicolon or line break", p.last_end());
            }
            p.end_group();

            // Uneat spaces we might have eaten eagerly.
            p.jump(p.last_end());
            return expr.map(Node::Expr);
        }

        // Comments.
        Token::LineComment(_) | Token::BlockComment(_) => {
            p.eat();
            return None;
        }

        _ => {
            p.unexpected();
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
        p.assert(Token::Hash);

        let mut level = 0u8;
        while p.eat_if(Token::Hash) {
            level = level.saturating_add(1);
        }
        level
    });

    if level.v > 5 {
        p.diag(warning!(level.span, "should not exceed depth 6"));
        level.v = 5;
    }

    // Parse the heading contents.
    let mut contents = vec![];
    while p.check(|t| !matches!(t, Token::Space(n) if n >= 1)) {
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
fn bracket_call(p: &mut Parser) -> Option<Expr> {
    p.start_group(Group::Bracket, TokenMode::Code);

    // One header is guaranteed, but there may be more (through chaining).
    let mut outer = vec![];
    let mut inner = p.span_if(bracket_subheader);

    while p.eat_if(Token::Pipe) {
        if let Some(new) = p.span_if(bracket_subheader) {
            outer.extend(inner);
            inner = Some(new);
        }
    }

    p.end_group();

    let body = if p.peek() == Some(Token::LeftBracket) {
        Some(p.span(|p| Expr::Template(bracket_body(p))))
    } else {
        None
    };

    let mut inner = inner?;
    if let Some(body) = body {
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

    Some(Expr::Call(inner.v))
}

/// Parse one subheader of a bracketed function call.
fn bracket_subheader(p: &mut Parser) -> Option<ExprCall> {
    p.start_group(Group::Subheader, TokenMode::Code);

    let name = p.span_if(ident);
    let args = p.span(arguments);
    p.end_group();

    Some(ExprCall {
        callee: Box::new(name?.map(Expr::Ident)),
        args,
    })
}

/// Parse the body of a bracketed function call.
fn bracket_body(p: &mut Parser) -> Tree {
    p.start_group(Group::Bracket, TokenMode::Markup);
    let tree = tree(p);
    p.end_group();
    tree
}

/// Parse an expression.
fn expr(p: &mut Parser) -> Option<Expr> {
    expr_with(p, 0)
}

/// Parse an expression with operators having at least the minimum precedence.
fn expr_with(p: &mut Parser, min_prec: usize) -> Option<Expr> {
    let mut lhs = match p.span_if(|p| p.eat_map(UnOp::from_token)) {
        Some(op) => {
            let prec = op.v.precedence();
            let expr = p.span_if(|p| expr_with(p, prec))?;
            let span = op.span.join(expr.span);
            let unary = Expr::Unary(ExprUnary { op, expr: Box::new(expr) });
            unary.with_span(span)
        }
        None => p.span_if(primary)?,
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

        match op.associativity() {
            Associativity::Left => prec += 1,
            Associativity::Right => {}
        }

        let op = op.with_span(p.peek_span());
        p.eat();

        let rhs = match p.span_if(|p| expr_with(p, prec)) {
            Some(rhs) => Box::new(rhs),
            None => break,
        };

        let span = lhs.span.join(rhs.span);
        let binary = Expr::Binary(ExprBinary { lhs: Box::new(lhs), op, rhs });
        lhs = binary.with_span(span);
    }

    Some(lhs.v)
}

/// Parse a primary expression.
fn primary(p: &mut Parser) -> Option<Expr> {
    let expr = match p.peek() {
        // Template.
        Some(Token::LeftBracket) => {
            return Some(template(p));
        }

        // Nested block.
        Some(Token::LeftBrace) => {
            return block(p, true);
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
        Some(Token::Color(color)) => Expr::Color(color),
        Some(Token::Str(token)) => Expr::Str(string(p, token)),

        // Keywords.
        Some(Token::Let) => return expr_let(p),
        Some(Token::If) => return expr_if(p),
        Some(Token::For) => return expr_for(p),

        // No value.
        _ => {
            p.expected("expression");
            return None;
        }
    };
    p.eat();
    Some(expr)
}

// Parse a template value: `[...]`.
fn template(p: &mut Parser) -> Expr {
    p.start_group(Group::Bracket, TokenMode::Markup);
    let tree = tree(p);
    p.end_group();
    Expr::Template(tree)
}

/// Parse a block expression: `{...}`.
fn block(p: &mut Parser, scopes: bool) -> Option<Expr> {
    p.start_group(Group::Brace, TokenMode::Code);
    let mut exprs = vec![];
    while !p.eof() {
        p.start_group(Group::Stmt, TokenMode::Code);
        if let Some(expr) = p.span_if(expr) {
            exprs.push(expr);
            if !p.eof() {
                p.expected_at("semicolon or line break", p.last_end());
            }
        }
        p.end_group();
        p.skip_white();
    }
    p.end_group();
    Some(Expr::Block(ExprBlock { exprs, scopes }))
}

/// Parse a parenthesized function call.
fn paren_call(p: &mut Parser, name: Spanned<Ident>) -> Expr {
    p.start_group(Group::Paren, TokenMode::Code);
    let args = p.span(arguments);
    p.end_group();
    Expr::Call(ExprCall {
        callee: Box::new(name.map(Expr::Ident)),
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
    p.assert(Token::Let);

    let mut expr_let = None;
    if let Some(pat) = p.span_if(ident) {
        let mut init = None;
        if p.eat_if(Token::Eq) {
            init = p.span_if(expr);
        }

        expr_let = Some(Expr::Let(ExprLet { pat, init: init.map(Box::new) }))
    }

    expr_let
}

/// Parse an if expresion.
fn expr_if(p: &mut Parser) -> Option<Expr> {
    p.assert(Token::If);

    let mut expr_if = None;
    if let Some(condition) = p.span_if(expr) {
        if let Some(if_body) = p.span_if(body) {
            let mut else_body = None;
            if p.eat_if(Token::Else) {
                else_body = p.span_if(body);
            }

            expr_if = Some(Expr::If(ExprIf {
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
    p.assert(Token::For);

    let mut expr_for = None;
    if let Some(pat) = p.span_if(for_pattern) {
        if p.expect(Token::In) {
            if let Some(iter) = p.span_if(expr) {
                if let Some(body) = p.span_if(body) {
                    expr_for = Some(Expr::For(ExprFor {
                        pat,
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
    match p.peek() {
        Some(Token::Ident(id)) => {
            p.eat();
            Some(Ident(id.into()))
        }
        _ => {
            p.expected("identifier");
            None
        }
    }
}

/// Parse a control flow body.
fn body(p: &mut Parser) -> Option<Expr> {
    match p.peek() {
        Some(Token::LeftBracket) => Some(template(p)),
        Some(Token::LeftBrace) => block(p, true),
        _ => {
            p.expected_at("body", p.last_end());
            None
        }
    }
}
