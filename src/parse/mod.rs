//! Parsing and tokenization.

mod incremental;
mod parser;
mod resolve;
mod scanner;
mod tokens;

pub use incremental::*;
pub use parser::*;
pub use resolve::*;
pub use scanner::*;
pub use tokens::*;

use std::rc::Rc;

use crate::syntax::ast::{Associativity, BinOp, UnOp};
use crate::syntax::{ErrorPos, Green, GreenNode, NodeKind};
use crate::util::EcoString;

/// Parse a source file.
pub fn parse(src: &str) -> Rc<GreenNode> {
    let mut p = Parser::new(src, TokenMode::Markup);
    markup(&mut p);
    match p.finish().into_iter().next() {
        Some(Green::Node(node)) => node,
        _ => unreachable!(),
    }
}

/// Parse an atomic primary. Returns `Some` if all of the input was consumed.
pub fn parse_atomic(
    prefix: &str,
    src: &str,
    _: bool,
    _: usize,
) -> Option<(Vec<Green>, bool)> {
    let mut p = Parser::with_prefix(prefix, src, TokenMode::Code);
    primary(&mut p, true).ok()?;
    p.consume_unterminated()
}

/// Parse an atomic primary. Returns `Some` if all of the input was consumed.
pub fn parse_atomic_markup(
    prefix: &str,
    src: &str,
    _: bool,
    _: usize,
) -> Option<(Vec<Green>, bool)> {
    let mut p = Parser::with_prefix(prefix, src, TokenMode::Markup);
    markup_expr(&mut p);
    p.consume_unterminated()
}

/// Parse some markup. Returns `Some` if all of the input was consumed.
pub fn parse_markup(
    prefix: &str,
    src: &str,
    _: bool,
    min_column: usize,
) -> Option<(Vec<Green>, bool)> {
    let mut p = Parser::with_prefix(prefix, src, TokenMode::Markup);
    if min_column == 0 {
        markup(&mut p);
    } else {
        markup_indented(&mut p, min_column);
    }
    p.consume()
}

/// Parse some markup without the topmost node. Returns `Some` if all of the
/// input was consumed.
pub fn parse_markup_elements(
    prefix: &str,
    src: &str,
    mut at_start: bool,
    _: usize,
) -> Option<(Vec<Green>, bool)> {
    let mut p = Parser::with_prefix(prefix, src, TokenMode::Markup);
    while !p.eof() {
        markup_node(&mut p, &mut at_start);
    }
    p.consume()
}

/// Parse a template literal. Returns `Some` if all of the input was consumed.
pub fn parse_template(
    prefix: &str,
    src: &str,
    _: bool,
    _: usize,
) -> Option<(Vec<Green>, bool)> {
    let mut p = Parser::with_prefix(prefix, src, TokenMode::Code);
    if !p.at(&NodeKind::LeftBracket) {
        return None;
    }

    template(&mut p);
    p.consume()
}

/// Parse a code block. Returns `Some` if all of the input was consumed.
pub fn parse_block(
    prefix: &str,
    src: &str,
    _: bool,
    _: usize,
) -> Option<(Vec<Green>, bool)> {
    let mut p = Parser::with_prefix(prefix, src, TokenMode::Code);
    if !p.at(&NodeKind::LeftBrace) {
        return None;
    }

    block(&mut p);
    p.consume()
}

/// Parse a comment. Returns `Some` if all of the input was consumed.
pub fn parse_comment(
    prefix: &str,
    src: &str,
    _: bool,
    _: usize,
) -> Option<(Vec<Green>, bool)> {
    let mut p = Parser::with_prefix(prefix, src, TokenMode::Code);
    comment(&mut p).ok()?;
    p.consume()
}

/// Parse markup.
fn markup(p: &mut Parser) {
    markup_while(p, true, 0, &mut |_| true)
}

/// Parse markup that stays right of the given column.
fn markup_indented(p: &mut Parser, column: usize) {
    p.eat_while(|t| match t {
        NodeKind::Space(n) => *n == 0,
        NodeKind::LineComment | NodeKind::BlockComment => true,
        _ => false,
    });

    markup_while(p, false, column, &mut |p| match p.peek() {
        Some(NodeKind::Space(n)) if *n >= 1 => p.column(p.current_end()) >= column,
        _ => true,
    })
}

/// Parse a syntax tree while the peeked NodeKind satisifies a condition.
///
/// If `at_start` is true, things like headings that may only appear at the
/// beginning of a line or template are allowed.
fn markup_while<F>(p: &mut Parser, mut at_start: bool, column: usize, f: &mut F)
where
    F: FnMut(&mut Parser) -> bool,
{
    p.perform(NodeKind::Markup(column), |p| {
        while !p.eof() && f(p) {
            markup_node(p, &mut at_start);
        }
    });
}

/// Parse a markup node.
fn markup_node(p: &mut Parser, at_start: &mut bool) {
    let token = match p.peek() {
        Some(t) => t,
        None => return,
    };

    match token {
        // Whitespace.
        NodeKind::Space(newlines) => {
            *at_start |= *newlines > 0;
            if *newlines < 2 {
                p.eat();
            } else {
                p.convert(NodeKind::Parbreak);
            }
            return;
        }

        // Comments.
        NodeKind::LineComment | NodeKind::BlockComment => {
            p.eat();
            return;
        }

        // Text and markup.
        NodeKind::Text(_)
        | NodeKind::EnDash
        | NodeKind::EmDash
        | NodeKind::NonBreakingSpace
        | NodeKind::Emph
        | NodeKind::Strong
        | NodeKind::Linebreak
        | NodeKind::Raw(_)
        | NodeKind::Math(_)
        | NodeKind::Escape(_) => {
            p.eat();
        }

        NodeKind::Eq => heading(p, *at_start),
        NodeKind::Minus => list_node(p, *at_start),
        NodeKind::EnumNumbering(_) => enum_node(p, *at_start),

        // Hashtag + keyword / identifier.
        NodeKind::Ident(_)
        | NodeKind::Let
        | NodeKind::Set
        | NodeKind::Show
        | NodeKind::Wrap
        | NodeKind::If
        | NodeKind::While
        | NodeKind::For
        | NodeKind::Import
        | NodeKind::Include => markup_expr(p),

        // Block and template.
        NodeKind::LeftBrace => block(p),
        NodeKind::LeftBracket => template(p),

        NodeKind::Error(_, _) => p.eat(),
        _ => p.unexpected(),
    };

    *at_start = false;
}

/// Parse a heading.
fn heading(p: &mut Parser, at_start: bool) {
    let marker = p.marker();
    let current_start = p.current_start();
    p.eat_assert(&NodeKind::Eq);
    while p.eat_if(&NodeKind::Eq) {}

    if at_start && p.peek().map_or(true, |kind| kind.is_whitespace()) {
        let column = p.column(p.prev_end());
        markup_indented(p, column);
        marker.end(p, NodeKind::Heading);
    } else {
        let text = p.get(current_start .. p.prev_end()).into();
        marker.convert(p, NodeKind::TextInLine(text));
    }
}

/// Parse a single list item.
fn list_node(p: &mut Parser, at_start: bool) {
    let marker = p.marker();
    let text: EcoString = p.peek_src().into();
    p.eat_assert(&NodeKind::Minus);

    if at_start && p.peek().map_or(true, |kind| kind.is_whitespace()) {
        let column = p.column(p.prev_end());
        markup_indented(p, column);
        marker.end(p, NodeKind::List);
    } else {
        marker.convert(p, NodeKind::TextInLine(text));
    }
}

/// Parse a single enum item.
fn enum_node(p: &mut Parser, at_start: bool) {
    let marker = p.marker();
    let text: EcoString = p.peek_src().into();
    p.eat();

    if at_start && p.peek().map_or(true, |kind| kind.is_whitespace()) {
        let column = p.column(p.prev_end());
        markup_indented(p, column);
        marker.end(p, NodeKind::Enum);
    } else {
        marker.convert(p, NodeKind::TextInLine(text));
    }
}

/// Parse an expression within markup mode.
fn markup_expr(p: &mut Parser) {
    if let Some(token) = p.peek() {
        let stmt = matches!(
            token,
            NodeKind::Let
                | NodeKind::Set
                | NodeKind::Show
                | NodeKind::Wrap
                | NodeKind::Import
        );
        let group = if stmt { Group::Stmt } else { Group::Expr };

        p.start_group(group);
        let res = expr_prec(p, true, 0);
        if stmt && res.is_ok() && !p.eof() {
            p.expected_at("semicolon or line break");
        }
        p.end_group();
    }
}

/// Parse an expression.
fn expr(p: &mut Parser) -> ParseResult {
    expr_prec(p, false, 0)
}

/// Parse an expression with operators having at least the minimum precedence.
///
/// If `atomic` is true, this does not parse binary operations and arrow
/// functions, which is exactly what we want in a shorthand expression directly
/// in markup.
///
/// Stops parsing at operations with lower precedence than `min_prec`,
fn expr_prec(p: &mut Parser, atomic: bool, min_prec: usize) -> ParseResult {
    let marker = p.marker();

    // Start the unary expression.
    match p.peek().and_then(UnOp::from_token) {
        Some(op) if !atomic => {
            p.eat();
            let prec = op.precedence();
            expr_prec(p, atomic, prec)?;
            marker.end(p, NodeKind::Unary);
        }
        _ => primary(p, atomic)?,
    };

    loop {
        // Exclamation mark, parenthesis or bracket means this is a function
        // call.
        if let Some(NodeKind::LeftParen | NodeKind::LeftBracket) = p.peek_direct() {
            call(p, marker)?;
            continue;
        }

        if atomic {
            break;
        }

        if p.at(&NodeKind::With) {
            with_expr(p, marker)?;
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

        marker.perform(p, NodeKind::Binary, |p| expr_prec(p, atomic, prec))?;
    }

    Ok(())
}

/// Parse a primary expression.
fn primary(p: &mut Parser, atomic: bool) -> ParseResult {
    if literal(p) {
        return Ok(());
    }

    match p.peek() {
        // Things that start with an identifier.
        Some(NodeKind::Ident(_)) => {
            let marker = p.marker();
            p.eat();

            // Arrow means this is a closure's lone parameter.
            if !atomic && p.at(&NodeKind::Arrow) {
                marker.end(p, NodeKind::ClosureParams);
                p.eat_assert(&NodeKind::Arrow);
                marker.perform(p, NodeKind::Closure, expr)
            } else {
                Ok(())
            }
        }

        // Structures.
        Some(NodeKind::LeftParen) => parenthesized(p, atomic),
        Some(NodeKind::LeftBracket) => {
            template(p);
            Ok(())
        }
        Some(NodeKind::LeftBrace) => {
            block(p);
            Ok(())
        }

        // Keywords.
        Some(NodeKind::Let) => let_expr(p),
        Some(NodeKind::Set) => set_expr(p),
        Some(NodeKind::Show) => show_expr(p),
        Some(NodeKind::Wrap) => wrap_expr(p),
        Some(NodeKind::If) => if_expr(p),
        Some(NodeKind::While) => while_expr(p),
        Some(NodeKind::For) => for_expr(p),
        Some(NodeKind::Import) => import_expr(p),
        Some(NodeKind::Include) => include_expr(p),

        Some(NodeKind::Error(_, _)) => {
            p.eat();
            Err(ParseError)
        }

        // Nothing.
        _ => {
            p.expected("expression");
            Err(ParseError)
        }
    }
}

/// Parse a literal.
fn literal(p: &mut Parser) -> bool {
    match p.peek() {
        // Basic values.
        Some(
            NodeKind::None
            | NodeKind::Auto
            | NodeKind::Int(_)
            | NodeKind::Float(_)
            | NodeKind::Bool(_)
            | NodeKind::Fraction(_)
            | NodeKind::Length(_, _)
            | NodeKind::Angle(_, _)
            | NodeKind::Percentage(_)
            | NodeKind::Str(_),
        ) => {
            p.eat();
            true
        }

        _ => false,
    }
}

/// Parse something that starts with a parenthesis, which can be either of:
/// - Array literal
/// - Dictionary literal
/// - Parenthesized expression
/// - Parameter list of closure expression
fn parenthesized(p: &mut Parser, atomic: bool) -> ParseResult {
    let marker = p.marker();

    p.start_group(Group::Paren);
    let colon = p.eat_if(&NodeKind::Colon);
    let kind = collection(p).0;
    p.end_group();

    // Leading colon makes this a (empty) dictionary.
    if colon {
        dict(p, marker);
        return Ok(());
    }

    // Arrow means this is a closure's parameter list.
    if !atomic && p.at(&NodeKind::Arrow) {
        params(p, marker);
        p.eat_assert(&NodeKind::Arrow);
        return marker.perform(p, NodeKind::Closure, expr);
    }

    // Transform into the identified collection.
    match kind {
        CollectionKind::Group => marker.end(p, NodeKind::Group),
        CollectionKind::Positional => array(p, marker),
        CollectionKind::Named => dict(p, marker),
    }

    Ok(())
}

/// The type of a collection.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum CollectionKind {
    /// The collection is only one item and has no comma.
    Group,
    /// The collection starts with a positional and has more items or a trailing
    /// comma.
    Positional,
    /// The collection starts with a named item.
    Named,
}

/// Parse a collection.
///
/// Returns the length of the collection and whether the literal contained any
/// commas.
fn collection(p: &mut Parser) -> (CollectionKind, usize) {
    let mut kind = CollectionKind::Positional;
    let mut items = 0;
    let mut can_group = true;
    let mut error = false;
    let mut missing_coma: Option<Marker> = None;

    while !p.eof() {
        if let Ok(item_kind) = item(p) {
            if items == 0 && item_kind == NodeKind::Named {
                kind = CollectionKind::Named;
                can_group = false;
            }

            if item_kind == NodeKind::Spread {
                can_group = false;
            }

            items += 1;

            if let Some(marker) = missing_coma.take() {
                marker.expected(p, "comma");
            }

            if p.eof() {
                break;
            }

            if p.eat_if(&NodeKind::Comma) {
                can_group = false;
            } else {
                missing_coma = Some(p.trivia_start());
            }
        } else {
            error = true;
        }
    }

    if error || (can_group && items == 1) {
        kind = CollectionKind::Group;
    }

    (kind, items)
}

/// Parse an expression or a named pair, returning whether it's a spread or a
/// named pair.
fn item(p: &mut Parser) -> ParseResult<NodeKind> {
    let marker = p.marker();
    if p.eat_if(&NodeKind::Dots) {
        marker.perform(p, NodeKind::Spread, expr)?;
        return Ok(NodeKind::Spread);
    }

    expr(p)?;

    if p.at(&NodeKind::Colon) {
        marker.perform(p, NodeKind::Named, |p| {
            if let Some(NodeKind::Ident(_)) = marker.peek(p).map(|c| c.kind()) {
                p.eat();
                expr(p)
            } else {
                let error = NodeKind::Error(ErrorPos::Full, "expected identifier".into());
                marker.end(p, error);
                p.eat();
                expr(p).ok();
                Err(ParseError)
            }
        })?;

        Ok(NodeKind::Named)
    } else {
        Ok(NodeKind::None)
    }
}

/// Convert a collection into an array, producing errors for anything other than
/// expressions.
fn array(p: &mut Parser, marker: Marker) {
    marker.filter_children(p, |x| match x.kind() {
        NodeKind::Named => Err("expected expression, found named pair"),
        NodeKind::Spread => Err("spreading is not allowed here"),
        _ => Ok(()),
    });
    marker.end(p, NodeKind::Array);
}

/// Convert a collection into a dictionary, producing errors for anything other
/// than named pairs.
fn dict(p: &mut Parser, marker: Marker) {
    marker.filter_children(p, |x| match x.kind() {
        kind if kind.is_paren() => Ok(()),
        NodeKind::Named | NodeKind::Comma | NodeKind::Colon => Ok(()),
        NodeKind::Spread => Err("spreading is not allowed here"),
        _ => Err("expected named pair, found expression"),
    });
    marker.end(p, NodeKind::Dict);
}

/// Convert a collection into a list of parameters, producing errors for
/// anything other than identifiers, spread operations and named pairs.
fn params(p: &mut Parser, marker: Marker) {
    marker.filter_children(p, |x| match x.kind() {
        kind if kind.is_paren() => Ok(()),
        NodeKind::Named | NodeKind::Comma | NodeKind::Ident(_) => Ok(()),
        NodeKind::Spread
            if matches!(
                x.children().last().map(|child| child.kind()),
                Some(&NodeKind::Ident(_))
            ) =>
        {
            Ok(())
        }
        _ => Err("expected identifier"),
    });
    marker.end(p, NodeKind::ClosureParams);
}

// Parse a template block: `[...]`.
fn template(p: &mut Parser) {
    p.perform(NodeKind::Template, |p| {
        p.start_group(Group::Bracket);
        markup(p);
        p.end_group();
    });
}

/// Parse a code block: `{...}`.
fn block(p: &mut Parser) {
    p.perform(NodeKind::Block, |p| {
        p.start_group(Group::Brace);
        while !p.eof() {
            p.start_group(Group::Stmt);
            if expr(p).is_ok() && !p.eof() {
                p.expected_at("semicolon or line break");
            }
            p.end_group();

            // Forcefully skip over newlines since the group's contents can't.
            p.eat_while(|t| matches!(t, NodeKind::Space(_)));
        }
        p.end_group();
    });
}

/// Parse a function call.
fn call(p: &mut Parser, callee: Marker) -> ParseResult {
    callee.perform(p, NodeKind::Call, |p| args(p, true, true))
}

/// Parse the arguments to a function call.
fn args(p: &mut Parser, direct: bool, brackets: bool) -> ParseResult {
    match if direct { p.peek_direct() } else { p.peek() } {
        Some(NodeKind::LeftParen) => {}
        Some(NodeKind::LeftBracket) if brackets => {}
        _ => {
            p.expected("argument list");
            return Err(ParseError);
        }
    }

    p.perform(NodeKind::CallArgs, |p| {
        if p.at(&NodeKind::LeftParen) {
            p.start_group(Group::Paren);
            collection(p);
            p.end_group();
        }

        while brackets && p.peek_direct() == Some(&NodeKind::LeftBracket) {
            template(p);
        }
    });

    Ok(())
}

/// Parse a with expression.
fn with_expr(p: &mut Parser, marker: Marker) -> ParseResult {
    marker.perform(p, NodeKind::WithExpr, |p| {
        p.eat_assert(&NodeKind::With);
        args(p, false, false)
    })
}

/// Parse a let expression.
fn let_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::LetExpr, |p| {
        p.eat_assert(&NodeKind::Let);

        let marker = p.marker();
        ident(p)?;

        if p.at(&NodeKind::With) {
            with_expr(p, marker)?;
        } else {
            // If a parenthesis follows, this is a function definition.
            let has_params = p.peek_direct() == Some(&NodeKind::LeftParen);
            if has_params {
                let marker = p.marker();
                p.start_group(Group::Paren);
                collection(p);
                p.end_group();
                params(p, marker);
            }

            if p.eat_if(&NodeKind::Eq) {
                expr(p)?;
            } else if has_params {
                // Function definitions must have a body.
                p.expected_at("body");
            }

            // Rewrite into a closure expression if it's a function definition.
            if has_params {
                marker.end(p, NodeKind::Closure);
            }
        }

        Ok(())
    })
}

/// Parse a set expression.
fn set_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::SetExpr, |p| {
        p.eat_assert(&NodeKind::Set);
        ident(p)?;
        args(p, true, false)
    })
}

/// Parse a show expression.
fn show_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ShowExpr, |p| {
        p.eat_assert(&NodeKind::Show);
        expr(p)?;
        p.eat_expect(&NodeKind::As)?;
        expr(p)
    })
}

/// Parse a wrap expression.
fn wrap_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::WrapExpr, |p| {
        p.eat_assert(&NodeKind::Wrap);
        ident(p)?;
        p.eat_expect(&NodeKind::In)?;
        expr(p)
    })
}

/// Parse an if expresion.
fn if_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::IfExpr, |p| {
        p.eat_assert(&NodeKind::If);

        expr(p)?;
        body(p)?;

        if p.eat_if(&NodeKind::Else) {
            if p.at(&NodeKind::If) {
                if_expr(p)?;
            } else {
                body(p)?;
            }
        }

        Ok(())
    })
}

/// Parse a while expresion.
fn while_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::WhileExpr, |p| {
        p.eat_assert(&NodeKind::While);
        expr(p)?;
        body(p)
    })
}

/// Parse a for expression.
fn for_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ForExpr, |p| {
        p.eat_assert(&NodeKind::For);
        for_pattern(p)?;
        p.eat_expect(&NodeKind::In)?;
        expr(p)?;
        body(p)
    })
}

/// Parse a for loop pattern.
fn for_pattern(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ForPattern, |p| {
        ident(p)?;
        if p.eat_if(&NodeKind::Comma) {
            ident(p)?;
        }
        Ok(())
    })
}

/// Parse an import expression.
fn import_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ImportExpr, |p| {
        p.eat_assert(&NodeKind::Import);

        if !p.eat_if(&NodeKind::Star) {
            // This is the list of identifiers scenario.
            p.perform(NodeKind::ImportItems, |p| {
                p.start_group(Group::Imports);
                let marker = p.marker();
                let items = collection(p).1;
                if items == 0 {
                    p.expected_at("import items");
                }
                p.end_group();

                marker.filter_children(p, |n| match n.kind() {
                    NodeKind::Ident(_) | NodeKind::Comma => Ok(()),
                    _ => Err("expected identifier"),
                });
            });
        };

        p.eat_expect(&NodeKind::From)?;
        expr(p)
    })
}

/// Parse an include expression.
fn include_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::IncludeExpr, |p| {
        p.eat_assert(&NodeKind::Include);
        expr(p)
    })
}

/// Parse an identifier.
fn ident(p: &mut Parser) -> ParseResult {
    match p.peek() {
        Some(NodeKind::Ident(_)) => {
            p.eat();
            Ok(())
        }
        _ => {
            p.expected("identifier");
            Err(ParseError)
        }
    }
}

/// Parse a control flow body.
fn body(p: &mut Parser) -> ParseResult {
    match p.peek() {
        Some(NodeKind::LeftBracket) => template(p),
        Some(NodeKind::LeftBrace) => block(p),
        _ => {
            p.expected_at("body");
            return Err(ParseError);
        }
    }
    Ok(())
}

/// Parse a comment.
fn comment(p: &mut Parser) -> ParseResult {
    match p.peek() {
        Some(NodeKind::LineComment | NodeKind::BlockComment) => {
            p.eat();
            Ok(())
        }
        _ => Err(ParseError),
    }
}
