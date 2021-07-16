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
use crate::eco::EcoString;
use crate::syntax::*;

/// Parse a string of source code.
pub fn parse(src: &str) -> Pass<SyntaxTree> {
    let mut p = Parser::new(src);
    Pass::new(tree(&mut p), p.diags)
}

/// Parse a syntax tree.
fn tree(p: &mut Parser) -> SyntaxTree {
    tree_while(p, true, &mut |_| true)
}

/// Parse a syntax tree that stays right of the column at the start of the next
/// non-whitespace token.
fn tree_indented(p: &mut Parser) -> SyntaxTree {
    p.eat_while(|t| match t {
        Token::Space(n) => n == 0,
        Token::LineComment(_) | Token::BlockComment(_) => true,
        _ => false,
    });

    let column = p.column(p.next_start());
    tree_while(p, false, &mut |p| match p.peek() {
        Some(Token::Space(n)) if n >= 1 => p.column(p.next_end()) >= column,
        _ => true,
    })
}

/// Parse a syntax tree.
fn tree_while<F>(p: &mut Parser, mut at_start: bool, f: &mut F) -> SyntaxTree
where
    F: FnMut(&mut Parser) -> bool,
{
    // We use `at_start` to keep track of whether we are at the start of a line
    // or template to know whether things like headings are allowed.
    let mut tree = vec![];
    while !p.eof() && f(p) {
        if let Some(mut node) = node(p, &mut at_start) {
            at_start &= matches!(node, SyntaxNode::Space | SyntaxNode::Parbreak(_));

            // Look for wide call.
            if let SyntaxNode::Expr(Expr::Call(call)) = &mut node {
                if call.wide {
                    let start = p.next_start();
                    let tree = tree_while(p, true, f);
                    call.args.items.push(CallArg::Pos(Expr::Template(TemplateExpr {
                        span: p.span(start),
                        tree: Rc::new(tree),
                    })));
                }
            }

            tree.push(node);
        }
    }

    tree
}

/// Parse a syntax node.
fn node(p: &mut Parser, at_start: &mut bool) -> Option<SyntaxNode> {
    let token = p.peek()?;
    let span = p.peek_span();
    let node = match token {
        // Whitespace.
        Token::Space(newlines) => {
            *at_start |= newlines > 0;
            if newlines < 2 {
                SyntaxNode::Space
            } else {
                SyntaxNode::Parbreak(span)
            }
        }

        // Text.
        Token::Text(text) => SyntaxNode::Text(text.into()),
        Token::Tilde => SyntaxNode::Text("\u{00A0}".into()),
        Token::HyphHyph => SyntaxNode::Text("\u{2013}".into()),
        Token::HyphHyphHyph => SyntaxNode::Text("\u{2014}".into()),
        Token::UnicodeEscape(t) => SyntaxNode::Text(unicode_escape(p, t)),

        // Markup.
        Token::Backslash => SyntaxNode::Linebreak(span),
        Token::Star => SyntaxNode::Strong(span),
        Token::Underscore => SyntaxNode::Emph(span),
        Token::Raw(t) => raw(p, t),
        Token::Eq if *at_start => return Some(heading(p)),
        Token::Hyph if *at_start => return Some(list_item(p)),
        Token::Numbering(number) if *at_start => return Some(enum_item(p, number)),

        // Line-based markup that is not currently at the start of the line.
        Token::Eq | Token::Hyph | Token::Numbering(_) => {
            SyntaxNode::Text(p.peek_src().into())
        }

        // Hashtag + keyword / identifier.
        Token::Ident(_)
        | Token::Let
        | Token::If
        | Token::While
        | Token::For
        | Token::Import
        | Token::Include => {
            let stmt = matches!(token, Token::Let | Token::Import);
            let group = if stmt { Group::Stmt } else { Group::Expr };

            p.start_group(group, TokenMode::Code);
            let expr = expr_with(p, true, 0);
            if stmt && expr.is_some() && !p.eof() {
                p.expected_at("semicolon or line break", p.prev_end());
            }
            p.end_group();

            return expr.map(SyntaxNode::Expr);
        }

        // Block and template.
        Token::LeftBrace => return Some(SyntaxNode::Expr(block(p, false))),
        Token::LeftBracket => return Some(SyntaxNode::Expr(template(p))),

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

/// Handle a unicode escape sequence.
fn unicode_escape(p: &mut Parser, token: UnicodeEscapeToken) -> EcoString {
    let span = p.peek_span();
    let text = if let Some(c) = resolve::resolve_hex(token.sequence) {
        c.into()
    } else {
        // Print out the escape sequence verbatim if it is invalid.
        p.diag(error!(span, "invalid unicode escape sequence"));
        p.peek_src().into()
    };

    if !token.terminated {
        p.diag(error!(span.end, "expected closing brace"));
    }

    text
}

/// Handle a raw block.
fn raw(p: &mut Parser, token: RawToken) -> SyntaxNode {
    let span = p.peek_span();
    let raw = resolve::resolve_raw(span, token.text, token.backticks);
    if !token.terminated {
        p.diag(error!(p.peek_span().end, "expected backtick(s)"));
    }
    SyntaxNode::Raw(raw)
}

/// Parse a heading.
fn heading(p: &mut Parser) -> SyntaxNode {
    let start = p.next_start();
    p.assert(Token::Eq);

    // Count depth.
    let mut level: usize = 1;
    while p.eat_if(Token::Eq) {
        level += 1;
    }

    if level > 6 {
        return SyntaxNode::Text(p.eaten_from(start).into());
    }

    let body = tree_indented(p);

    SyntaxNode::Heading(HeadingNode {
        span: p.span(start),
        level,
        body: Rc::new(body),
    })
}

/// Parse a single list item.
fn list_item(p: &mut Parser) -> SyntaxNode {
    let start = p.next_start();
    p.assert(Token::Hyph);
    let body = tree_indented(p);
    SyntaxNode::List(ListItem { span: p.span(start), body })
}

/// Parse a single enum item.
fn enum_item(p: &mut Parser, number: Option<usize>) -> SyntaxNode {
    let start = p.next_start();
    p.assert(Token::Numbering(number));
    let body = tree_indented(p);
    SyntaxNode::Enum(EnumItem { span: p.span(start), number, body })
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
    let start = p.next_start();
    let mut lhs = match p.eat_map(UnOp::from_token) {
        Some(op) => {
            let prec = op.precedence();
            let expr = Box::new(expr_with(p, atomic, prec)?);
            Expr::Unary(UnaryExpr { span: p.span(start), op, expr })
        }
        None => primary(p, atomic)?,
    };

    loop {
        // Exclamation mark, parenthesis or bracket means this is a function
        // call.
        if matches!(
            p.peek_direct(),
            Some(Token::Excl | Token::LeftParen | Token::LeftBracket),
        ) {
            lhs = call(p, lhs)?;
            continue;
        }

        if p.eat_if(Token::With) {
            lhs = with_expr(p, lhs)?;
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
        Some(Token::Let) => let_expr(p),
        Some(Token::If) => if_expr(p),
        Some(Token::While) => while_expr(p),
        Some(Token::For) => for_expr(p),
        Some(Token::Import) => import_expr(p),
        Some(Token::Include) => include_expr(p),

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
        Token::Auto => Expr::Auto(span),
        Token::Bool(b) => Expr::Bool(span, b),
        Token::Int(i) => Expr::Int(span, i),
        Token::Float(f) => Expr::Float(span, f),
        Token::Length(val, unit) => Expr::Length(span, val, unit),
        Token::Angle(val, unit) => Expr::Angle(span, val, unit),
        Token::Percent(p) => Expr::Percent(span, p),
        Token::Fraction(p) => Expr::Fractional(span, p),
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

            let behind = p.prev_end();
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
                p.expected_at("semicolon or line break", p.prev_end());
            }
        }
        p.end_group();

        // Forcefully skip over newlines since the group's contents can't.
        p.eat_while(|t| matches!(t, Token::Space(_)));
    }
    let span = p.end_group();
    Expr::Block(BlockExpr { span, exprs, scoping })
}

/// Parse a function call.
fn call(p: &mut Parser, callee: Expr) -> Option<Expr> {
    let mut wide = p.eat_if(Token::Excl);
    if wide && p.outer_mode() == TokenMode::Code {
        let span = p.span(callee.span().start);
        p.diag(error!(
            span,
            "wide calls are only allowed directly in templates",
        ));
        wide = false;
    }

    let mut args = match p.peek_direct() {
        Some(Token::LeftParen) => {
            p.start_group(Group::Paren, TokenMode::Code);
            let args = args(p);
            p.end_group();
            args
        }
        Some(Token::LeftBracket) => CallArgs {
            span: Span::at(callee.span().end),
            items: vec![],
        },
        _ => {
            p.expected_at("argument list", p.prev_end());
            return None;
        }
    };

    if p.peek_direct() == Some(Token::LeftBracket) {
        let body = template(p);
        args.items.push(CallArg::Pos(body));
    }

    Some(Expr::Call(CallExpr {
        span: p.span(callee.span().start),
        callee: Box::new(callee),
        wide,
        args,
    }))
}

/// Parse the arguments to a function call.
fn args(p: &mut Parser) -> CallArgs {
    let start = p.next_start();
    let items = collection(p).0;
    CallArgs { span: p.span(start), items }
}

/// Parse a with expression.
fn with_expr(p: &mut Parser, callee: Expr) -> Option<Expr> {
    if p.peek() == Some(Token::LeftParen) {
        p.start_group(Group::Paren, TokenMode::Code);
        let args = args(p);
        p.end_group();

        Some(Expr::With(WithExpr {
            span: p.span(callee.span().start),
            callee: Box::new(callee),
            args,
        }))
    } else {
        p.expected("argument list");
        None
    }
}

/// Parse a let expression.
fn let_expr(p: &mut Parser) -> Option<Expr> {
    let start = p.next_start();
    p.assert(Token::Let);

    let mut let_expr = None;
    if let Some(binding) = ident(p) {
        let mut init = None;

        if p.eat_if(Token::With) {
            init = with_expr(p, Expr::Ident(binding.clone()));
        } else {
            // If a parenthesis follows, this is a function definition.
            let mut params = None;
            if p.peek_direct() == Some(Token::LeftParen) {
                p.start_group(Group::Paren, TokenMode::Code);
                let items = collection(p).0;
                params = Some(idents(p, items));
                p.end_group();
            }

            if p.eat_if(Token::Eq) {
                init = expr(p);
            } else if params.is_some() {
                // Function definitions must have a body.
                p.expected_at("body", p.prev_end());
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
        }

        let_expr = Some(Expr::Let(LetExpr {
            span: p.span(start),
            binding,
            init: init.map(Box::new),
        }));
    }

    let_expr
}

/// Parse an if expresion.
fn if_expr(p: &mut Parser) -> Option<Expr> {
    let start = p.next_start();
    p.assert(Token::If);

    let mut if_expr = None;
    if let Some(condition) = expr(p) {
        if let Some(if_body) = body(p) {
            let mut else_body = None;

            // We are in code mode but still want to react to `#else` if the
            // outer mode is markup.
            if (p.outer_mode() == TokenMode::Code || p.eat_if(Token::Invalid("#")))
                && p.eat_if(Token::Else)
            {
                else_body = body(p);
            }

            if_expr = Some(Expr::If(IfExpr {
                span: p.span(start),
                condition: Box::new(condition),
                if_body: Box::new(if_body),
                else_body: else_body.map(Box::new),
            }));
        }
    }

    if_expr
}

/// Parse a while expresion.
fn while_expr(p: &mut Parser) -> Option<Expr> {
    let start = p.next_start();
    p.assert(Token::While);

    let mut while_expr = None;
    if let Some(condition) = expr(p) {
        if let Some(body) = body(p) {
            while_expr = Some(Expr::While(WhileExpr {
                span: p.span(start),
                condition: Box::new(condition),
                body: Box::new(body),
            }));
        }
    }

    while_expr
}

/// Parse a for expression.
fn for_expr(p: &mut Parser) -> Option<Expr> {
    let start = p.next_start();
    p.assert(Token::For);

    let mut for_expr = None;
    if let Some(pattern) = for_pattern(p) {
        if p.expect(Token::In) {
            if let Some(iter) = expr(p) {
                if let Some(body) = body(p) {
                    for_expr = Some(Expr::For(ForExpr {
                        span: p.span(start),
                        pattern,
                        iter: Box::new(iter),
                        body: Box::new(body),
                    }));
                }
            }
        }
    }

    for_expr
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

/// Parse an import expression.
fn import_expr(p: &mut Parser) -> Option<Expr> {
    let start = p.next_start();
    p.assert(Token::Import);

    let imports = if p.eat_if(Token::Star) {
        // This is the wildcard scenario.
        Imports::Wildcard
    } else {
        // This is the list of identifiers scenario.
        p.start_group(Group::Imports, TokenMode::Code);
        let items = collection(p).0;
        if items.is_empty() {
            p.expected_at("import items", p.prev_end());
        }
        p.end_group();
        Imports::Idents(idents(p, items))
    };

    let mut import_expr = None;
    if p.expect(Token::From) {
        if let Some(path) = expr(p) {
            import_expr = Some(Expr::Import(ImportExpr {
                span: p.span(start),
                imports,
                path: Box::new(path),
            }));
        }
    }

    import_expr
}

/// Parse an include expression.
fn include_expr(p: &mut Parser) -> Option<Expr> {
    let start = p.next_start();
    p.assert(Token::Include);

    expr(p).map(|path| {
        Expr::Include(IncludeExpr {
            span: p.span(start),
            path: Box::new(path),
        })
    })
}

/// Parse an identifier.
fn ident(p: &mut Parser) -> Option<Ident> {
    if let Some(Token::Ident(string)) = p.peek() {
        Some(Ident {
            span: p.eat_span(),
            string: string.into(),
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
            p.expected_at("body", p.prev_end());
            None
        }
    }
}
