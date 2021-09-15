//! Parsing and tokenization.

mod parser;
mod resolve;
mod scanner;
mod tokens;

pub use parser::*;
pub use resolve::*;
pub use scanner::*;
pub use tokens::*;

use std::rc::Rc;

use crate::diag::TypResult;
use crate::source::SourceFile;
use crate::syntax::*;
use crate::util::EcoString;

/// Parse a source file.
pub fn parse(source: &SourceFile) -> TypResult<Markup> {
    let mut p = Parser::new(source);
    let markup = markup(&mut p);
    let errors = p.finish();
    if errors.is_empty() {
        Ok(markup)
    } else {
        Err(Box::new(errors))
    }
}

/// Parse markup.
fn markup(p: &mut Parser) -> Markup {
    markup_while(p, true, &mut |_| true)
}

/// Parse markup that stays right of the given column.
fn markup_indented(p: &mut Parser, column: usize) -> Markup {
    p.eat_while(|t| match t {
        Token::Space(n) => n == 0,
        Token::LineComment(_) | Token::BlockComment(_) => true,
        _ => false,
    });

    markup_while(p, false, &mut |p| match p.peek() {
        Some(Token::Space(n)) if n >= 1 => p.column(p.next_end()) > column,
        _ => true,
    })
}

/// Parse a syntax tree while the peeked token satisifies a condition.
///
/// If `at_start` is true, things like headings that may only appear at the
/// beginning of a line or template are allowed.
fn markup_while<F>(p: &mut Parser, mut at_start: bool, f: &mut F) -> Markup
where
    F: FnMut(&mut Parser) -> bool,
{
    let mut tree = vec![];
    while !p.eof() && f(p) {
        if let Some(node) = markup_node(p, &mut at_start) {
            at_start &= matches!(node, MarkupNode::Space | MarkupNode::Parbreak(_));
            tree.push(node);
        }
    }

    tree
}

/// Parse a markup node.
fn markup_node(p: &mut Parser, at_start: &mut bool) -> Option<MarkupNode> {
    let token = p.peek()?;
    let span = p.peek_span();
    let node = match token {
        // Whitespace.
        Token::Space(newlines) => {
            *at_start |= newlines > 0;
            if newlines < 2 {
                MarkupNode::Space
            } else {
                MarkupNode::Parbreak(span)
            }
        }

        // Text.
        Token::Text(text) => MarkupNode::Text(text.into()),
        Token::Tilde => MarkupNode::Text("\u{00A0}".into()),
        Token::HyphHyph => MarkupNode::Text("\u{2013}".into()),
        Token::HyphHyphHyph => MarkupNode::Text("\u{2014}".into()),
        Token::UnicodeEscape(t) => MarkupNode::Text(unicode_escape(p, t)),

        // Markup.
        Token::Backslash => MarkupNode::Linebreak(span),
        Token::Star => MarkupNode::Strong(span),
        Token::Underscore => MarkupNode::Emph(span),
        Token::Raw(t) => raw(p, t),
        Token::Eq if *at_start => return Some(heading(p)),
        Token::Hyph if *at_start => return Some(list_node(p)),
        Token::Numbering(number) if *at_start => return Some(enum_node(p, number)),

        // Line-based markup that is not currently at the start of the line.
        Token::Eq | Token::Hyph | Token::Numbering(_) => {
            MarkupNode::Text(p.peek_src().into())
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
                p.expected_at(p.prev_end(), "semicolon or line break");
            }
            p.end_group();

            return expr.map(MarkupNode::Expr);
        }

        // Block and template.
        Token::LeftBrace => return Some(MarkupNode::Expr(block(p, false))),
        Token::LeftBracket => return Some(MarkupNode::Expr(template(p))),

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
        p.error(span, "invalid unicode escape sequence");
        p.peek_src().into()
    };

    if !token.terminated {
        p.error(span.end, "expected closing brace");
    }

    text
}

/// Handle a raw block.
fn raw(p: &mut Parser, token: RawToken) -> MarkupNode {
    let span = p.peek_span();
    let raw = resolve::resolve_raw(span, token.text, token.backticks);
    if !token.terminated {
        p.error(span.end, "expected backtick(s)");
    }
    MarkupNode::Raw(Box::new(raw))
}

/// Parse a heading.
fn heading(p: &mut Parser) -> MarkupNode {
    let start = p.next_start();
    let column = p.column(start);
    p.eat_assert(Token::Eq);

    // Count depth.
    let mut level: usize = 1;
    while p.eat_if(Token::Eq) {
        level += 1;
    }

    if level > 6 {
        return MarkupNode::Text(p.get(start .. p.prev_end()).into());
    }

    let body = markup_indented(p, column);
    MarkupNode::Heading(Box::new(HeadingNode {
        span: p.span_from(start),
        level,
        body,
    }))
}

/// Parse a single list item.
fn list_node(p: &mut Parser) -> MarkupNode {
    let start = p.next_start();
    let column = p.column(start);
    p.eat_assert(Token::Hyph);
    let body = markup_indented(p, column);
    MarkupNode::List(Box::new(ListNode { span: p.span_from(start), body }))
}

/// Parse a single enum item.
fn enum_node(p: &mut Parser, number: Option<usize>) -> MarkupNode {
    let start = p.next_start();
    let column = p.column(start);
    p.eat_assert(Token::Numbering(number));
    let body = markup_indented(p, column);
    MarkupNode::Enum(Box::new(EnumNode {
        span: p.span_from(start),
        number,
        body,
    }))
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
            let expr = expr_with(p, atomic, prec)?;
            Expr::Unary(Box::new(UnaryExpr { span: p.span_from(start), op, expr }))
        }
        None => primary(p, atomic)?,
    };

    loop {
        // Exclamation mark, parenthesis or bracket means this is a function
        // call.
        if matches!(p.peek_direct(), Some(Token::LeftParen | Token::LeftBracket)) {
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
            Some(rhs) => rhs,
            None => break,
        };

        let span = lhs.span().join(rhs.span());
        lhs = Expr::Binary(Box::new(BinaryExpr { span, lhs, op, rhs }));
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
            let ident = Ident {
                span: p.eat_span(),
                string: string.into(),
            };

            // Arrow means this is a closure's lone parameter.
            Some(if !atomic && p.eat_if(Token::Arrow) {
                let body = expr(p)?;
                Expr::Closure(Box::new(ClosureExpr {
                    span: ident.span.join(body.span()),
                    name: None,
                    params: vec![ClosureParam::Pos(ident)],
                    body: Rc::new(body),
                }))
            } else {
                Expr::Ident(Box::new(ident))
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
    let lit = match p.peek()? {
        // Basic values.
        Token::None => Lit::None(span),
        Token::Auto => Lit::Auto(span),
        Token::Bool(b) => Lit::Bool(span, b),
        Token::Int(i) => Lit::Int(span, i),
        Token::Float(f) => Lit::Float(span, f),
        Token::Length(val, unit) => Lit::Length(span, val, unit),
        Token::Angle(val, unit) => Lit::Angle(span, val, unit),
        Token::Percent(p) => Lit::Percent(span, p),
        Token::Fraction(p) => Lit::Fractional(span, p),
        Token::Str(token) => Lit::Str(span, {
            if !token.terminated {
                p.expected_at(span.end, "quote");
            }
            resolve::resolve_string(token.string)
        }),
        _ => return None,
    };
    p.eat();
    Some(Expr::Lit(Box::new(lit)))
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
        let params = params(p, items);
        let body = expr(p)?;
        return Some(Expr::Closure(Box::new(ClosureExpr {
            span: span.join(body.span()),
            name: None,
            params,
            body: Rc::new(body),
        })));
    }

    // Find out which kind of collection this is.
    Some(match items.as_slice() {
        [] => array(p, items, span),
        [CallArg::Pos(_)] if !has_comma => match items.into_iter().next() {
            Some(CallArg::Pos(expr)) => Expr::Group(Box::new(GroupExpr { span, expr })),
            _ => unreachable!(),
        },
        [CallArg::Pos(_), ..] => array(p, items, span),
        [CallArg::Named(_), ..] => dict(p, items, span),
        [CallArg::Spread(expr), ..] => {
            p.error(expr.span(), "spreading is not allowed here");
            return None;
        }
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
                p.expected_at(pos, "comma");
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
    if p.eat_if(Token::Dots) {
        return expr(p).map(CallArg::Spread);
    }

    let first = expr(p)?;
    if p.eat_if(Token::Colon) {
        if let Expr::Ident(name) = first {
            Some(CallArg::Named(Named { name: *name, expr: expr(p)? }))
        } else {
            p.error(first.span(), "expected identifier");
            expr(p);
            None
        }
    } else {
        Some(CallArg::Pos(first))
    }
}

/// Convert a collection into an array, producing errors for anything other than
/// expressions.
fn array(p: &mut Parser, items: Vec<CallArg>, span: Span) -> Expr {
    let iter = items.into_iter().filter_map(|item| match item {
        CallArg::Pos(expr) => Some(expr),
        CallArg::Named(_) => {
            p.error(item.span(), "expected expression, found named pair");
            None
        }
        CallArg::Spread(_) => {
            p.error(item.span(), "spreading is not allowed here");
            None
        }
    });
    Expr::Array(Box::new(ArrayExpr { span, items: iter.collect() }))
}

/// Convert a collection into a dictionary, producing errors for anything other
/// than named pairs.
fn dict(p: &mut Parser, items: Vec<CallArg>, span: Span) -> Expr {
    let iter = items.into_iter().filter_map(|item| match item {
        CallArg::Named(named) => Some(named),
        CallArg::Pos(_) => {
            p.error(item.span(), "expected named pair, found expression");
            None
        }
        CallArg::Spread(_) => {
            p.error(item.span(), "spreading is not allowed here");
            None
        }
    });
    Expr::Dict(Box::new(DictExpr { span, items: iter.collect() }))
}

/// Convert a collection into a list of parameters, producing errors for
/// anything other than identifiers, spread operations and named pairs.
fn params(p: &mut Parser, items: Vec<CallArg>) -> Vec<ClosureParam> {
    let iter = items.into_iter().filter_map(|item| match item {
        CallArg::Pos(Expr::Ident(ident)) => Some(ClosureParam::Pos(*ident)),
        CallArg::Named(named) => Some(ClosureParam::Named(named)),
        CallArg::Spread(Expr::Ident(ident)) => Some(ClosureParam::Sink(*ident)),
        _ => {
            p.error(item.span(), "expected identifier");
            None
        }
    });
    iter.collect()
}

/// Convert a collection into a list of identifiers, producing errors for
/// anything other than identifiers.
fn idents(p: &mut Parser, items: Vec<CallArg>) -> Vec<Ident> {
    let iter = items.into_iter().filter_map(|item| match item {
        CallArg::Pos(Expr::Ident(ident)) => Some(*ident),
        _ => {
            p.error(item.span(), "expected identifier");
            None
        }
    });
    iter.collect()
}

// Parse a template block: `[...]`.
fn template(p: &mut Parser) -> Expr {
    p.start_group(Group::Bracket, TokenMode::Markup);
    let tree = markup(p);
    let span = p.end_group();
    Expr::Template(Box::new(TemplateExpr { span, body: tree }))
}

/// Parse a code block: `{...}`.
fn block(p: &mut Parser, scoping: bool) -> Expr {
    p.start_group(Group::Brace, TokenMode::Code);
    let mut exprs = vec![];
    while !p.eof() {
        p.start_group(Group::Stmt, TokenMode::Code);
        if let Some(expr) = expr(p) {
            exprs.push(expr);
            if !p.eof() {
                p.expected_at(p.prev_end(), "semicolon or line break");
            }
        }
        p.end_group();

        // Forcefully skip over newlines since the group's contents can't.
        p.eat_while(|t| matches!(t, Token::Space(_)));
    }
    let span = p.end_group();
    Expr::Block(Box::new(BlockExpr { span, exprs, scoping }))
}

/// Parse a function call.
fn call(p: &mut Parser, callee: Expr) -> Option<Expr> {
    let mut args = match p.peek_direct() {
        Some(Token::LeftParen) => args(p),
        Some(Token::LeftBracket) => CallArgs {
            span: Span::at(p.id(), callee.span().end),
            items: vec![],
        },
        _ => {
            p.expected_at(p.prev_end(), "argument list");
            return None;
        }
    };

    if p.peek_direct() == Some(Token::LeftBracket) {
        let body = template(p);
        args.items.push(CallArg::Pos(body));
    }

    Some(Expr::Call(Box::new(CallExpr {
        span: p.span_from(callee.span().start),
        callee,
        args,
    })))
}

/// Parse the arguments to a function call.
fn args(p: &mut Parser) -> CallArgs {
    p.start_group(Group::Paren, TokenMode::Code);
    let items = collection(p).0;
    let span = p.end_group();
    CallArgs { span, items }
}

/// Parse a with expression.
fn with_expr(p: &mut Parser, callee: Expr) -> Option<Expr> {
    if p.peek() == Some(Token::LeftParen) {
        Some(Expr::With(Box::new(WithExpr {
            span: p.span_from(callee.span().start),
            callee,
            args: args(p),
        })))
    } else {
        p.expected("argument list");
        None
    }
}

/// Parse a let expression.
fn let_expr(p: &mut Parser) -> Option<Expr> {
    let start = p.next_start();
    p.eat_assert(Token::Let);

    let mut let_expr = None;
    if let Some(binding) = ident(p) {
        let mut init = None;

        if p.eat_if(Token::With) {
            init = with_expr(p, Expr::Ident(Box::new(binding.clone())));
        } else {
            // If a parenthesis follows, this is a function definition.
            let mut maybe_params = None;
            if p.peek_direct() == Some(Token::LeftParen) {
                p.start_group(Group::Paren, TokenMode::Code);
                let items = collection(p).0;
                maybe_params = Some(params(p, items));
                p.end_group();
            }

            if p.eat_if(Token::Eq) {
                init = expr(p);
            } else if maybe_params.is_some() {
                // Function definitions must have a body.
                p.expected_at(p.prev_end(), "body");
            }

            // Rewrite into a closure expression if it's a function definition.
            if let Some(params) = maybe_params {
                let body = init?;
                init = Some(Expr::Closure(Box::new(ClosureExpr {
                    span: binding.span.join(body.span()),
                    name: Some(binding.clone()),
                    params,
                    body: Rc::new(body),
                })));
            }
        }

        let_expr = Some(Expr::Let(Box::new(LetExpr {
            span: p.span_from(start),
            binding,
            init,
        })));
    }

    let_expr
}

/// Parse an if expresion.
fn if_expr(p: &mut Parser) -> Option<Expr> {
    let start = p.next_start();
    p.eat_assert(Token::If);

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

            if_expr = Some(Expr::If(Box::new(IfExpr {
                span: p.span_from(start),
                condition,
                if_body,
                else_body,
            })));
        }
    }

    if_expr
}

/// Parse a while expresion.
fn while_expr(p: &mut Parser) -> Option<Expr> {
    let start = p.next_start();
    p.eat_assert(Token::While);

    let mut while_expr = None;
    if let Some(condition) = expr(p) {
        if let Some(body) = body(p) {
            while_expr = Some(Expr::While(Box::new(WhileExpr {
                span: p.span_from(start),
                condition,
                body,
            })));
        }
    }

    while_expr
}

/// Parse a for expression.
fn for_expr(p: &mut Parser) -> Option<Expr> {
    let start = p.next_start();
    p.eat_assert(Token::For);

    let mut for_expr = None;
    if let Some(pattern) = for_pattern(p) {
        if p.eat_expect(Token::In) {
            if let Some(iter) = expr(p) {
                if let Some(body) = body(p) {
                    for_expr = Some(Expr::For(Box::new(ForExpr {
                        span: p.span_from(start),
                        pattern,
                        iter,
                        body,
                    })));
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
    p.eat_assert(Token::Import);

    let imports = if p.eat_if(Token::Star) {
        // This is the wildcard scenario.
        Imports::Wildcard
    } else {
        // This is the list of identifiers scenario.
        p.start_group(Group::Imports, TokenMode::Code);
        let items = collection(p).0;
        if items.is_empty() {
            p.expected_at(p.prev_end(), "import items");
        }
        p.end_group();
        Imports::Idents(idents(p, items))
    };

    let mut import_expr = None;
    if p.eat_expect(Token::From) {
        if let Some(path) = expr(p) {
            import_expr = Some(Expr::Import(Box::new(ImportExpr {
                span: p.span_from(start),
                imports,
                path,
            })));
        }
    }

    import_expr
}

/// Parse an include expression.
fn include_expr(p: &mut Parser) -> Option<Expr> {
    let start = p.next_start();
    p.eat_assert(Token::Include);

    expr(p).map(|path| {
        Expr::Include(Box::new(IncludeExpr { span: p.span_from(start), path }))
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
            p.expected_at(p.prev_end(), "body");
            None
        }
    }
}
