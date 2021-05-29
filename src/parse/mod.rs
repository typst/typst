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

use std::rc::Rc;

use crate::diag::Pass;
use crate::syntax::*;

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
            if !matches!(node, Node::Parbreak(_) | Node::Space) {
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
    let span = p.peek_span();
    let node = match token {
        // Whitespace.
        Token::Space(newlines) => {
            *at_start |= newlines > 0;
            if newlines < 2 {
                Node::Space
            } else {
                Node::Parbreak(span)
            }
        }

        // Text.
        Token::Text(text) => Node::Text(text.into()),

        // Markup.
        Token::Star => Node::Strong(span),
        Token::Underscore => Node::Emph(span),
        Token::Eq => {
            if *at_start {
                return Some(heading(p));
            } else {
                Node::Text(p.peek_src().into())
            }
        }
        Token::Tilde => Node::Text("\u{00A0}".into()),
        Token::Backslash => Node::Linebreak(span),
        Token::Raw(t) => raw(p, t),
        Token::UnicodeEscape(t) => Node::Text(unicode_escape(p, t)),

        // Hashtag + keyword / identifier.
        Token::Ident(_)
        | Token::Let
        | Token::If
        | Token::While
        | Token::For
        | Token::Import
        | Token::Include => {
            *at_start = false;
            let stmt = token == Token::Let || token == Token::Import;
            let group = if stmt { Group::Stmt } else { Group::Expr };

            p.start_group(group, TokenMode::Code);
            let expr = expr_with(p, true, 0);
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
            return Some(Node::Expr(block(p, false)));
        }

        // Template.
        Token::LeftBracket => {
            *at_start = false;
            return Some(Node::Expr(template(p)));
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
    p.assert(Token::Eq);

    // Count depth.
    let mut level: usize = 1;
    while p.eat_if(Token::Eq) {
        level += 1;
    }

    if level > 6 {
        p.diag(warning!(start .. p.end(), "should not exceed depth 6"));
        level = 6;
    }

    // Parse the heading contents.
    let mut tree = vec![];
    while p.check(|t| !matches!(t, Token::Space(n) if n >= 1)) {
        tree.extend(node(p, &mut false));
    }

    Node::Heading(HeadingNode {
        span: p.span(start),
        level,
        contents: Rc::new(tree),
    })
}

/// Handle a raw block.
fn raw(p: &mut Parser, token: RawToken) -> Node {
    let span = p.peek_span();
    let raw = resolve::resolve_raw(span, token.text, token.backticks);
    if !token.terminated {
        p.diag(error!(p.peek_span().end, "expected backtick(s)"));
    }
    Node::Raw(raw)
}

/// Handle a unicode escape sequence.
fn unicode_escape(p: &mut Parser, token: UnicodeEscapeToken) -> String {
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

/// Parse an expression.
fn expr(p: &mut Parser) -> Option<Expr> {
    expr_with(p, false, 0)
}

/// Parse an expression with operators having at least the minimum precedence.
///
/// If `atomic` is true, this does not parse binary operations and arrow
/// functions, which is exactly what we want in a shorthand expression directly
/// in markup.
///
/// Stops parsing at operations with lower precedence than `min_prec`,
fn expr_with(p: &mut Parser, atomic: bool, min_prec: usize) -> Option<Expr> {
    let start = p.start();
    let mut lhs = match p.eat_map(UnOp::from_token) {
        Some(op) => {
            let prec = op.precedence();
            let expr = Box::new(expr_with(p, atomic, prec)?);
            Expr::Unary(UnaryExpr { span: p.span(start), op, expr })
        }
        None => primary(p, atomic)?,
    };

    loop {
        // Parenthesis or bracket means this is a function call.
        if matches!(
            p.peek_direct(),
            Some(Token::LeftParen) | Some(Token::LeftBracket),
        ) {
            lhs = call(p, lhs);
            continue;
        }

        if atomic {
            break;
        }

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

        let rhs = match expr_with(p, atomic, prec) {
            Some(rhs) => Box::new(rhs),
            None => break,
        };

        let span = lhs.span().join(rhs.span());
        lhs = Expr::Binary(BinaryExpr { span, lhs: Box::new(lhs), op, rhs });
    }

    Some(lhs)
}

/// Parse a primary expression.
fn primary(p: &mut Parser, atomic: bool) -> Option<Expr> {
    if let Some(expr) = literal(p) {
        return Some(expr);
    }

    match p.peek() {
        // Things that start with an identifier.
        Some(Token::Ident(string)) => {
            let id = Ident {
                span: p.eat_span(),
                string: string.into(),
            };

            // Arrow means this is a closure's lone parameter.
            Some(if !atomic && p.eat_if(Token::Arrow) {
                let body = expr(p)?;
                Expr::Closure(ClosureExpr {
                    span: id.span.join(body.span()),
                    name: None,
                    params: Rc::new(vec![id]),
                    body: Rc::new(body),
                })
            } else {
                Expr::Ident(id)
            })
        }

        // Structures.
        Some(Token::LeftParen) => parenthesized(p),
        Some(Token::LeftBracket) => Some(template(p)),
        Some(Token::LeftBrace) => Some(block(p, true)),

        // Keywords.
        Some(Token::Let) => expr_let(p),
        Some(Token::If) => expr_if(p),
        Some(Token::While) => expr_while(p),
        Some(Token::For) => expr_for(p),
        Some(Token::Import) => expr_import(p),
        Some(Token::Include) => expr_include(p),

        // Nothing.
        _ => {
            p.expected("expression");
            None
        }
    }
}

/// Parse a literal.
fn literal(p: &mut Parser) -> Option<Expr> {
    let span = p.peek_span();
    let expr = match p.peek()? {
        // Basic values.
        Token::None => Expr::None(span),
        Token::Bool(b) => Expr::Bool(span, b),
        Token::Int(i) => Expr::Int(span, i),
        Token::Float(f) => Expr::Float(span, f),
        Token::Length(val, unit) => Expr::Length(span, val, unit),
        Token::Angle(val, unit) => Expr::Angle(span, val, unit),
        Token::Percent(p) => Expr::Percent(span, p),
        Token::Color(color) => Expr::Color(span, color),
        Token::Str(token) => Expr::Str(span, {
            if !token.terminated {
                p.expected_at("quote", p.peek_span().end);
            }
            resolve::resolve_string(token.string)
        }),
        _ => return None,
    };
    p.eat();
    Some(expr)
}

/// Parse something that starts with a parenthesis, which can be either of:
/// - Array literal
/// - Dictionary literal
/// - Parenthesized expression
/// - Parameter list of closure expression
fn parenthesized(p: &mut Parser) -> Option<Expr> {
    p.start_group(Group::Paren, TokenMode::Code);
    let colon = p.eat_if(Token::Colon);
    let (items, has_comma) = collection(p);
    let span = p.end_group();

    // Leading colon makes this a dictionary.
    if colon {
        return Some(dict(p, items, span));
    }

    // Arrow means this is a closure's parameter list.
    if p.eat_if(Token::Arrow) {
        let params = idents(p, items);
        let body = expr(p)?;
        return Some(Expr::Closure(ClosureExpr {
            span: span.join(body.span()),
            name: None,
            params: Rc::new(params),
            body: Rc::new(body),
        }));
    }

    // Find out which kind of collection this is.
    Some(match items.as_slice() {
        [] => array(p, items, span),
        [CallArg::Pos(_)] if !has_comma => match items.into_iter().next() {
            Some(CallArg::Pos(expr)) => {
                Expr::Group(GroupExpr { span, expr: Box::new(expr) })
            }
            _ => unreachable!(),
        },
        [CallArg::Pos(_), ..] => array(p, items, span),
        [CallArg::Named(_), ..] => dict(p, items, span),
    })
}

/// Parse a collection.
///
/// Returns whether the literal contained any commas.
fn collection(p: &mut Parser) -> (Vec<CallArg>, bool) {
    let mut items = vec![];
    let mut has_comma = false;
    let mut missing_coma = None;

    while !p.eof() {
        if let Some(arg) = item(p) {
            items.push(arg);

            if let Some(pos) = missing_coma.take() {
                p.expected_at("comma", pos);
            }

            if p.eof() {
                break;
            }

            let behind = p.end();
            if p.eat_if(Token::Comma) {
                has_comma = true;
            } else {
                missing_coma = Some(behind);
            }
        }
    }

    (items, has_comma)
}

/// Parse an expression or a named pair.
fn item(p: &mut Parser) -> Option<CallArg> {
    let first = expr(p)?;
    if p.eat_if(Token::Colon) {
        if let Expr::Ident(name) = first {
            Some(CallArg::Named(Named { name, expr: expr(p)? }))
        } else {
            p.diag(error!(first.span(), "expected identifier"));
            expr(p);
            None
        }
    } else {
        Some(CallArg::Pos(first))
    }
}

/// Convert a collection into an array, producing errors for named items.
fn array(p: &mut Parser, items: Vec<CallArg>, span: Span) -> Expr {
    let items = items.into_iter().filter_map(|item| match item {
        CallArg::Pos(expr) => Some(expr),
        CallArg::Named(_) => {
            p.diag(error!(item.span(), "expected expression, found named pair"));
            None
        }
    });

    Expr::Array(ArrayExpr { span, items: items.collect() })
}

/// Convert a collection into a dictionary, producing errors for expressions.
fn dict(p: &mut Parser, items: Vec<CallArg>, span: Span) -> Expr {
    let items = items.into_iter().filter_map(|item| match item {
        CallArg::Named(named) => Some(named),
        CallArg::Pos(_) => {
            p.diag(error!(item.span(), "expected named pair, found expression"));
            None
        }
    });

    Expr::Dict(DictExpr { span, items: items.collect() })
}

/// Convert a collection into a list of identifiers, producing errors for
/// anything other than identifiers.
fn idents(p: &mut Parser, items: Vec<CallArg>) -> Vec<Ident> {
    let items = items.into_iter().filter_map(|item| match item {
        CallArg::Pos(Expr::Ident(id)) => Some(id),
        _ => {
            p.diag(error!(item.span(), "expected identifier"));
            None
        }
    });
    items.collect()
}

// Parse a template value: `[...]`.
fn template(p: &mut Parser) -> Expr {
    p.start_group(Group::Bracket, TokenMode::Markup);
    let tree = Rc::new(tree(p));
    let span = p.end_group();
    Expr::Template(TemplateExpr { span, tree })
}

/// Parse a block expression: `{...}`.
fn block(p: &mut Parser, scoping: bool) -> Expr {
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
    Expr::Block(BlockExpr { span, exprs, scoping })
}

/// Parse a function call.
fn call(p: &mut Parser, callee: Expr) -> Expr {
    let mut args = match p.peek_direct() {
        Some(Token::LeftParen) => {
            p.start_group(Group::Paren, TokenMode::Code);
            let args = args(p);
            p.end_group();
            args
        }
        _ => CallArgs {
            span: Span::at(callee.span().end),
            items: vec![],
        },
    };

    if p.peek_direct() == Some(Token::LeftBracket) {
        let body = template(p);
        args.items.push(CallArg::Pos(body));
    }

    Expr::Call(CallExpr {
        span: p.span(callee.span().start),
        callee: Box::new(callee),
        args,
    })
}

/// Parse the arguments to a function call.
fn args(p: &mut Parser) -> CallArgs {
    let start = p.start();
    let items = collection(p).0;
    CallArgs { span: p.span(start), items }
}

/// Parse a let expression.
fn expr_let(p: &mut Parser) -> Option<Expr> {
    let start = p.start();
    p.assert(Token::Let);

    let mut expr_let = None;
    if let Some(binding) = ident(p) {
        // If a parenthesis follows, this is a function definition.
        let mut params = None;
        if p.peek_direct() == Some(Token::LeftParen) {
            p.start_group(Group::Paren, TokenMode::Code);
            let items = collection(p).0;
            params = Some(idents(p, items));
            p.end_group();
        }

        let mut init = None;
        if p.eat_if(Token::Eq) {
            init = expr(p);
        } else if params.is_some() {
            // Function definitions must have a body.
            p.expected_at("body", p.end());
        }

        // Rewrite into a closure expression if it's a function definition.
        if let Some(params) = params {
            let body = init?;
            init = Some(Expr::Closure(ClosureExpr {
                span: binding.span.join(body.span()),
                name: Some(binding.clone()),
                params: Rc::new(params),
                body: Rc::new(body),
            }));
        }

        expr_let = Some(Expr::Let(LetExpr {
            span: p.span(start),
            binding,
            init: init.map(Box::new),
        }));
    }

    expr_let
}

/// Parse an import expression.
fn expr_import(p: &mut Parser) -> Option<Expr> {
    let start = p.start();
    p.assert(Token::Import);

    let mut expr_import = None;
    if let Some(path) = expr(p) {
        if p.expect(Token::Using) {
            let imports = if p.eat_if(Token::Star) {
                // This is the wildcard scenario.
                Imports::Wildcard
            } else {
                // This is the list of identifier scenario.
                p.start_group(Group::Expr, TokenMode::Code);
                let items = collection(p).0;
                if items.is_empty() {
                    p.expected_at("import items", p.end());
                }

                let idents = idents(p, items);
                p.end_group();
                Imports::Idents(idents)
            };

            expr_import = Some(Expr::Import(ImportExpr {
                span: p.span(start),
                imports,
                path: Box::new(path),
            }));
        }
    }

    expr_import
}

/// Parse an include expression.
fn expr_include(p: &mut Parser) -> Option<Expr> {
    let start = p.start();
    p.assert(Token::Include);

    expr(p).map(|path| {
        Expr::Include(IncludeExpr {
            span: p.span(start),
            path: Box::new(path),
        })
    })
}

/// Parse an if expresion.
fn expr_if(p: &mut Parser) -> Option<Expr> {
    let start = p.start();
    p.assert(Token::If);

    let mut expr_if = None;
    if let Some(condition) = expr(p) {
        if let Some(if_body) = body(p) {
            let mut else_body = None;

            // We are in code mode but still want to react to `#else` if the
            // outer mode is markup.
            if match p.outer_mode() {
                TokenMode::Markup => p.eat_if(Token::Invalid("#else")),
                TokenMode::Code => p.eat_if(Token::Else),
            } {
                else_body = body(p);
            }

            expr_if = Some(Expr::If(IfExpr {
                span: p.span(start),
                condition: Box::new(condition),
                if_body: Box::new(if_body),
                else_body: else_body.map(Box::new),
            }));
        }
    }

    expr_if
}

/// Parse a while expresion.
fn expr_while(p: &mut Parser) -> Option<Expr> {
    let start = p.start();
    p.assert(Token::While);

    let mut expr_while = None;
    if let Some(condition) = expr(p) {
        if let Some(body) = body(p) {
            expr_while = Some(Expr::While(WhileExpr {
                span: p.span(start),
                condition: Box::new(condition),
                body: Box::new(body),
            }));
        }
    }

    expr_while
}

/// Parse a for expression.
fn expr_for(p: &mut Parser) -> Option<Expr> {
    let start = p.start();
    p.assert(Token::For);

    let mut expr_for = None;
    if let Some(pattern) = for_pattern(p) {
        if p.expect(Token::In) {
            if let Some(iter) = expr(p) {
                if let Some(body) = body(p) {
                    expr_for = Some(Expr::For(ForExpr {
                        span: p.span(start),
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
        Some(Token::LeftBrace) => Some(block(p, true)),
        _ => {
            p.expected_at("body", p.end());
            None
        }
    }
}
