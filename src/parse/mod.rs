//! Parsing and tokenization.

mod incremental;
mod parser;
mod resolve;
mod tokens;

pub use incremental::*;
pub use parser::*;
pub use tokens::*;

use std::collections::HashSet;

use crate::syntax::ast::{Associativity, BinOp, UnOp};
use crate::syntax::{NodeKind, SpanPos, SyntaxNode};
use crate::util::EcoString;

/// Parse a source file.
pub fn parse(src: &str) -> SyntaxNode {
    let mut p = Parser::new(src, TokenMode::Markup);
    markup(&mut p, true);
    p.finish().into_iter().next().unwrap()
}

/// Parse code directly, only used for syntax highlighting.
pub fn parse_code(src: &str) -> Vec<SyntaxNode> {
    let mut p = Parser::new(src, TokenMode::Code);
    code(&mut p);
    p.finish()
}

/// Reparse a code block.
///
/// Returns `Some` if all of the input was consumed.
fn reparse_code_block(
    prefix: &str,
    src: &str,
    end_pos: usize,
) -> Option<(Vec<SyntaxNode>, bool, usize)> {
    let mut p = Parser::with_prefix(prefix, src, TokenMode::Code);
    if !p.at(NodeKind::LeftBrace) {
        return None;
    }

    code_block(&mut p);

    let (mut node, terminated) = p.consume()?;
    let first = node.remove(0);
    if first.len() != end_pos {
        return None;
    }

    Some((vec![first], terminated, 1))
}

/// Reparse a content block.
///
/// Returns `Some` if all of the input was consumed.
fn reparse_content_block(
    prefix: &str,
    src: &str,
    end_pos: usize,
) -> Option<(Vec<SyntaxNode>, bool, usize)> {
    let mut p = Parser::with_prefix(prefix, src, TokenMode::Code);
    if !p.at(NodeKind::LeftBracket) {
        return None;
    }

    content_block(&mut p);

    let (mut node, terminated) = p.consume()?;
    let first = node.remove(0);
    if first.len() != end_pos {
        return None;
    }

    Some((vec![first], terminated, 1))
}

/// Reparse a sequence markup elements without the topmost node.
///
/// Returns `Some` if all of the input was consumed.
fn reparse_markup_elements(
    prefix: &str,
    src: &str,
    end_pos: usize,
    differential: isize,
    reference: &[SyntaxNode],
    mut at_start: bool,
    min_indent: usize,
) -> Option<(Vec<SyntaxNode>, bool, usize)> {
    let mut p = Parser::with_prefix(prefix, src, TokenMode::Markup);

    let mut node: Option<&SyntaxNode> = None;
    let mut iter = reference.iter();
    let mut offset = differential;
    let mut replaced = 0;
    let mut stopped = false;

    'outer: while !p.eof() {
        if let Some(NodeKind::Space { newlines: (1 ..) }) = p.peek() {
            if p.column(p.current_end()) < min_indent {
                return None;
            }
        }

        markup_node(&mut p, &mut at_start);

        if p.prev_end() <= end_pos {
            continue;
        }

        let recent = p.marker().before(&p).unwrap();
        let recent_start = p.prev_end() - recent.len();

        while offset <= recent_start as isize {
            if let Some(node) = node {
                // The nodes are equal, at the same position and have the
                // same content. The parsing trees have converged again, so
                // the reparse may stop here.
                if offset == recent_start as isize && node == recent {
                    replaced -= 1;
                    stopped = true;
                    break 'outer;
                }
            }

            if let Some(node) = node {
                offset += node.len() as isize;
            }

            node = iter.next();
            if node.is_none() {
                break;
            }

            replaced += 1;
        }
    }

    if p.eof() && !stopped {
        replaced = reference.len();
    }

    let (mut res, terminated) = p.consume()?;
    if stopped {
        res.pop().unwrap();
    }

    Some((res, terminated, replaced))
}

/// Parse markup.
///
/// If `at_start` is true, things like headings that may only appear at the
/// beginning of a line or content block are initially allowed.
fn markup(p: &mut Parser, mut at_start: bool) {
    p.perform(NodeKind::Markup { min_indent: 0 }, |p| {
        while !p.eof() {
            markup_node(p, &mut at_start);
        }
    });
}

/// Parse a single line of markup.
fn markup_line(p: &mut Parser) {
    markup_indented(p, usize::MAX);
}

/// Parse markup that stays right of the given `column`.
fn markup_indented(p: &mut Parser, min_indent: usize) {
    p.eat_while(|t| match t {
        NodeKind::Space { newlines } => *newlines == 0,
        NodeKind::LineComment | NodeKind::BlockComment => true,
        _ => false,
    });

    let mut at_start = false;
    p.perform(NodeKind::Markup { min_indent }, |p| {
        while !p.eof() {
            if let Some(NodeKind::Space { newlines: (1 ..) }) = p.peek() {
                if p.column(p.current_end()) < min_indent {
                    break;
                }
            }

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
        NodeKind::Space { newlines } => {
            *at_start |= *newlines > 0;
            p.eat();
            return;
        }

        // Comments.
        NodeKind::LineComment | NodeKind::BlockComment => {
            p.eat();
            return;
        }

        // Text and markup.
        NodeKind::Text(_)
        | NodeKind::NonBreakingSpace
        | NodeKind::Shy
        | NodeKind::EnDash
        | NodeKind::EmDash
        | NodeKind::Ellipsis
        | NodeKind::Quote { .. }
        | NodeKind::Linebreak { .. }
        | NodeKind::Raw(_)
        | NodeKind::Math(_)
        | NodeKind::Escape(_) => {
            p.eat();
        }

        // Grouping markup.
        NodeKind::Star => strong(p),
        NodeKind::Underscore => emph(p),
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
        | NodeKind::Include
        | NodeKind::Break
        | NodeKind::Continue
        | NodeKind::Return => markup_expr(p),

        // Code and content block.
        NodeKind::LeftBrace => code_block(p),
        NodeKind::LeftBracket => content_block(p),

        NodeKind::Error(_, _) => p.eat(),
        _ => p.unexpected(),
    };

    *at_start = false;
}

/// Parse strong content.
fn strong(p: &mut Parser) {
    p.perform(NodeKind::Strong, |p| {
        p.start_group(Group::Strong);
        markup(p, false);
        p.end_group();
    })
}

/// Parse emphasized content.
fn emph(p: &mut Parser) {
    p.perform(NodeKind::Emph, |p| {
        p.start_group(Group::Emph);
        markup(p, false);
        p.end_group();
    })
}

/// Parse a heading.
fn heading(p: &mut Parser, at_start: bool) {
    let marker = p.marker();
    let current_start = p.current_start();
    p.assert(NodeKind::Eq);
    while p.eat_if(NodeKind::Eq) {}

    if at_start && p.peek().map_or(true, |kind| kind.is_space()) {
        p.eat_while(|kind| *kind == NodeKind::Space { newlines: 0 });
        markup_line(p);
        marker.end(p, NodeKind::Heading);
    } else {
        let text = p.get(current_start .. p.prev_end()).into();
        marker.convert(p, NodeKind::Text(text));
    }
}

/// Parse a single list item.
fn list_node(p: &mut Parser, at_start: bool) {
    let marker = p.marker();
    let text: EcoString = p.peek_src().into();
    p.assert(NodeKind::Minus);

    let min_indent = p.column(p.prev_end());
    if at_start && p.eat_if(NodeKind::Space { newlines: 0 }) && !p.eof() {
        markup_indented(p, min_indent);
        marker.end(p, NodeKind::List);
    } else {
        marker.convert(p, NodeKind::Text(text));
    }
}

/// Parse a single enum item.
fn enum_node(p: &mut Parser, at_start: bool) {
    let marker = p.marker();
    let text: EcoString = p.peek_src().into();
    p.eat();

    let min_indent = p.column(p.prev_end());
    if at_start && p.eat_if(NodeKind::Space { newlines: 0 }) && !p.eof() {
        markup_indented(p, min_indent);
        marker.end(p, NodeKind::Enum);
    } else {
        marker.convert(p, NodeKind::Text(text));
    }
}

/// Parse an expression within a markup mode.
fn markup_expr(p: &mut Parser) {
    // Does the expression need termination or can content follow directly?
    let stmt = matches!(
        p.peek(),
        Some(
            NodeKind::Let
                | NodeKind::Set
                | NodeKind::Show
                | NodeKind::Wrap
                | NodeKind::Import
                | NodeKind::Include
        )
    );

    p.start_group(Group::Expr);
    let res = expr_prec(p, true, 0);
    if stmt && res.is_ok() && !p.eof() {
        p.expected("semicolon or line break");
    }
    p.end_group();
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
            marker.end(p, NodeKind::UnaryExpr);
        }
        _ => primary(p, atomic)?,
    };

    loop {
        // Parenthesis or bracket means this is a function call.
        if let Some(NodeKind::LeftParen | NodeKind::LeftBracket) = p.peek_direct() {
            marker.perform(p, NodeKind::FuncCall, |p| args(p, true, true))?;
            continue;
        }

        if atomic {
            break;
        }

        // Method call or field access.
        if p.eat_if(NodeKind::Dot) {
            ident(p)?;
            if let Some(NodeKind::LeftParen | NodeKind::LeftBracket) = p.peek_direct() {
                marker.perform(p, NodeKind::MethodCall, |p| args(p, true, true))?;
            } else {
                marker.end(p, NodeKind::FieldAccess);
            }
            continue;
        }

        let op = if p.eat_if(NodeKind::Not) {
            if p.at(NodeKind::In) {
                BinOp::NotIn
            } else {
                p.expected("keyword `in`");
                return Err(ParseError);
            }
        } else {
            match p.peek().and_then(BinOp::from_token) {
                Some(binop) => binop,
                None => break,
            }
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

        marker.perform(p, NodeKind::BinaryExpr, |p| expr_prec(p, atomic, prec))?;
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
            if !atomic && p.at(NodeKind::Arrow) {
                marker.end(p, NodeKind::ClosureParams);
                p.assert(NodeKind::Arrow);
                marker.perform(p, NodeKind::ClosureExpr, expr)
            } else {
                Ok(())
            }
        }

        // Structures.
        Some(NodeKind::LeftParen) => parenthesized(p, atomic),
        Some(NodeKind::LeftBrace) => Ok(code_block(p)),
        Some(NodeKind::LeftBracket) => Ok(content_block(p)),

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
        Some(NodeKind::Break) => break_expr(p),
        Some(NodeKind::Continue) => continue_expr(p),
        Some(NodeKind::Return) => return_expr(p),

        Some(NodeKind::Error(_, _)) => {
            p.eat();
            Err(ParseError)
        }

        // Nothing.
        _ => {
            p.expected_found("expression");
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
            | NodeKind::Numeric(_, _)
            | NodeKind::Str(_),
        ) => {
            p.eat();
            true
        }

        _ => false,
    }
}

/// Parse an identifier.
fn ident(p: &mut Parser) -> ParseResult {
    match p.peek() {
        Some(NodeKind::Ident(_)) => {
            p.eat();
            Ok(())
        }
        _ => {
            p.expected_found("identifier");
            Err(ParseError)
        }
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
    let colon = p.eat_if(NodeKind::Colon);
    let kind = collection(p, true).0;
    p.end_group();

    // Leading colon makes this a dictionary.
    if colon {
        dict(p, marker);
        return Ok(());
    }

    // Arrow means this is a closure's parameter list.
    if !atomic && p.at(NodeKind::Arrow) {
        params(p, marker);
        p.assert(NodeKind::Arrow);
        return marker.perform(p, NodeKind::ClosureExpr, expr);
    }

    // Transform into the identified collection.
    match kind {
        CollectionKind::Group => marker.end(p, NodeKind::GroupExpr),
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
    /// The collection starts with a positional item and has multiple items or a
    /// trailing comma.
    Positional,
    /// The collection starts with a colon or named item.
    Named,
}

/// Parse a collection.
///
/// Returns the length of the collection and whether the literal contained any
/// commas.
fn collection(p: &mut Parser, keyed: bool) -> (CollectionKind, usize) {
    let mut kind = None;
    let mut items = 0;
    let mut can_group = true;
    let mut missing_coma: Option<Marker> = None;

    while !p.eof() {
        if let Ok(item_kind) = item(p, keyed) {
            match item_kind {
                NodeKind::Spread => can_group = false,
                NodeKind::Named if kind.is_none() => {
                    kind = Some(CollectionKind::Named);
                    can_group = false;
                }
                _ if kind.is_none() => {
                    kind = Some(CollectionKind::Positional);
                }
                _ => {}
            }

            items += 1;

            if let Some(marker) = missing_coma.take() {
                p.expected_at(marker, "comma");
            }

            if p.eof() {
                break;
            }

            if p.eat_if(NodeKind::Comma) {
                can_group = false;
            } else {
                missing_coma = Some(p.trivia_start());
            }
        } else {
            p.eat_if(NodeKind::Comma);
            kind = Some(CollectionKind::Group);
        }
    }

    let kind = if can_group && items == 1 {
        CollectionKind::Group
    } else {
        kind.unwrap_or(CollectionKind::Positional)
    };

    (kind, items)
}

/// Parse an expression or a named pair, returning whether it's a spread or a
/// named pair.
fn item(p: &mut Parser, keyed: bool) -> ParseResult<NodeKind> {
    let marker = p.marker();
    if p.eat_if(NodeKind::Dots) {
        marker.perform(p, NodeKind::Spread, expr)?;
        return Ok(NodeKind::Spread);
    }

    expr(p)?;

    if p.at(NodeKind::Colon) {
        match marker.after(p).map(|c| c.kind()) {
            Some(NodeKind::Ident(_)) => {
                p.eat();
                marker.perform(p, NodeKind::Named, expr)?;
            }
            Some(NodeKind::Str(_)) if keyed => {
                p.eat();
                marker.perform(p, NodeKind::Keyed, expr)?;
            }
            kind => {
                let mut msg = EcoString::from("expected identifier");
                if keyed {
                    msg.push_str(" or string");
                }
                if let Some(kind) = kind {
                    msg.push_str(", found ");
                    msg.push_str(kind.as_str());
                }
                let error = NodeKind::Error(SpanPos::Full, msg);
                marker.end(p, error);
                p.eat();
                marker.perform(p, NodeKind::Named, expr).ok();
                return Err(ParseError);
            }
        }

        Ok(NodeKind::Named)
    } else {
        Ok(NodeKind::None)
    }
}

/// Convert a collection into an array, producing errors for anything other than
/// expressions.
fn array(p: &mut Parser, marker: Marker) {
    marker.filter_children(p, |x| match x.kind() {
        NodeKind::Named | NodeKind::Keyed => Err("expected expression"),
        _ => Ok(()),
    });
    marker.end(p, NodeKind::ArrayExpr);
}

/// Convert a collection into a dictionary, producing errors for anything other
/// than named and keyed pairs.
fn dict(p: &mut Parser, marker: Marker) {
    let mut used = HashSet::new();
    marker.filter_children(p, |x| match x.kind() {
        kind if kind.is_paren() => Ok(()),
        NodeKind::Named | NodeKind::Keyed => {
            if let Some(NodeKind::Ident(key) | NodeKind::Str(key)) =
                x.children().next().map(|child| child.kind())
            {
                if !used.insert(key.clone()) {
                    return Err("pair has duplicate key");
                }
            }
            Ok(())
        }
        NodeKind::Spread | NodeKind::Comma | NodeKind::Colon => Ok(()),
        _ => Err("expected named or keyed pair"),
    });
    marker.end(p, NodeKind::DictExpr);
}

/// Convert a collection into a list of parameters, producing errors for
/// anything other than identifiers, spread operations and named pairs.
fn params(p: &mut Parser, marker: Marker) {
    marker.filter_children(p, |x| match x.kind() {
        kind if kind.is_paren() => Ok(()),
        NodeKind::Named | NodeKind::Ident(_) | NodeKind::Comma => Ok(()),
        NodeKind::Spread
            if matches!(
                x.children().last().map(|child| child.kind()),
                Some(&NodeKind::Ident(_))
            ) =>
        {
            Ok(())
        }
        _ => Err("expected identifier, named pair or argument sink"),
    });
    marker.end(p, NodeKind::ClosureParams);
}

/// Parse a code block: `{...}`.
fn code_block(p: &mut Parser) {
    p.perform(NodeKind::CodeBlock, |p| {
        p.start_group(Group::Brace);
        code(p);
        p.end_group();
    });
}

/// Parse expressions.
fn code(p: &mut Parser) {
    while !p.eof() {
        p.start_group(Group::Expr);
        if expr(p).is_ok() && !p.eof() {
            p.expected("semicolon or line break");
        }
        p.end_group();

        // Forcefully skip over newlines since the group's contents can't.
        p.eat_while(NodeKind::is_space);
    }
}

/// Parse a content block: `[...]`.
fn content_block(p: &mut Parser) {
    p.perform(NodeKind::ContentBlock, |p| {
        p.start_group(Group::Bracket);
        markup(p, true);
        p.end_group();
    });
}

/// Parse the arguments to a function call.
fn args(p: &mut Parser, direct: bool, brackets: bool) -> ParseResult {
    match if direct { p.peek_direct() } else { p.peek() } {
        Some(NodeKind::LeftParen) => {}
        Some(NodeKind::LeftBracket) if brackets => {}
        _ => {
            p.expected_found("argument list");
            return Err(ParseError);
        }
    }

    p.perform(NodeKind::CallArgs, |p| {
        if p.at(NodeKind::LeftParen) {
            let marker = p.marker();
            p.start_group(Group::Paren);
            collection(p, false);
            p.end_group();

            let mut used = HashSet::new();
            marker.filter_children(p, |x| match x.kind() {
                NodeKind::Named => {
                    if let Some(NodeKind::Ident(ident)) =
                        x.children().next().map(|child| child.kind())
                    {
                        if !used.insert(ident.clone()) {
                            return Err("duplicate argument");
                        }
                    }
                    Ok(())
                }
                _ => Ok(()),
            });
        }

        while brackets && p.peek_direct() == Some(&NodeKind::LeftBracket) {
            content_block(p);
        }
    });

    Ok(())
}

/// Parse a let expression.
fn let_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::LetExpr, |p| {
        p.assert(NodeKind::Let);

        let marker = p.marker();
        ident(p)?;

        // If a parenthesis follows, this is a function definition.
        let has_params = p.peek_direct() == Some(&NodeKind::LeftParen);
        if has_params {
            let marker = p.marker();
            p.start_group(Group::Paren);
            collection(p, false);
            p.end_group();
            params(p, marker);
        }

        if p.eat_if(NodeKind::Eq) {
            expr(p)?;
        } else if has_params {
            // Function definitions must have a body.
            p.expected("body");
        }

        // Rewrite into a closure expression if it's a function definition.
        if has_params {
            marker.end(p, NodeKind::ClosureExpr);
        }

        Ok(())
    })
}

/// Parse a set expression.
fn set_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::SetExpr, |p| {
        p.assert(NodeKind::Set);
        ident(p)?;
        args(p, true, false)
    })
}

/// Parse a show expression.
fn show_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ShowExpr, |p| {
        p.assert(NodeKind::Show);
        let marker = p.marker();
        expr(p)?;
        if p.eat_if(NodeKind::Colon) {
            marker.filter_children(p, |child| match child.kind() {
                NodeKind::Ident(_) | NodeKind::Colon => Ok(()),
                _ => Err("expected identifier"),
            });
            expr(p)?;
        }
        p.expect(NodeKind::As)?;
        expr(p)
    })
}

/// Parse a wrap expression.
fn wrap_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::WrapExpr, |p| {
        p.assert(NodeKind::Wrap);
        ident(p)?;
        p.expect(NodeKind::In)?;
        expr(p)
    })
}

/// Parse an if-else expresion.
fn if_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::IfExpr, |p| {
        p.assert(NodeKind::If);

        expr(p)?;
        body(p)?;

        if p.eat_if(NodeKind::Else) {
            if p.at(NodeKind::If) {
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
        p.assert(NodeKind::While);
        expr(p)?;
        body(p)
    })
}

/// Parse a for-in expression.
fn for_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ForExpr, |p| {
        p.assert(NodeKind::For);
        for_pattern(p)?;
        p.expect(NodeKind::In)?;
        expr(p)?;
        body(p)
    })
}

/// Parse a for loop pattern.
fn for_pattern(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ForPattern, |p| {
        ident(p)?;
        if p.eat_if(NodeKind::Comma) {
            ident(p)?;
        }
        Ok(())
    })
}

/// Parse an import expression.
fn import_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ImportExpr, |p| {
        p.assert(NodeKind::Import);

        if !p.eat_if(NodeKind::Star) {
            // This is the list of identifiers scenario.
            p.perform(NodeKind::ImportItems, |p| {
                p.start_group(Group::Imports);
                let marker = p.marker();
                let items = collection(p, false).1;
                if items == 0 {
                    p.expected("import items");
                }
                p.end_group();

                marker.filter_children(p, |n| match n.kind() {
                    NodeKind::Ident(_) | NodeKind::Comma => Ok(()),
                    _ => Err("expected identifier"),
                });
            });
        };

        p.expect(NodeKind::From)?;
        expr(p)
    })
}

/// Parse an include expression.
fn include_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::IncludeExpr, |p| {
        p.assert(NodeKind::Include);
        expr(p)
    })
}

/// Parse a break expression.
fn break_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::BreakExpr, |p| {
        p.assert(NodeKind::Break);
        Ok(())
    })
}

/// Parse a continue expression.
fn continue_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ContinueExpr, |p| {
        p.assert(NodeKind::Continue);
        Ok(())
    })
}

/// Parse a return expression.
fn return_expr(p: &mut Parser) -> ParseResult {
    p.perform(NodeKind::ReturnExpr, |p| {
        p.assert(NodeKind::Return);
        if !p.at(NodeKind::Comma) && !p.eof() {
            expr(p)?;
        }
        Ok(())
    })
}

/// Parse a control flow body.
fn body(p: &mut Parser) -> ParseResult {
    match p.peek() {
        Some(NodeKind::LeftBracket) => Ok(content_block(p)),
        Some(NodeKind::LeftBrace) => Ok(code_block(p)),
        _ => {
            p.expected("body");
            Err(ParseError)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    #[track_caller]
    pub fn check<T>(src: &str, found: T, expected: T)
    where
        T: Debug + PartialEq,
    {
        if found != expected {
            println!("source:   {src:?}");
            println!("expected: {expected:#?}");
            println!("found:    {found:#?}");
            panic!("test failed");
        }
    }
}
