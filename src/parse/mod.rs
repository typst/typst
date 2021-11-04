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

use crate::syntax::ast::{Associativity, BinOp, UnOp};
use crate::syntax::{ErrorPosition, GreenNode, NodeKind};
use crate::util::EcoString;

type ParseResult = Result<(), ()>;

/// Parse a source file.
pub fn parse(source: &str) -> Rc<GreenNode> {
    let mut p = Parser::new(source);
    markup(&mut p);
    p.finish()
}

/// Parse markup.
fn markup(p: &mut Parser) {
    markup_while(p, true, &mut |_| true)
}

/// Parse markup that stays right of the given column.
fn markup_indented(p: &mut Parser, column: usize) {
    p.eat_while(|t| match t {
        NodeKind::Space(n) => *n == 0,
        NodeKind::LineComment | NodeKind::BlockComment => true,
        _ => false,
    });

    markup_while(p, false, &mut |p| match p.peek() {
        Some(NodeKind::Space(n)) if *n >= 1 => p.column(p.next_end()) >= column,
        _ => true,
    })
}

/// Parse a syntax tree while the peeked NodeKind satisifies a condition.
///
/// If `at_start` is true, things like headings that may only appear at the
/// beginning of a line or template are allowed.
fn markup_while<F>(p: &mut Parser, mut at_start: bool, f: &mut F)
where
    F: FnMut(&mut Parser) -> bool,
{
    p.start();
    while !p.eof() && f(p) {
        markup_node(p, &mut at_start);
    }

    p.end(NodeKind::Markup);
}

/// Parse a markup node.
fn markup_node(p: &mut Parser, at_start: &mut bool) -> ParseResult {
    let token = match p.peek() {
        Some(t) => t,
        None => return Ok(()),
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
            return Ok(());
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
        | NodeKind::UnicodeEscape(_) => {
            p.eat();
            Ok(())
        }

        NodeKind::Eq if *at_start => heading(p),
        NodeKind::ListBullet if *at_start => list_node(p),
        NodeKind::EnumNumbering(_) if *at_start => enum_node(p),

        // Line-based markup that is not currently at the start of the line.
        NodeKind::Eq | NodeKind::ListBullet | NodeKind::EnumNumbering(_) => {
            p.convert(NodeKind::Text(p.peek_src().into()));
            Ok(())
        }

        // Hashtag + keyword / identifier.
        NodeKind::Ident(_)
        | NodeKind::Let
        | NodeKind::If
        | NodeKind::While
        | NodeKind::For
        | NodeKind::Import
        | NodeKind::Include => {
            let stmt = matches!(token, NodeKind::Let | NodeKind::Import);
            let group = if stmt { Group::Stmt } else { Group::Expr };

            p.start_group(group, TokenMode::Code);
            let res = expr_with(p, true, 0);
            if stmt && res.is_ok() && !p.eof() {
                p.expected_at("semicolon or line break");
            }
            p.end_group()
        }

        // Block and template.
        NodeKind::LeftBrace => block(p),
        NodeKind::LeftBracket => template(p),

        // Comments.
        NodeKind::LineComment | NodeKind::BlockComment => {
            p.eat();
            return Ok(());
        }

        NodeKind::Error(_, _) => {
            p.eat();
            Ok(())
        }

        _ => {
            p.unexpected();
            Err(())
        }
    }?;

    *at_start = false;
    Ok(())
}

/// Parse a heading.
fn heading(p: &mut Parser) -> ParseResult {
    p.start();
    p.eat_assert(&NodeKind::Eq);

    // Count depth.
    let mut level: usize = 1;
    while p.eat_if(&NodeKind::Eq) {
        level += 1;
    }

    if level > 6 {
        p.end(NodeKind::Text(EcoString::from('=').repeat(level)));
    } else {
        let column = p.column(p.prev_end());
        markup_indented(p, column);
        p.end(NodeKind::Heading);
    }
    Ok(())
}

/// Parse a single list item.
fn list_node(p: &mut Parser) -> ParseResult {
    p.start();
    p.eat_assert(&NodeKind::ListBullet);
    let column = p.column(p.prev_end());
    markup_indented(p, column);
    p.end(NodeKind::List);
    Ok(())
}

/// Parse a single enum item.
fn enum_node(p: &mut Parser) -> ParseResult {
    p.start();
    p.eat();
    let column = p.column(p.prev_end());
    markup_indented(p, column);
    p.end(NodeKind::Enum);
    Ok(())
}

/// Parse an expression.
fn expr(p: &mut Parser) -> ParseResult {
    expr_with(p, false, 0)
}

/// Parse an expression with operators having at least the minimum precedence.
///
/// If `atomic` is true, this does not parse binary operations and arrow
/// functions, which is exactly what we want in a shorthand expression directly
/// in markup.
///
/// Stops parsing at operations with lower precedence than `min_prec`,
fn expr_with(p: &mut Parser, atomic: bool, min_prec: usize) -> ParseResult {
    let marker = p.marker();

    // Start the unary expression.
    match p.eat_map(|x| UnOp::from_token(&x)) {
        Some(op) => {
            let prec = op.precedence();
            expr_with(p, atomic, prec)?;

            marker.end(p, NodeKind::Unary);
        }
        None => {
            primary(p, atomic)?;
        }
    };

    loop {
        // Exclamation mark, parenthesis or bracket means this is a function
        // call.
        if matches!(
            p.peek_direct(),
            Some(NodeKind::LeftParen | NodeKind::LeftBracket)
        ) {
            call(p, &marker);
            continue;
        }

        if atomic {
            break Ok(());
        }

        if p.peek() == Some(&NodeKind::With) {
            with_expr(p, &marker)?;
        }

        let op = match p.peek().and_then(BinOp::from_token) {
            Some(binop) => binop,
            None => {
                break Ok(());
            }
        };

        let mut prec = op.precedence();
        if prec < min_prec {
            break Ok(());
        }

        p.eat();

        match op.associativity() {
            Associativity::Left => prec += 1,
            Associativity::Right => {}
        }

        if expr_with(p, atomic, prec).is_err() {
            break Ok(());
        }

        marker.end(p, NodeKind::Binary);
    }
}

/// Parse a primary expression.
fn primary(p: &mut Parser, atomic: bool) -> ParseResult {
    let lit = literal(p);
    if lit.is_ok() {
        return lit;
    }

    match p.peek() {
        // Things that start with an identifier.
        Some(NodeKind::Ident(_)) => {
            // Start closure params.
            let marker = p.marker();
            p.eat();

            // Arrow means this is a closure's lone parameter.
            if !atomic && p.peek() == Some(&NodeKind::Arrow) {
                marker.end(p, NodeKind::ClosureParams);
                p.eat();

                let e = expr(p);
                marker.end(p, NodeKind::Closure);
                e
            } else {
                Ok(())
            }
        }

        // Structures.
        Some(NodeKind::LeftParen) => parenthesized(p),
        Some(NodeKind::LeftBracket) => template(p),
        Some(NodeKind::LeftBrace) => block(p),

        // Keywords.
        Some(NodeKind::Let) => let_expr(p),
        Some(NodeKind::If) => if_expr(p),
        Some(NodeKind::While) => while_expr(p),
        Some(NodeKind::For) => for_expr(p),
        Some(NodeKind::Import) => import_expr(p),
        Some(NodeKind::Include) => include_expr(p),

        Some(NodeKind::Error(_, _)) => {
            p.eat();
            Ok(())
        }

        // Nothing.
        _ => {
            p.expected("expression");
            Err(())
        }
    }
}

/// Parse a literal.
fn literal(p: &mut Parser) -> ParseResult {
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
            Ok(())
        }

        _ => Err(()),
    }
}

/// Parse something that starts with a parenthesis, which can be either of:
/// - Array literal
/// - Dictionary literal
/// - Parenthesized expression
/// - Parameter list of closure expression
fn parenthesized(p: &mut Parser) -> ParseResult {
    let marker = p.marker();
    p.start_group(Group::Paren, TokenMode::Code);
    let colon = p.eat_if(&NodeKind::Colon);
    let kind = collection(p).0;
    p.end_group();

    // Leading colon makes this a (empty) dictionary.
    if colon {
        return dict(p, &marker);
    }

    // Arrow means this is a closure's parameter list.
    if p.peek() == Some(&NodeKind::Arrow) {
        params(p, &marker, true);
        marker.end(p, NodeKind::ClosureParams);

        p.eat_assert(&NodeKind::Arrow);

        let r = expr(p);

        marker.end(p, NodeKind::Closure);
        return r;
    }

    // Find out which kind of collection this is.
    match kind {
        CollectionKind::Group => {
            marker.end(p, NodeKind::Group);
            Ok(())
        }
        CollectionKind::Positional => array(p, &marker),
        CollectionKind::Named => dict(p, &marker),
    }
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
    let mut items = 0;
    let mut kind = CollectionKind::Positional;
    let mut has_comma = false;
    let mut missing_coma: Option<Marker> = None;

    while !p.eof() {
        if let Ok(item_kind) = item(p) {
            if items == 0 && item_kind == NodeKind::Named {
                kind = CollectionKind::Named;
            }

            if item_kind == NodeKind::Spread {
                has_comma = true;
            }

            items += 1;

            if let Some(marker) = missing_coma.take() {
                marker.expected_at(p, "comma");
            }

            if p.eof() {
                break;
            }

            if p.eat_if(&NodeKind::Comma) {
                has_comma = true;
            } else {
                missing_coma = Some(p.marker());
            }
        }
    }

    if !has_comma && items == 1 && kind == CollectionKind::Positional {
        kind = CollectionKind::Group;
    }

    (kind, items)
}

/// Parse an expression or a named pair. Returns if this is a named pair.
fn item(p: &mut Parser) -> Result<NodeKind, ()> {
    let marker = p.marker();
    if p.eat_if(&NodeKind::Dots) {
        let r = expr(p);

        marker.end(p, NodeKind::Spread);
        return r.map(|_| NodeKind::Spread);
    }

    let ident_marker = p.marker();
    if expr(p).is_err() {
        return Err(());
    }

    if p.peek() == Some(&NodeKind::Colon) {
        let r = if matches!(p.child(0).unwrap().kind(), &NodeKind::Ident(_)) {
            p.eat();
            expr(p)
        } else {
            ident_marker.end(
                p,
                NodeKind::Error(ErrorPosition::Full, "expected identifier".into()),
            );
            p.eat();

            expr(p);
            Err(())
        };

        marker.end(p, NodeKind::Named);
        r.map(|_| NodeKind::Named)
    } else {
        Ok(p.last_child().unwrap().kind().clone())
    }
}

/// Convert a collection into an array, producing errors for anything other than
/// expressions.
fn array(p: &mut Parser, marker: &Marker) -> ParseResult {
    marker.filter_children(
        p,
        |x| match x.kind() {
            NodeKind::Named | NodeKind::Spread => false,
            _ => true,
        },
        |kind| match kind {
            NodeKind::Named => (
                ErrorPosition::Full,
                "expected expression, found named pair".into(),
            ),
            NodeKind::Spread => {
                (ErrorPosition::Full, "spreading is not allowed here".into())
            }
            _ => unreachable!(),
        },
    );

    marker.end(p, NodeKind::Array);
    Ok(())
}

/// Convert a collection into a dictionary, producing errors for anything other
/// than named pairs.
fn dict(p: &mut Parser, marker: &Marker) -> ParseResult {
    marker.filter_children(
        p,
        |x| {
            x.kind() == &NodeKind::Named
                || x.kind().is_paren()
                || x.kind() == &NodeKind::Comma
                || x.kind() == &NodeKind::Colon
        },
        |kind| match kind {
            NodeKind::Spread => {
                (ErrorPosition::Full, "spreading is not allowed here".into())
            }
            _ => (
                ErrorPosition::Full,
                "expected named pair, found expression".into(),
            ),
        },
    );

    marker.end(p, NodeKind::Dict);
    Ok(())
}

/// Convert a collection into a list of parameters, producing errors for
/// anything other than identifiers, spread operations and named pairs.
fn params(p: &mut Parser, marker: &Marker, allow_parens: bool) {
    marker.filter_children(
        p,
        |x| match x.kind() {
                NodeKind::Named | NodeKind::Comma | NodeKind::Ident(_) => true,
                NodeKind::Spread => matches!(
                    x.children().last().map(|x| x.kind()),
                    Some(&NodeKind::Ident(_))
                ),
                _ => false,
            }
            || (allow_parens && x.kind().is_paren()),
        |_| (ErrorPosition::Full, "expected identifier".into()),
    );
}

// Parse a template block: `[...]`.
fn template(p: &mut Parser) -> ParseResult {
    p.start();
    p.start_group(Group::Bracket, TokenMode::Markup);
    markup(p);
    p.end_group();
    p.end(NodeKind::Template);
    Ok(())
}

/// Parse a code block: `{...}`.
fn block(p: &mut Parser) -> ParseResult {
    p.start();
    p.start_group(Group::Brace, TokenMode::Code);
    while !p.eof() {
        p.start_group(Group::Stmt, TokenMode::Code);
        if expr(p).is_ok() {
            if !p.eof() {
                p.expected_at("semicolon or line break");
            }
        }
        p.end_group();

        // Forcefully skip over newlines since the group's contents can't.
        p.eat_while(|t| matches!(t, NodeKind::Space(_)));
    }
    p.end_group();
    p.end(NodeKind::Block);
    Ok(())
}

/// Parse a function call.
fn call(p: &mut Parser, callee: &Marker) -> ParseResult {
    let res = match p.peek_direct() {
        Some(NodeKind::LeftParen) | Some(NodeKind::LeftBracket) => args(p, true),
        _ => {
            p.expected_at("argument list");
            Err(())
        }
    };

    callee.end(p, NodeKind::Call);
    res
}

/// Parse the arguments to a function call.
fn args(p: &mut Parser, allow_template: bool) -> ParseResult {
    p.start();
    if !allow_template || p.peek_direct() == Some(&NodeKind::LeftParen) {
        p.start_group(Group::Paren, TokenMode::Code);
        collection(p);
        p.end_group();
    }

    while allow_template && p.peek_direct() == Some(&NodeKind::LeftBracket) {
        template(p);
    }

    p.end(NodeKind::CallArgs);
    Ok(())
}

/// Parse a with expression.
fn with_expr(p: &mut Parser, marker: &Marker) -> ParseResult {
    p.eat_assert(&NodeKind::With);

    let res = if p.peek() == Some(&NodeKind::LeftParen) {
        args(p, false)
    } else {
        p.expected("argument list");
        Err(())
    };

    marker.end(p, NodeKind::WithExpr);
    res
}

/// Parse a let expression.
fn let_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::LetExpr, |p| {
        p.eat_assert(&NodeKind::Let);

        let marker = p.marker();
        ident(p)?;

        if p.peek() == Some(&NodeKind::With) {
            with_expr(p, &marker);
        } else {
            // If a parenthesis follows, this is a function definition.
            let has_params = if p.peek_direct() == Some(&NodeKind::LeftParen) {
                p.start();
                p.start_group(Group::Paren, TokenMode::Code);
                let marker = p.marker();
                collection(p);
                params(p, &marker, true);
                p.end_group();
                p.end(NodeKind::ClosureParams);
                true
            } else {
                false
            };

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

/// Parse an if expresion.
fn if_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::IfExpr, |p| {
        p.eat_assert(&NodeKind::If);

        expr(p)?;
        body(p)?;

        if p.eat_if(&NodeKind::Else) {
            if p.peek() == Some(&NodeKind::If) {
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
        body(p)?;
        Ok(())
    })
}

/// Parse a for expression.
fn for_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ForExpr, |p| {
        p.eat_assert(&NodeKind::For);

        for_pattern(p)?;
        if p.eat_expect(&NodeKind::In) {
            expr(p)?;
            body(p)?;
            Ok(())
        } else {
            Err(())
        }
    })
}

/// Parse a for loop pattern.
fn for_pattern(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ForPattern, |p| {
        ident(p)?;
        if p.peek() == Some(&NodeKind::Comma) {
            p.eat();
            ident(p)?;
        }
        Ok(())
    })
}

/// Parse an import expression.
fn import_expr(p: &mut Parser) -> ParseResult {
    p.start();
    p.eat_assert(&NodeKind::Import);

    if !p.eat_if(&NodeKind::Star) {
        // This is the list of identifiers scenario.
        p.start();
        p.start_group(Group::Imports, TokenMode::Code);
        let marker = p.marker();
        let items = collection(p).1;
        if items == 0 {
            p.expected_at("import items");
        }
        p.end_group();

        marker.filter_children(
            p,
            |n| matches!(n.kind(), NodeKind::Ident(_) | NodeKind::Comma),
            |_| (ErrorPosition::Full, "expected identifier".into()),
        );
        p.end(NodeKind::ImportItems);
    };

    if p.eat_expect(&NodeKind::From) {
        expr(p);
    }

    p.end(NodeKind::ImportExpr);
    Ok(())
}

/// Parse an include expression.
fn include_expr(p: &mut Parser) -> ParseResult {
    p.start();
    p.eat_assert(&NodeKind::Include);

    expr(p);
    p.end(NodeKind::IncludeExpr);
    Ok(())
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
            Err(())
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
            Err(())
        }
    }
}
