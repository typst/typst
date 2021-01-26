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
    let node = match p.peek()? {
        // Bracket call.
        Token::LeftBracket => {
            return Some(Node::Expr(bracket_call(p)?));
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
        Token::Let => return Some(Node::Expr(stmt_let(p)?)),
        Token::If => return Some(Node::Expr(expr_if(p)?)),

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
        p.eat_assert(Token::Hash);

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
    if name.is_none() {
        p.expected("function name");
    }

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

/// Parse an identifier.
fn ident(p: &mut Parser) -> Option<Ident> {
    p.eat_map(|token| match token {
        Token::Ident(id) => Some(Ident(id.into())),
        _ => None,
    })
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
        Some(Token::Color(color)) => Expr::Color(color),
        Some(Token::Str(token)) => Expr::Str(string(p, token)),

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
fn block(p: &mut Parser) -> Option<Expr> {
    p.start_group(Group::Brace, TokenMode::Code);
    let expr = p.span_if(expr);
    while !p.eof() {
        p.unexpected();
    }
    p.end_group();
    Some(Expr::Block(Box::new(expr?)))
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

/// Parse a let statement.
fn stmt_let(p: &mut Parser) -> Option<Expr> {
    p.start_group(Group::Stmt, TokenMode::Code);
    p.eat_assert(Token::Let);

    let pat = match p.span_if(ident) {
        Some(pat) => pat,
        None => {
            p.expected("identifier");
            p.end_group();
            return None;
        }
    };

    let rhs = if p.eat_if(Token::Eq) { p.span_if(expr) } else { None };

    if !p.eof() {
        p.expected_at("semicolon or line break", p.last_end());
    }

    p.end_group();

    Some(Expr::Let(ExprLet { pat, expr: rhs.map(Box::new) }))
}

/// Parse an if expresion.
fn expr_if(p: &mut Parser) -> Option<Expr> {
    p.start_group(Group::Expr, TokenMode::Code);
    p.eat_assert(Token::If);

    let condition = match p.span_if(expr) {
        Some(condition) => Box::new(condition),
        None => {
            p.end_group();
            return None;
        }
    };

    p.end_group();

    let if_body = Box::new(control_body(p)?);

    let start = p.last_end();
    p.skip_white();

    let else_body = if p.eat_if(Token::Else) {
        control_body(p).map(Box::new)
    } else {
        p.jump(start);
        None
    };

    Some(Expr::If(ExprIf { condition, if_body, else_body }))
}

/// Parse a control flow body.
fn control_body(p: &mut Parser) -> Option<Spanned<Expr>> {
    let start = p.last_end();
    p.skip_white();

    match p.peek() {
        Some(Token::LeftBracket) => Some(p.span(template)),
        Some(Token::LeftBrace) => p.span_if(block),
        _ => {
            p.expected_at("body", start);
            p.jump(start);
            None
        }
    }
}
