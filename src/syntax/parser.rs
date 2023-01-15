use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use std::mem;

use super::ast::{self, Assoc, BinOp, UnOp};
use super::{ErrorPos, LexMode, Lexer, SyntaxKind, SyntaxNode};
use crate::util::{format_eco, EcoString};

/// Parse a source file.
pub fn parse(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, LexMode::Markup);
    markup(&mut p, true);
    p.finish().into_iter().next().unwrap()
}

/// Parse code directly, only used for syntax highlighting.
pub fn parse_code(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, LexMode::Code);
    p.perform(SyntaxKind::CodeBlock, code);
    p.finish().into_iter().next().unwrap()
}

/// Reparse a code block.
///
/// Returns `Some` if all of the input was consumed.
pub(super) fn reparse_code_block(
    prefix: &str,
    text: &str,
    end_pos: usize,
) -> Option<(Vec<SyntaxNode>, bool, usize)> {
    let mut p = Parser::with_prefix(prefix, text, LexMode::Code);
    if !p.at(SyntaxKind::LeftBrace) {
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
pub(super) fn reparse_content_block(
    prefix: &str,
    text: &str,
    end_pos: usize,
) -> Option<(Vec<SyntaxNode>, bool, usize)> {
    let mut p = Parser::with_prefix(prefix, text, LexMode::Code);
    if !p.at(SyntaxKind::LeftBracket) {
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
pub(super) fn reparse_markup_elements(
    prefix: &str,
    text: &str,
    end_pos: usize,
    differential: isize,
    reference: &[SyntaxNode],
    mut at_start: bool,
    min_indent: usize,
) -> Option<(Vec<SyntaxNode>, bool, usize)> {
    let mut p = Parser::with_prefix(prefix, text, LexMode::Markup);

    let mut node: Option<&SyntaxNode> = None;
    let mut iter = reference.iter();
    let mut offset = differential;
    let mut replaced = 0;
    let mut stopped = false;

    'outer: while !p.eof() {
        if let Some(SyntaxKind::Space { newlines: (1..) }) = p.peek() {
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
    p.perform(SyntaxKind::Markup { min_indent: 0 }, |p| {
        while !p.eof() {
            markup_node(p, &mut at_start);
        }
    });
}

/// Parse markup that stays right of the given `column`.
fn markup_indented(p: &mut Parser, min_indent: usize) {
    p.eat_while(|t| match t {
        SyntaxKind::Space { newlines } => newlines == 0,
        SyntaxKind::LineComment | SyntaxKind::BlockComment => true,
        _ => false,
    });

    let marker = p.marker();
    let mut at_start = false;

    while !p.eof() {
        match p.peek() {
            Some(SyntaxKind::Space { newlines: (1..) })
                if p.column(p.current_end()) < min_indent =>
            {
                break;
            }
            _ => {}
        }

        markup_node(p, &mut at_start);
    }

    marker.end(p, SyntaxKind::Markup { min_indent });
}

/// Parse a line of markup that can prematurely end if `f` returns true.
fn markup_line<F>(p: &mut Parser, mut f: F)
where
    F: FnMut(SyntaxKind) -> bool,
{
    p.eat_while(|t| match t {
        SyntaxKind::Space { newlines } => newlines == 0,
        SyntaxKind::LineComment | SyntaxKind::BlockComment => true,
        _ => false,
    });

    p.perform(SyntaxKind::Markup { min_indent: usize::MAX }, |p| {
        let mut at_start = false;
        while let Some(kind) = p.peek() {
            if let SyntaxKind::Space { newlines: (1..) } = kind {
                break;
            }

            if f(kind) {
                break;
            }

            markup_node(p, &mut at_start);
        }
    });
}

fn markup_node(p: &mut Parser, at_start: &mut bool) {
    let Some(token) = p.peek() else { return };
    match token {
        // Whitespace.
        SyntaxKind::Space { newlines } => {
            *at_start |= newlines > 0;
            p.eat();
            return;
        }

        // Comments.
        SyntaxKind::LineComment | SyntaxKind::BlockComment => {
            p.eat();
            return;
        }

        // Text and markup.
        SyntaxKind::Text
        | SyntaxKind::Linebreak
        | SyntaxKind::SmartQuote { .. }
        | SyntaxKind::Escape
        | SyntaxKind::Shorthand
        | SyntaxKind::Symbol
        | SyntaxKind::Link
        | SyntaxKind::Raw { .. }
        | SyntaxKind::Ref => p.eat(),

        // Math.
        SyntaxKind::Dollar => math(p),

        // Strong, emph, heading.
        SyntaxKind::Star => strong(p),
        SyntaxKind::Underscore => emph(p),
        SyntaxKind::Eq => heading(p, *at_start),

        // Lists.
        SyntaxKind::Minus => list_item(p, *at_start),
        SyntaxKind::Plus | SyntaxKind::EnumNumbering => enum_item(p, *at_start),
        SyntaxKind::Slash => {
            term_item(p, *at_start).ok();
        }
        SyntaxKind::Colon => {
            let marker = p.marker();
            p.eat();
            marker.convert(p, SyntaxKind::Text);
        }

        // Hashtag + keyword / identifier.
        SyntaxKind::Ident
        | SyntaxKind::Label
        | SyntaxKind::Let
        | SyntaxKind::Set
        | SyntaxKind::Show
        | SyntaxKind::If
        | SyntaxKind::While
        | SyntaxKind::For
        | SyntaxKind::Import
        | SyntaxKind::Include
        | SyntaxKind::Break
        | SyntaxKind::Continue
        | SyntaxKind::Return => embedded_expr(p),

        // Code and content block.
        SyntaxKind::LeftBrace => code_block(p),
        SyntaxKind::LeftBracket => content_block(p),

        SyntaxKind::Error => p.eat(),
        _ => p.unexpected(),
    };

    *at_start = false;
}

fn strong(p: &mut Parser) {
    p.perform(SyntaxKind::Strong, |p| {
        p.start_group(Group::Strong);
        markup(p, false);
        p.end_group();
    })
}

fn emph(p: &mut Parser) {
    p.perform(SyntaxKind::Emph, |p| {
        p.start_group(Group::Emph);
        markup(p, false);
        p.end_group();
    })
}

fn heading(p: &mut Parser, at_start: bool) {
    let marker = p.marker();
    let mut markers = vec![];
    while p.at(SyntaxKind::Eq) {
        markers.push(p.marker());
        p.eat();
    }

    if at_start && p.peek().map_or(true, |kind| kind.is_space()) {
        p.eat_while(|kind| kind == SyntaxKind::Space { newlines: 0 });
        markup_line(p, |kind| matches!(kind, SyntaxKind::Label));
        marker.end(p, SyntaxKind::Heading);
    } else {
        for marker in markers {
            marker.convert(p, SyntaxKind::Text);
        }
    }
}

fn list_item(p: &mut Parser, at_start: bool) {
    let marker = p.marker();
    p.assert(SyntaxKind::Minus);

    let min_indent = p.column(p.prev_end());
    if at_start && p.eat_if(SyntaxKind::Space { newlines: 0 }) && !p.eof() {
        markup_indented(p, min_indent);
        marker.end(p, SyntaxKind::ListItem);
    } else {
        marker.convert(p, SyntaxKind::Text);
    }
}

fn enum_item(p: &mut Parser, at_start: bool) {
    let marker = p.marker();
    p.eat();

    let min_indent = p.column(p.prev_end());
    if at_start && p.eat_if(SyntaxKind::Space { newlines: 0 }) && !p.eof() {
        markup_indented(p, min_indent);
        marker.end(p, SyntaxKind::EnumItem);
    } else {
        marker.convert(p, SyntaxKind::Text);
    }
}

fn term_item(p: &mut Parser, at_start: bool) -> ParseResult {
    let marker = p.marker();
    p.eat();

    let min_indent = p.column(p.prev_end());
    if at_start && p.eat_if(SyntaxKind::Space { newlines: 0 }) && !p.eof() {
        markup_line(p, |node| matches!(node, SyntaxKind::Colon));
        p.expect(SyntaxKind::Colon)?;
        markup_indented(p, min_indent);
        marker.end(p, SyntaxKind::TermItem);
    } else {
        marker.convert(p, SyntaxKind::Text);
    }

    Ok(())
}

fn embedded_expr(p: &mut Parser) {
    // Does the expression need termination or can content follow directly?
    let stmt = matches!(
        p.peek(),
        Some(
            SyntaxKind::Let
                | SyntaxKind::Set
                | SyntaxKind::Show
                | SyntaxKind::Import
                | SyntaxKind::Include
        )
    );

    p.start_group(Group::Expr);
    let res = expr_prec(p, true, 0);
    if stmt && res.is_ok() && !p.eof() {
        p.expected("semicolon or line break");
    }
    p.end_group();
}

fn math(p: &mut Parser) {
    p.perform(SyntaxKind::Math, |p| {
        p.start_group(Group::Math);
        while !p.eof() {
            math_node(p);
        }
        p.end_group();
    });
}

fn math_node(p: &mut Parser) {
    math_node_prec(p, 0, None)
}

fn math_node_prec(p: &mut Parser, min_prec: usize, stop: Option<SyntaxKind>) {
    let marker = p.marker();
    math_primary(p);

    loop {
        let (kind, mut prec, assoc, stop) = match p.peek() {
            v if v == stop => break,
            Some(SyntaxKind::Underscore) => {
                (SyntaxKind::Script, 2, Assoc::Right, Some(SyntaxKind::Hat))
            }
            Some(SyntaxKind::Hat) => {
                (SyntaxKind::Script, 2, Assoc::Right, Some(SyntaxKind::Underscore))
            }
            Some(SyntaxKind::Slash) => (SyntaxKind::Frac, 1, Assoc::Left, None),
            _ => break,
        };

        if prec < min_prec {
            break;
        }

        match assoc {
            Assoc::Left => prec += 1,
            Assoc::Right => {}
        }

        p.eat();
        math_node_prec(p, prec, stop);

        // Allow up to two different scripts. We do not risk encountering the
        // previous script kind again here due to right-associativity.
        if p.eat_if(SyntaxKind::Underscore) || p.eat_if(SyntaxKind::Hat) {
            math_node_prec(p, prec, None);
        }

        marker.end(p, kind);
    }
}

/// Parse a primary math node.
fn math_primary(p: &mut Parser) {
    let Some(token) = p.peek() else { return };
    match token {
        // Spaces and expressions.
        SyntaxKind::Space { .. }
        | SyntaxKind::Linebreak
        | SyntaxKind::Escape
        | SyntaxKind::Str
        | SyntaxKind::Shorthand
        | SyntaxKind::AlignPoint
        | SyntaxKind::Symbol => p.eat(),

        // Atoms.
        SyntaxKind::Atom => match p.peek_src() {
            "(" => math_group(p, Group::MathRow('(', ')')),
            "{" => math_group(p, Group::MathRow('{', '}')),
            "[" => math_group(p, Group::MathRow('[', ']')),
            _ => p.eat(),
        },

        // Identifiers and math calls.
        SyntaxKind::Ident => {
            let marker = p.marker();
            p.eat();

            // Parenthesis or bracket means this is a function call.
            if matches!(p.peek_direct(), Some(SyntaxKind::Atom) if p.peek_src() == "(") {
                marker.perform(p, SyntaxKind::FuncCall, math_args);
            }
        }

        // Hashtag + keyword / identifier.
        SyntaxKind::Let
        | SyntaxKind::Set
        | SyntaxKind::Show
        | SyntaxKind::If
        | SyntaxKind::While
        | SyntaxKind::For
        | SyntaxKind::Import
        | SyntaxKind::Include
        | SyntaxKind::Break
        | SyntaxKind::Continue
        | SyntaxKind::Return => embedded_expr(p),

        // Code and content block.
        SyntaxKind::LeftBrace => code_block(p),
        SyntaxKind::LeftBracket => content_block(p),

        _ => p.unexpected(),
    }
}

fn math_group(p: &mut Parser, group: Group) {
    p.perform(SyntaxKind::Math, |p| {
        p.start_group(group);
        while !p.eof() {
            math_node(p);
        }
        p.end_group();
    })
}

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
            marker.end(p, SyntaxKind::Unary);
        }
        _ => primary(p, atomic)?,
    };

    loop {
        // Parenthesis or bracket means this is a function call.
        if let Some(SyntaxKind::LeftParen | SyntaxKind::LeftBracket) = p.peek_direct() {
            marker.perform(p, SyntaxKind::FuncCall, args)?;
            continue;
        }

        if atomic {
            break;
        }

        // Method call or field access.
        if p.eat_if(SyntaxKind::Dot) {
            ident(p)?;
            if let Some(SyntaxKind::LeftParen | SyntaxKind::LeftBracket) = p.peek_direct()
            {
                marker.perform(p, SyntaxKind::MethodCall, args)?;
            } else {
                marker.end(p, SyntaxKind::FieldAccess);
            }
            continue;
        }

        let op = if p.eat_if(SyntaxKind::Not) {
            if p.at(SyntaxKind::In) {
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

        match op.assoc() {
            Assoc::Left => prec += 1,
            Assoc::Right => {}
        }

        marker.perform(p, SyntaxKind::Binary, |p| expr_prec(p, atomic, prec))?;
    }

    Ok(())
}

fn primary(p: &mut Parser, atomic: bool) -> ParseResult {
    match p.peek() {
        // Literals and few other things.
        Some(
            SyntaxKind::None
            | SyntaxKind::Auto
            | SyntaxKind::Int
            | SyntaxKind::Float
            | SyntaxKind::Bool
            | SyntaxKind::Numeric
            | SyntaxKind::Str
            | SyntaxKind::Label
            | SyntaxKind::Raw { .. },
        ) => {
            p.eat();
            Ok(())
        }

        // Things that start with an identifier.
        Some(SyntaxKind::Ident) => {
            let marker = p.marker();
            p.eat();

            // Arrow means this is a closure's lone parameter.
            if !atomic && p.at(SyntaxKind::Arrow) {
                marker.end(p, SyntaxKind::Params);
                p.assert(SyntaxKind::Arrow);
                marker.perform(p, SyntaxKind::Closure, expr)
            } else {
                Ok(())
            }
        }

        // Structures.
        Some(SyntaxKind::LeftParen) => parenthesized(p, atomic),
        Some(SyntaxKind::LeftBrace) => Ok(code_block(p)),
        Some(SyntaxKind::LeftBracket) => Ok(content_block(p)),
        Some(SyntaxKind::Dollar) => Ok(math(p)),

        // Keywords.
        Some(SyntaxKind::Let) => let_binding(p),
        Some(SyntaxKind::Set) => set_rule(p),
        Some(SyntaxKind::Show) => show_rule(p),
        Some(SyntaxKind::If) => conditional(p),
        Some(SyntaxKind::While) => while_loop(p),
        Some(SyntaxKind::For) => for_loop(p),
        Some(SyntaxKind::Import) => module_import(p),
        Some(SyntaxKind::Include) => module_include(p),
        Some(SyntaxKind::Break) => break_stmt(p),
        Some(SyntaxKind::Continue) => continue_stmt(p),
        Some(SyntaxKind::Return) => return_stmt(p),

        Some(SyntaxKind::Error) => {
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

fn ident(p: &mut Parser) -> ParseResult {
    match p.peek() {
        Some(SyntaxKind::Ident) => {
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
    let colon = p.eat_if(SyntaxKind::Colon);
    let kind = collection(p, true).0;
    p.end_group();

    // Leading colon makes this a dictionary.
    if colon {
        dict(p, marker);
        return Ok(());
    }

    // Arrow means this is a closure's parameter list.
    if !atomic && p.at(SyntaxKind::Arrow) {
        params(p, marker);
        p.assert(SyntaxKind::Arrow);
        return marker.perform(p, SyntaxKind::Closure, expr);
    }

    // Transform into the identified collection.
    match kind {
        CollectionKind::Group => marker.end(p, SyntaxKind::Parenthesized),
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
    let mut collection_kind = None;
    let mut items = 0;
    let mut can_group = true;
    let mut missing_coma: Option<Marker> = None;

    while !p.eof() {
        let Ok(item_kind) = item(p, keyed) else {
            p.eat_if(SyntaxKind::Comma);
            collection_kind = Some(CollectionKind::Group);
            continue;
        };

        match item_kind {
            SyntaxKind::Spread => can_group = false,
            SyntaxKind::Named if collection_kind.is_none() => {
                collection_kind = Some(CollectionKind::Named);
                can_group = false;
            }
            _ if collection_kind.is_none() => {
                collection_kind = Some(CollectionKind::Positional);
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

        if p.eat_if(SyntaxKind::Comma) {
            can_group = false;
        } else {
            missing_coma = Some(p.trivia_start());
        }
    }

    let kind = if can_group && items == 1 {
        CollectionKind::Group
    } else {
        collection_kind.unwrap_or(CollectionKind::Positional)
    };

    (kind, items)
}

fn item(p: &mut Parser, keyed: bool) -> ParseResult<SyntaxKind> {
    let marker = p.marker();
    if p.eat_if(SyntaxKind::Dots) {
        marker.perform(p, SyntaxKind::Spread, expr)?;
        return Ok(SyntaxKind::Spread);
    }

    expr(p)?;

    if p.at(SyntaxKind::Colon) {
        match marker.after(p).map(|c| c.kind()) {
            Some(SyntaxKind::Ident) => {
                p.eat();
                marker.perform(p, SyntaxKind::Named, expr)?;
            }
            Some(SyntaxKind::Str) if keyed => {
                p.eat();
                marker.perform(p, SyntaxKind::Keyed, expr)?;
            }
            kind => {
                let mut msg = EcoString::from("expected identifier");
                if keyed {
                    msg.push_str(" or string");
                }
                if let Some(kind) = kind {
                    msg.push_str(", found ");
                    msg.push_str(kind.name());
                }
                marker.to_error(p, msg);
                p.eat();
                marker.perform(p, SyntaxKind::Named, expr).ok();
                return Err(ParseError);
            }
        }

        Ok(SyntaxKind::Named)
    } else {
        Ok(SyntaxKind::None)
    }
}

fn array(p: &mut Parser, marker: Marker) {
    marker.filter_children(p, |x| match x.kind() {
        SyntaxKind::Named | SyntaxKind::Keyed => Err("expected expression"),
        _ => Ok(()),
    });
    marker.end(p, SyntaxKind::Array);
}

fn dict(p: &mut Parser, marker: Marker) {
    let mut used = HashSet::new();
    marker.filter_children(p, |x| match x.kind() {
        kind if kind.is_paren() => Ok(()),
        SyntaxKind::Named | SyntaxKind::Keyed => {
            if let Some(child) = x.children().next() {
                let key = match child.cast::<ast::Str>() {
                    Some(str) => str.get(),
                    None => child.text().clone(),
                };

                if !used.insert(key) {
                    return Err("pair has duplicate key");
                }
            }
            Ok(())
        }
        SyntaxKind::Spread | SyntaxKind::Comma | SyntaxKind::Colon => Ok(()),
        _ => Err("expected named or keyed pair"),
    });
    marker.end(p, SyntaxKind::Dict);
}

fn params(p: &mut Parser, marker: Marker) {
    marker.filter_children(p, |x| match x.kind() {
        kind if kind.is_paren() => Ok(()),
        SyntaxKind::Named | SyntaxKind::Ident | SyntaxKind::Comma => Ok(()),
        SyntaxKind::Spread
            if matches!(
                x.children().last().map(|child| child.kind()),
                Some(SyntaxKind::Ident)
            ) =>
        {
            Ok(())
        }
        _ => Err("expected identifier, named pair or argument sink"),
    });
    marker.end(p, SyntaxKind::Params);
}

/// Parse a code block: `{...}`.
fn code_block(p: &mut Parser) {
    p.perform(SyntaxKind::CodeBlock, |p| {
        p.start_group(Group::Brace);
        code(p);
        p.end_group();
    });
}

fn code(p: &mut Parser) {
    while !p.eof() {
        p.start_group(Group::Expr);
        if expr(p).is_ok() && !p.eof() {
            p.expected("semicolon or line break");
        }
        p.end_group();

        // Forcefully skip over newlines since the group's contents can't.
        p.eat_while(SyntaxKind::is_space);
    }
}

fn content_block(p: &mut Parser) {
    p.perform(SyntaxKind::ContentBlock, |p| {
        p.start_group(Group::Bracket);
        markup(p, true);
        p.end_group();
    });
}

fn args(p: &mut Parser) -> ParseResult {
    match p.peek_direct() {
        Some(SyntaxKind::LeftParen) => {}
        Some(SyntaxKind::LeftBracket) => {}
        _ => {
            p.expected_found("argument list");
            return Err(ParseError);
        }
    }

    p.perform(SyntaxKind::Args, |p| {
        if p.at(SyntaxKind::LeftParen) {
            let marker = p.marker();
            p.start_group(Group::Paren);
            collection(p, false);
            p.end_group();

            let mut used = HashSet::new();
            marker.filter_children(p, |x| match x.kind() {
                SyntaxKind::Named => {
                    if let Some(ident) =
                        x.children().next().and_then(|child| child.cast::<ast::Ident>())
                    {
                        if !used.insert(ident.take()) {
                            return Err("duplicate argument");
                        }
                    }
                    Ok(())
                }
                _ => Ok(()),
            });
        }

        while p.peek_direct() == Some(SyntaxKind::LeftBracket) {
            content_block(p);
        }
    });

    Ok(())
}

fn math_args(p: &mut Parser) {
    p.start_group(Group::MathRow('(', ')'));
    p.perform(SyntaxKind::Args, |p| {
        let mut marker = p.marker();
        while !p.eof() {
            if matches!(p.peek(), Some(SyntaxKind::Atom) if p.peek_src() == ",") {
                marker.end(p, SyntaxKind::Math);
                let comma = p.marker();
                p.eat();
                comma.convert(p, SyntaxKind::Comma);
                marker = p.marker();
            } else {
                math_node(p);
            }
        }
        if marker != p.marker() {
            marker.end(p, SyntaxKind::Math);
        }
    });
    p.end_group();
}

fn let_binding(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::LetBinding, |p| {
        p.assert(SyntaxKind::Let);

        let marker = p.marker();
        ident(p)?;

        // If a parenthesis follows, this is a function definition.
        let has_params = p.peek_direct() == Some(SyntaxKind::LeftParen);
        if has_params {
            let marker = p.marker();
            p.start_group(Group::Paren);
            collection(p, false);
            p.end_group();
            params(p, marker);
        }

        if p.eat_if(SyntaxKind::Eq) {
            expr(p)?;
        } else if has_params {
            // Function definitions must have a body.
            p.expected("body");
        }

        // Rewrite into a closure expression if it's a function definition.
        if has_params {
            marker.end(p, SyntaxKind::Closure);
        }

        Ok(())
    })
}

fn set_rule(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::SetRule, |p| {
        p.assert(SyntaxKind::Set);
        ident(p)?;
        args(p)?;
        if p.eat_if(SyntaxKind::If) {
            expr(p)?;
        }
        Ok(())
    })
}

fn show_rule(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::ShowRule, |p| {
        p.assert(SyntaxKind::Show);
        expr(p)?;
        if p.eat_if(SyntaxKind::Colon) {
            expr(p)?;
        }
        Ok(())
    })
}

fn conditional(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::Conditional, |p| {
        p.assert(SyntaxKind::If);

        expr(p)?;
        body(p)?;

        if p.eat_if(SyntaxKind::Else) {
            if p.at(SyntaxKind::If) {
                conditional(p)?;
            } else {
                body(p)?;
            }
        }

        Ok(())
    })
}

fn while_loop(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::WhileLoop, |p| {
        p.assert(SyntaxKind::While);
        expr(p)?;
        body(p)
    })
}

fn for_loop(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::ForLoop, |p| {
        p.assert(SyntaxKind::For);
        for_pattern(p)?;
        p.expect(SyntaxKind::In)?;
        expr(p)?;
        body(p)
    })
}

fn for_pattern(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::ForPattern, |p| {
        ident(p)?;
        if p.eat_if(SyntaxKind::Comma) {
            ident(p)?;
        }
        Ok(())
    })
}

fn module_import(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::ModuleImport, |p| {
        p.assert(SyntaxKind::Import);
        expr(p)?;

        if !p.eat_if(SyntaxKind::Colon) || p.eat_if(SyntaxKind::Star) {
            return Ok(());
        }

        // This is the list of identifiers scenario.
        p.perform(SyntaxKind::ImportItems, |p| {
            let marker = p.marker();
            let items = collection(p, false).1;
            if items == 0 {
                p.expected("import items");
            }
            marker.filter_children(p, |n| match n.kind() {
                SyntaxKind::Ident | SyntaxKind::Comma => Ok(()),
                _ => Err("expected identifier"),
            });
        });

        Ok(())
    })
}

fn module_include(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::ModuleInclude, |p| {
        p.assert(SyntaxKind::Include);
        expr(p)
    })
}

fn break_stmt(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::LoopBreak, |p| {
        p.assert(SyntaxKind::Break);
        Ok(())
    })
}

fn continue_stmt(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::LoopContinue, |p| {
        p.assert(SyntaxKind::Continue);
        Ok(())
    })
}

fn return_stmt(p: &mut Parser) -> ParseResult {
    p.perform(SyntaxKind::FuncReturn, |p| {
        p.assert(SyntaxKind::Return);
        if !p.at(SyntaxKind::Comma) && !p.eof() {
            expr(p)?;
        }
        Ok(())
    })
}

fn body(p: &mut Parser) -> ParseResult {
    match p.peek() {
        Some(SyntaxKind::LeftBracket) => Ok(content_block(p)),
        Some(SyntaxKind::LeftBrace) => Ok(code_block(p)),
        _ => {
            p.expected("body");
            Err(ParseError)
        }
    }
}

/// A convenient token-based parser.
struct Parser<'s> {
    /// An iterator over the source tokens.
    lexer: Lexer<'s>,
    /// Whether we are at the end of the file or of a group.
    eof: bool,
    /// The current token.
    current: Option<SyntaxKind>,
    /// The end byte index of the last non-trivia token.
    prev_end: usize,
    /// The start byte index of the peeked token.
    current_start: usize,
    /// The stack of open groups.
    groups: Vec<GroupEntry>,
    /// The children of the currently built node.
    children: Vec<SyntaxNode>,
    /// Whether the last group was not correctly terminated.
    unterminated_group: bool,
    /// Whether a group terminator was found that did not close a group.
    stray_terminator: bool,
}

impl<'s> Parser<'s> {
    /// Create a new parser for the source string.
    fn new(text: &'s str, mode: LexMode) -> Self {
        Self::with_prefix("", text, mode)
    }

    /// Create a new parser for the source string that is prefixed by some text
    /// that does not need to be parsed but taken into account for column
    /// calculation.
    fn with_prefix(prefix: &str, text: &'s str, mode: LexMode) -> Self {
        let mut lexer = Lexer::with_prefix(prefix, text, mode);
        let current = lexer.next();
        Self {
            lexer,
            eof: current.is_none(),
            current,
            prev_end: 0,
            current_start: 0,
            groups: vec![],
            children: vec![],
            unterminated_group: false,
            stray_terminator: false,
        }
    }

    /// End the parsing process and return the parsed children.
    fn finish(self) -> Vec<SyntaxNode> {
        self.children
    }

    /// End the parsing process and return
    /// - the parsed children and whether the last token was terminated, if all
    ///   groups were terminated correctly, or
    /// - `None` otherwise.
    fn consume(self) -> Option<(Vec<SyntaxNode>, bool)> {
        self.terminated().then(|| (self.children, self.lexer.terminated()))
    }

    /// Create a new marker.
    fn marker(&mut self) -> Marker {
        Marker(self.children.len())
    }

    /// Create a marker right before the trailing trivia.
    fn trivia_start(&self) -> Marker {
        let count = self
            .children
            .iter()
            .rev()
            .take_while(|node| self.is_trivia(node.kind()))
            .count();
        Marker(self.children.len() - count)
    }

    /// Perform a subparse that wraps its result in a node with the given kind.
    fn perform<F, T>(&mut self, kind: SyntaxKind, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let prev = mem::take(&mut self.children);
        let output = f(self);
        let until = self.trivia_start();
        let mut children = mem::replace(&mut self.children, prev);

        if self.lexer.mode() == LexMode::Markup {
            self.children.push(SyntaxNode::inner(kind, children));
        } else {
            // Trailing trivia should not be wrapped into the new node.
            let idx = self.children.len();
            self.children.push(SyntaxNode::default());
            self.children.extend(children.drain(until.0..));
            self.children[idx] = SyntaxNode::inner(kind, children);
        }

        output
    }

    /// Whether the end of the source string or group is reached.
    fn eof(&self) -> bool {
        self.eof
    }

    /// Consume the current token and also trailing trivia.
    fn eat(&mut self) {
        self.stray_terminator |= match self.current {
            Some(SyntaxKind::RightParen) => !self.inside(Group::Paren),
            Some(SyntaxKind::RightBracket) => !self.inside(Group::Bracket),
            Some(SyntaxKind::RightBrace) => !self.inside(Group::Brace),
            _ => false,
        };

        self.prev_end = self.lexer.cursor();
        self.bump();

        if self.lexer.mode() != LexMode::Markup {
            // Skip whitespace and comments.
            while self.current.map_or(false, |kind| self.is_trivia(kind)) {
                self.bump();
            }
        }

        self.repeek();
    }

    /// Consume the current token if it is the given one.
    fn eat_if(&mut self, kind: SyntaxKind) -> bool {
        let at = self.at(kind);
        if at {
            self.eat();
        }
        at
    }

    /// Eat tokens while the condition is true.
    fn eat_while<F>(&mut self, mut f: F)
    where
        F: FnMut(SyntaxKind) -> bool,
    {
        while self.peek().map_or(false, |t| f(t)) {
            self.eat();
        }
    }

    /// Consume the current token if it is the given one and produce an error if
    /// not.
    fn expect(&mut self, kind: SyntaxKind) -> ParseResult {
        let at = self.peek() == Some(kind);
        if at {
            self.eat();
            Ok(())
        } else {
            self.expected(kind.name());
            Err(ParseError)
        }
    }

    /// Consume the current token, debug-asserting that it is the given one.
    #[track_caller]
    fn assert(&mut self, kind: SyntaxKind) {
        debug_assert_eq!(self.peek(), Some(kind));
        self.eat();
    }

    /// Whether the current token is of the given type.
    fn at(&self, kind: SyntaxKind) -> bool {
        self.peek() == Some(kind)
    }

    /// Peek at the current token without consuming it.
    fn peek(&self) -> Option<SyntaxKind> {
        if self.eof {
            None
        } else {
            self.current
        }
    }

    /// Peek at the current token, but only if it follows immediately after the
    /// last one without any trivia in between.
    fn peek_direct(&self) -> Option<SyntaxKind> {
        if self.prev_end() == self.current_start() {
            self.peek()
        } else {
            None
        }
    }

    /// The byte index at which the last non-trivia token ended.
    fn prev_end(&self) -> usize {
        self.prev_end
    }

    /// The byte index at which the current token starts.
    fn current_start(&self) -> usize {
        self.current_start
    }

    /// The byte index at which the current token ends.
    fn current_end(&self) -> usize {
        self.lexer.cursor()
    }

    /// The byte length of the current token.
    fn current_len(&self) -> usize {
        self.current_end() - self.current_start()
    }

    /// The text of the current node.
    fn peek_src(&self) -> &str {
        self.lexer.scanner().from(self.current_start)
    }

    /// Determine the column index for the given byte index.
    fn column(&self, index: usize) -> usize {
        self.lexer.column(index)
    }

    /// Continue parsing in a group.
    ///
    /// When the end delimiter of the group is reached, all subsequent calls to
    /// `peek()` return `None`. Parsing can only continue with a matching call
    /// to `end_group`.
    ///
    /// This panics if the current token does not start the given group.
    #[track_caller]
    fn start_group(&mut self, kind: Group) {
        self.groups.push(GroupEntry { kind, prev_mode: self.lexer.mode() });
        self.lexer.set_mode(match kind {
            Group::Bracket | Group::Strong | Group::Emph => LexMode::Markup,
            Group::Math | Group::MathRow(_, _) => LexMode::Math,
            Group::Brace | Group::Paren | Group::Expr => LexMode::Code,
        });

        match kind {
            Group::Brace => self.assert(SyntaxKind::LeftBrace),
            Group::Bracket => self.assert(SyntaxKind::LeftBracket),
            Group::Paren => self.assert(SyntaxKind::LeftParen),
            Group::Strong => self.assert(SyntaxKind::Star),
            Group::Emph => self.assert(SyntaxKind::Underscore),
            Group::Math => self.assert(SyntaxKind::Dollar),
            Group::MathRow(..) => self.assert(SyntaxKind::Atom),
            Group::Expr => self.repeek(),
        }
    }

    /// End the parsing of a group.
    ///
    /// This panics if no group was started.
    #[track_caller]
    fn end_group(&mut self) {
        let group_mode = self.lexer.mode();
        let group = self.groups.pop().expect("no started group");
        self.lexer.set_mode(group.prev_mode);

        let mut rescan = self.lexer.mode() != group_mode;

        // Eat the end delimiter if there is one.
        if let Some((end, required)) = match group.kind {
            Group::Brace => Some((SyntaxKind::RightBrace, true)),
            Group::Bracket => Some((SyntaxKind::RightBracket, true)),
            Group::Paren => Some((SyntaxKind::RightParen, true)),
            Group::Strong => Some((SyntaxKind::Star, true)),
            Group::Emph => Some((SyntaxKind::Underscore, true)),
            Group::Math => Some((SyntaxKind::Dollar, true)),
            Group::MathRow(..) => Some((SyntaxKind::Atom, true)),
            Group::Expr => Some((SyntaxKind::Semicolon, false)),
        } {
            if self.current.as_ref() == Some(&end) {
                // If another group closes after a group with the missing
                // terminator, its scope of influence ends here and no longer
                // taints the rest of the reparse.
                self.unterminated_group = false;

                // Bump the delimeter and return. No need to rescan in this
                // case. Also, we know that the delimiter is not stray even
                // though we already removed the group.
                let s = self.stray_terminator;
                self.eat();
                self.stray_terminator = s;
                rescan = false;
            } else if required {
                self.expected(end.name());
                self.unterminated_group = true;
            }
        }

        // Rescan the peeked token if the mode changed.
        if rescan {
            let mut target = self.prev_end();
            if group_mode != LexMode::Markup {
                let start = self.trivia_start().0;
                target = self.current_start
                    - self.children[start..].iter().map(SyntaxNode::len).sum::<usize>();
                self.children.truncate(start);
            }

            self.lexer.jump(target);
            self.prev_end = self.lexer.cursor();
            self.current_start = self.lexer.cursor();
            self.current = self.lexer.next();
        }

        self.repeek();
    }

    /// Checks if all groups were correctly terminated.
    fn terminated(&self) -> bool {
        self.groups.is_empty() && !self.unterminated_group && !self.stray_terminator
    }

    /// Low-level bump that consumes exactly one token without special trivia
    /// handling.
    fn bump(&mut self) {
        if let Some((message, pos)) = self.lexer.last_error() {
            let len = self.current_len();
            self.children.push(SyntaxNode::error(message, pos, len))
        } else {
            let kind = self.current.unwrap();
            let text = self.peek_src();
            self.children.push(SyntaxNode::leaf(kind, text));
        }
        self.current_start = self.lexer.cursor();
        self.current = self.lexer.next();
    }

    /// Take another look at the current token to recheck whether it ends a
    /// group.
    fn repeek(&mut self) {
        self.eof = match &self.current {
            Some(SyntaxKind::RightBrace) => self.inside(Group::Brace),
            Some(SyntaxKind::RightBracket) => self.inside(Group::Bracket),
            Some(SyntaxKind::RightParen) => self.inside(Group::Paren),
            Some(SyntaxKind::Star) => self.inside(Group::Strong),
            Some(SyntaxKind::Underscore) => self.inside(Group::Emph),
            Some(SyntaxKind::Dollar) => self
                .groups
                .iter()
                .rev()
                .skip_while(|group| matches!(group.kind, Group::MathRow(..)))
                .next()
                .map_or(false, |group| group.kind == Group::Math),
            Some(SyntaxKind::Semicolon) => self.inside(Group::Expr),
            Some(SyntaxKind::Atom) => match self.peek_src() {
                ")" => self.inside(Group::MathRow('(', ')')),
                "}" => self.inside(Group::MathRow('{', '}')),
                "]" => self.inside(Group::MathRow('[', ']')),
                _ => false,
            },
            Some(SyntaxKind::Space { newlines }) => self.space_ends_group(*newlines),
            Some(_) => false,
            None => true,
        };
    }

    /// Returns whether the given type can be skipped over.
    fn is_trivia(&self, token: SyntaxKind) -> bool {
        match token {
            SyntaxKind::Space { newlines } => !self.space_ends_group(newlines),
            SyntaxKind::LineComment => true,
            SyntaxKind::BlockComment => true,
            _ => false,
        }
    }

    /// Whether a space with the given number of newlines ends the current group.
    fn space_ends_group(&self, n: usize) -> bool {
        if n == 0 {
            return false;
        }

        match self.groups.last().map(|group| group.kind) {
            Some(Group::Strong | Group::Emph) => n >= 2,
            Some(Group::Expr) if n >= 1 => {
                // Allow else and method call to continue on next line.
                self.groups.iter().nth_back(1).map(|group| group.kind)
                    != Some(Group::Brace)
                    || !matches!(
                        self.lexer.clone().next(),
                        Some(SyntaxKind::Else | SyntaxKind::Dot)
                    )
            }
            _ => false,
        }
    }

    /// Whether we are inside the given group (can be nested).
    fn inside(&self, kind: Group) -> bool {
        self.groups
            .iter()
            .rev()
            .take_while(|g| !kind.is_weak() || g.kind.is_weak())
            .any(|g| g.kind == kind)
    }
}

/// Error handling.
impl Parser<'_> {
    /// Eat the current token and add an error that it is unexpected.
    fn unexpected(&mut self) {
        if let Some(found) = self.peek() {
            let marker = self.marker();
            let msg = format_eco!("unexpected {}", found.name());
            self.eat();
            marker.to_error(self, msg);
        }
    }

    /// Add an error that the `thing` was expected at the end of the last
    /// non-trivia token.
    fn expected(&mut self, thing: &str) {
        self.expected_at(self.trivia_start(), thing);
    }

    /// Insert an error message that `what` was expected at the marker position.
    fn expected_at(&mut self, marker: Marker, what: &str) {
        let msg = format_eco!("expected {}", what);
        self.children
            .insert(marker.0, SyntaxNode::error(msg, ErrorPos::Full, 0));
    }

    /// Eat the current token and add an error that it is not the expected
    /// `thing`.
    fn expected_found(&mut self, thing: &str) {
        match self.peek() {
            Some(found) => {
                let marker = self.marker();
                let msg = format_eco!("expected {}, found {}", thing, found.name());
                self.eat();
                marker.to_error(self, msg);
            }
            None => self.expected(thing),
        }
    }
}

/// Marks a location in a parser's child list.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Marker(usize);

impl Marker {
    /// Peek at the child directly before the marker.
    fn before<'a>(self, p: &'a Parser) -> Option<&'a SyntaxNode> {
        p.children.get(self.0.checked_sub(1)?)
    }

    /// Peek at the child directly after the marker.
    fn after<'a>(self, p: &'a Parser) -> Option<&'a SyntaxNode> {
        p.children.get(self.0)
    }

    /// Convert the child directly after marker.
    fn convert(self, p: &mut Parser, kind: SyntaxKind) {
        if let Some(child) = p.children.get_mut(self.0) {
            child.convert_to(kind);
        }
    }

    /// Convert the child directly after marker.
    fn to_error(self, p: &mut Parser, message: impl Into<EcoString>) {
        if let Some(child) = p.children.get_mut(self.0) {
            child.convert_to_error(message);
        }
    }

    /// Perform a subparse that wraps all children after the marker in a node
    /// with the given kind.
    fn perform<T, F>(self, p: &mut Parser, kind: SyntaxKind, f: F) -> T
    where
        F: FnOnce(&mut Parser) -> T,
    {
        let success = f(p);
        self.end(p, kind);
        success
    }

    /// Wrap all children after the marker (excluding trailing trivia) in a node
    /// with the given `kind`.
    fn end(self, p: &mut Parser, kind: SyntaxKind) {
        let until = p.trivia_start().0.max(self.0);
        let children = p.children.drain(self.0..until).collect();
        p.children.insert(self.0, SyntaxNode::inner(kind, children));
    }

    /// Wrap all children that do not fulfill the predicate in error nodes.
    fn filter_children<F>(self, p: &mut Parser, mut f: F)
    where
        F: FnMut(&SyntaxNode) -> Result<(), &'static str>,
    {
        for child in &mut p.children[self.0..] {
            // Don't expose errors.
            if child.kind().is_error() {
                continue;
            }

            // Don't expose trivia in code.
            if p.lexer.mode() != LexMode::Markup && child.kind().is_trivia() {
                continue;
            }

            if let Err(msg) = f(child) {
                let mut msg = EcoString::from(msg);
                if msg.starts_with("expected") {
                    msg.push_str(", found ");
                    msg.push_str(child.kind().name());
                }
                let len = child.len();
                *child = SyntaxNode::error(msg, ErrorPos::Full, len);
            }
        }
    }
}

/// A logical group of tokens, e.g. `[...]`.
#[derive(Debug)]
struct GroupEntry {
    /// The kind of group this is. This decides which token(s) will end the
    /// group. For example, a [`Group::Paren`] will be ended by
    /// [`Token::RightParen`].
    kind: Group,
    /// The mode the parser was in _before_ the group started (to which we go
    /// back once the group ends).
    prev_mode: LexMode,
}

/// A group, confined by optional start and end delimiters.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Group {
    /// A curly-braced group: `{...}`.
    Brace,
    /// A bracketed group: `[...]`.
    Bracket,
    /// A parenthesized group: `(...)`.
    Paren,
    /// A group surrounded with stars: `*...*`.
    Strong,
    /// A group surrounded with underscore: `_..._`.
    Emph,
    /// A group surrounded by dollar signs: `$...$`.
    Math,
    /// A group surrounded by math delimiters.
    MathRow(char, char),
    /// A group ended by a semicolon or a line break: `;`, `\n`.
    Expr,
}

impl Group {
    /// Whether the group can only force other weak groups to end.
    fn is_weak(self) -> bool {
        matches!(self, Group::Strong | Group::Emph)
    }
}

/// Allows parser methods to use the try operator. Never returned top-level
/// because the parser recovers from all errors.
type ParseResult<T = ()> = Result<T, ParseError>;

/// The error type for parsing.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct ParseError;

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("failed to parse")
    }
}

impl std::error::Error for ParseError {}
