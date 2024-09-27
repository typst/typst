use std::collections::{HashMap, HashSet};
use std::mem;
use std::ops::{Index, IndexMut, Range};

use ecow::{eco_format, EcoString};
use unicode_math_class::MathClass;

use crate::set::SyntaxSet;
use crate::{
    ast, is_ident, is_newline, set, LexMode, Lexer, SyntaxError, SyntaxKind, SyntaxNode,
};

/// Parses a source file.
pub fn parse(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, 0, LexMode::Markup);
    markup(&mut p, true, 0, |_| false);
    p.finish().into_iter().next().unwrap()
}

/// Parses top-level code.
pub fn parse_code(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, 0, LexMode::Code);
    let m = p.marker();
    p.skip();
    code_exprs(&mut p, |_| false);
    p.wrap_all(m, SyntaxKind::Code);
    p.finish().into_iter().next().unwrap()
}

/// Parses top-level math.
pub fn parse_math(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, 0, LexMode::Math);
    math(&mut p, |_| false);
    p.finish().into_iter().next().unwrap()
}

/// Parses the contents of a file or content block.
fn markup(
    p: &mut Parser,
    mut at_start: bool,
    min_indent: usize,
    mut stop: impl FnMut(&Parser) -> bool,
) {
    let m = p.marker();
    let mut nesting: usize = 0;
    while !p.end() {
        match p.current() {
            SyntaxKind::LeftBracket => nesting += 1,
            SyntaxKind::RightBracket if nesting > 0 => nesting -= 1,
            _ if stop(p) => break,
            _ => {}
        }

        if p.newline() {
            at_start = true;
            if min_indent > 0 && p.column(p.current_end()) < min_indent {
                break;
            }
            p.eat();
            continue;
        }

        if p.at_set(set::MARKUP_EXPR) {
            markup_expr(p, &mut at_start);
        } else {
            p.unexpected();
        }
    }
    p.wrap(m, SyntaxKind::Markup);
}

/// Reparses a subsection of markup incrementally.
pub(super) fn reparse_markup(
    text: &str,
    range: Range<usize>,
    at_start: &mut bool,
    nesting: &mut usize,
    mut stop: impl FnMut(SyntaxKind) -> bool,
) -> Option<Vec<SyntaxNode>> {
    let mut p = Parser::new(text, range.start, LexMode::Markup);
    while !p.end() && p.current_start() < range.end {
        match p.current() {
            SyntaxKind::LeftBracket => *nesting += 1,
            SyntaxKind::RightBracket if *nesting > 0 => *nesting -= 1,
            _ if stop(p.current()) => break,
            _ => {}
        }

        if p.newline() {
            *at_start = true;
            p.eat();
            continue;
        }

        if p.at_set(set::MARKUP_EXPR) {
            markup_expr(&mut p, at_start);
        } else {
            p.unexpected();
        }
    }
    (p.balanced && p.current_start() == range.end).then(|| p.finish())
}

/// Parses a single markup expression: This includes markup elements like
/// spaces, text, and headings, and embedded code expressions.
fn markup_expr(p: &mut Parser, at_start: &mut bool) {
    match p.current() {
        SyntaxKind::Space
        | SyntaxKind::Parbreak
        | SyntaxKind::LineComment
        | SyntaxKind::BlockComment => {
            p.eat();
            return;
        }

        SyntaxKind::Text
        | SyntaxKind::Linebreak
        | SyntaxKind::Escape
        | SyntaxKind::Shorthand
        | SyntaxKind::SmartQuote
        | SyntaxKind::Link
        | SyntaxKind::Label => p.eat(),

        SyntaxKind::Hash => embedded_code_expr(p),
        SyntaxKind::Star => strong(p),
        SyntaxKind::Underscore => emph(p),
        SyntaxKind::RawDelim => raw(p),
        SyntaxKind::HeadingMarker if *at_start => heading(p),
        SyntaxKind::ListMarker if *at_start => list_item(p),
        SyntaxKind::EnumMarker if *at_start => enum_item(p),
        SyntaxKind::TermMarker if *at_start => term_item(p),
        SyntaxKind::RefMarker => reference(p),
        SyntaxKind::Dollar => equation(p),

        SyntaxKind::LeftBracket
        | SyntaxKind::RightBracket
        | SyntaxKind::HeadingMarker
        | SyntaxKind::ListMarker
        | SyntaxKind::EnumMarker
        | SyntaxKind::TermMarker
        | SyntaxKind::Colon => p.convert(SyntaxKind::Text),

        _ => {}
    }

    *at_start = false;
}

/// Parses strong content: `*Strong*`.
fn strong(p: &mut Parser) {
    const END: SyntaxSet = SyntaxSet::new()
        .add(SyntaxKind::Star)
        .add(SyntaxKind::Parbreak)
        .add(SyntaxKind::RightBracket);

    let m = p.marker();
    p.assert(SyntaxKind::Star);
    markup(p, false, 0, |p| p.at_set(END));
    p.expect_closing_delimiter(m, SyntaxKind::Star);
    p.wrap(m, SyntaxKind::Strong);
}

/// Parses emphasized content: `_Emphasized_`.
fn emph(p: &mut Parser) {
    const END: SyntaxSet = SyntaxSet::new()
        .add(SyntaxKind::Underscore)
        .add(SyntaxKind::Parbreak)
        .add(SyntaxKind::RightBracket);

    let m = p.marker();
    p.assert(SyntaxKind::Underscore);
    markup(p, false, 0, |p| p.at_set(END));
    p.expect_closing_delimiter(m, SyntaxKind::Underscore);
    p.wrap(m, SyntaxKind::Emph);
}

/// Parses raw text with optional syntax highlighting: `` `...` ``.
fn raw(p: &mut Parser) {
    let m = p.marker();
    p.enter(LexMode::Raw);
    p.assert(SyntaxKind::RawDelim);

    // Eats until the closing delimiter.
    while !p.end() && !p.at(SyntaxKind::RawDelim) {
        p.eat();
    }

    p.expect(SyntaxKind::RawDelim);
    p.exit();
    p.wrap(m, SyntaxKind::Raw);
}

/// Parses a section heading: `= Introduction`.
fn heading(p: &mut Parser) {
    const END: SyntaxSet = SyntaxSet::new()
        .add(SyntaxKind::Label)
        .add(SyntaxKind::RightBracket)
        .add(SyntaxKind::Space);

    let m = p.marker();
    p.assert(SyntaxKind::HeadingMarker);
    whitespace_line(p);
    markup(p, false, usize::MAX, |p| {
        p.at_set(END)
            && (!p.at(SyntaxKind::Space) || p.lexer.clone().next() == SyntaxKind::Label)
    });
    p.wrap(m, SyntaxKind::Heading);
}

/// Parses an item in a bullet list: `- ...`.
fn list_item(p: &mut Parser) {
    let m = p.marker();
    let min_indent = p.column(p.current_start()) + 1;
    p.assert(SyntaxKind::ListMarker);
    whitespace_line(p);
    markup(p, false, min_indent, |p| p.at(SyntaxKind::RightBracket));
    p.wrap(m, SyntaxKind::ListItem);
}

/// Parses an item in an enumeration (numbered list): `+ ...` or `1. ...`.
fn enum_item(p: &mut Parser) {
    let m = p.marker();
    let min_indent = p.column(p.current_start()) + 1;
    p.assert(SyntaxKind::EnumMarker);
    whitespace_line(p);
    markup(p, false, min_indent, |p| p.at(SyntaxKind::RightBracket));
    p.wrap(m, SyntaxKind::EnumItem);
}

/// Parses an item in a term list: `/ Term: Details`.
fn term_item(p: &mut Parser) {
    const TERM_END: SyntaxSet =
        SyntaxSet::new().add(SyntaxKind::Colon).add(SyntaxKind::RightBracket);

    let m = p.marker();
    p.assert(SyntaxKind::TermMarker);
    let min_indent = p.column(p.prev_end());
    whitespace_line(p);
    markup(p, false, usize::MAX, |p| p.at_set(TERM_END));
    p.expect(SyntaxKind::Colon);
    whitespace_line(p);
    markup(p, false, min_indent, |p| p.at(SyntaxKind::RightBracket));
    p.wrap(m, SyntaxKind::TermItem);
}

/// Parses a reference: `@target`, `@target[..]`.
fn reference(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::RefMarker);
    if p.directly_at(SyntaxKind::LeftBracket) {
        content_block(p);
    }
    p.wrap(m, SyntaxKind::Ref);
}

/// Consumes whitespace that does not contain a newline.
fn whitespace_line(p: &mut Parser) {
    while !p.newline() && p.current().is_trivia() {
        p.eat();
    }
}

/// Parses a mathematical equation: `$x$`, `$ x^2 $`.
fn equation(p: &mut Parser) {
    let m = p.marker();
    p.enter(LexMode::Math);
    p.assert(SyntaxKind::Dollar);
    math(p, |p| p.at(SyntaxKind::Dollar));
    p.expect_closing_delimiter(m, SyntaxKind::Dollar);
    p.exit();
    p.wrap(m, SyntaxKind::Equation);
}

/// Parses the contents of a mathematical equation: `x^2 + 1`.
fn math(p: &mut Parser, mut stop: impl FnMut(&Parser) -> bool) {
    let m = p.marker();
    while !p.end() && !stop(p) {
        if p.at_set(set::MATH_EXPR) {
            math_expr(p);
        } else {
            p.unexpected();
        }
    }
    p.wrap(m, SyntaxKind::Math);
}

/// Parses a single math expression: This includes math elements like
/// attachment, fractions, and roots, and embedded code expressions.
fn math_expr(p: &mut Parser) {
    math_expr_prec(p, 0, SyntaxKind::End)
}

/// Parses a math expression with at least the given precedence.
fn math_expr_prec(p: &mut Parser, min_prec: usize, stop: SyntaxKind) {
    let m = p.marker();
    let mut continuable = false;
    match p.current() {
        SyntaxKind::Hash => embedded_code_expr(p),
        SyntaxKind::MathIdent => {
            continuable = true;
            p.eat();
            while p.directly_at(SyntaxKind::Text) && p.current_text() == "." && {
                let mut copy = p.lexer.clone();
                let start = copy.cursor();
                let next = copy.next();
                let end = copy.cursor();
                matches!(next, SyntaxKind::MathIdent | SyntaxKind::Text)
                    && is_ident(&p.text[start..end])
            } {
                p.convert(SyntaxKind::Dot);
                p.convert(SyntaxKind::Ident);
                p.wrap(m, SyntaxKind::FieldAccess);
            }
            if min_prec < 3 && p.directly_at(SyntaxKind::Text) && p.current_text() == "("
            {
                math_args(p);
                p.wrap(m, SyntaxKind::FuncCall);
                continuable = false;
            }
        }

        SyntaxKind::Text | SyntaxKind::MathShorthand => {
            continuable = matches!(
                math_class(p.current_text()),
                None | Some(MathClass::Alphabetic)
            );
            if !maybe_delimited(p) {
                p.eat();
            }
        }

        SyntaxKind::Linebreak | SyntaxKind::MathAlignPoint => p.eat(),
        SyntaxKind::Escape | SyntaxKind::Str => {
            continuable = true;
            p.eat();
        }

        SyntaxKind::Root => {
            if min_prec < 3 {
                p.eat();
                let m2 = p.marker();
                math_expr_prec(p, 2, stop);
                math_unparen(p, m2);
                p.wrap(m, SyntaxKind::MathRoot);
            }
        }

        SyntaxKind::Prime => {
            // Means that there is nothing to attach the prime to.
            continuable = true;
            while p.at(SyntaxKind::Prime) {
                let m2 = p.marker();
                p.eat();
                // Eat the group until the space.
                while p.eat_if_direct(SyntaxKind::Prime) {}
                p.wrap(m2, SyntaxKind::MathPrimes);
            }
        }

        _ => p.expected("expression"),
    }

    if continuable
        && min_prec < 3
        && p.prev_end() == p.current_start()
        && maybe_delimited(p)
    {
        p.wrap(m, SyntaxKind::Math);
    }

    // Whether there were _any_ primes in the loop.
    let mut primed = false;

    while !p.end() && !p.at(stop) {
        if p.directly_at(SyntaxKind::Text) && p.current_text() == "!" {
            p.eat();
            p.wrap(m, SyntaxKind::Math);
            continue;
        }

        let prime_marker = p.marker();
        if p.eat_if_direct(SyntaxKind::Prime) {
            // Eat as many primes as possible.
            while p.eat_if_direct(SyntaxKind::Prime) {}
            p.wrap(prime_marker, SyntaxKind::MathPrimes);

            // Will not be continued, so need to wrap the prime as attachment.
            if p.at(stop) {
                p.wrap(m, SyntaxKind::MathAttach);
            }

            primed = true;
            continue;
        }

        let Some((kind, stop, assoc, mut prec)) = math_op(p.current()) else {
            // No attachments, so we need to wrap primes as attachment.
            if primed {
                p.wrap(m, SyntaxKind::MathAttach);
            }

            break;
        };

        if primed && kind == SyntaxKind::MathFrac {
            p.wrap(m, SyntaxKind::MathAttach);
        }

        if prec < min_prec {
            break;
        }

        match assoc {
            ast::Assoc::Left => prec += 1,
            ast::Assoc::Right => {}
        }

        if kind == SyntaxKind::MathFrac {
            math_unparen(p, m);
        }

        p.eat();
        let m2 = p.marker();
        math_expr_prec(p, prec, stop);
        math_unparen(p, m2);

        if p.eat_if(SyntaxKind::Underscore) || p.eat_if(SyntaxKind::Hat) {
            let m3 = p.marker();
            math_expr_prec(p, prec, SyntaxKind::End);
            math_unparen(p, m3);
        }

        p.wrap(m, kind);
    }
}

fn maybe_delimited(p: &mut Parser) -> bool {
    let open = math_class(p.current_text()) == Some(MathClass::Opening);
    if open {
        math_delimited(p);
    }
    open
}

fn math_delimited(p: &mut Parser) {
    let m = p.marker();
    p.eat();
    let m2 = p.marker();
    while !p.end() && !p.at(SyntaxKind::Dollar) {
        if math_class(p.current_text()) == Some(MathClass::Closing) {
            p.wrap(m2, SyntaxKind::Math);
            p.eat();
            p.wrap(m, SyntaxKind::MathDelimited);
            return;
        }

        if p.at_set(set::MATH_EXPR) {
            math_expr(p);
        } else {
            p.unexpected();
        }
    }

    p.wrap(m, SyntaxKind::Math);
}

fn math_unparen(p: &mut Parser, m: Marker) {
    let Some(node) = p.nodes.get_mut(m.0) else { return };
    if node.kind() != SyntaxKind::MathDelimited {
        return;
    }

    if let [first, .., last] = node.children_mut() {
        if first.text() == "(" && last.text() == ")" {
            first.convert_to_kind(SyntaxKind::LeftParen);
            last.convert_to_kind(SyntaxKind::RightParen);
        }
    }

    node.convert_to_kind(SyntaxKind::Math);
}

fn math_class(text: &str) -> Option<MathClass> {
    match text {
        "[|" => return Some(MathClass::Opening),
        "|]" => return Some(MathClass::Closing),
        "||" => return Some(MathClass::Fence),
        _ => {}
    }

    let mut chars = text.chars();
    chars
        .next()
        .filter(|_| chars.next().is_none())
        .and_then(unicode_math_class::class)
}

fn math_op(kind: SyntaxKind) -> Option<(SyntaxKind, SyntaxKind, ast::Assoc, usize)> {
    match kind {
        SyntaxKind::Underscore => {
            Some((SyntaxKind::MathAttach, SyntaxKind::Hat, ast::Assoc::Right, 2))
        }
        SyntaxKind::Hat => {
            Some((SyntaxKind::MathAttach, SyntaxKind::Underscore, ast::Assoc::Right, 2))
        }
        SyntaxKind::Slash => {
            Some((SyntaxKind::MathFrac, SyntaxKind::End, ast::Assoc::Left, 1))
        }
        _ => None,
    }
}

fn math_args(p: &mut Parser) {
    let m = p.marker();
    p.convert(SyntaxKind::LeftParen);

    let mut namable = true;
    let mut named = None;
    let mut has_arrays = false;
    let mut array = p.marker();
    let mut arg = p.marker();

    while !p.end() && !p.at(SyntaxKind::Dollar) {
        if namable
            && (p.at(SyntaxKind::MathIdent) || p.at(SyntaxKind::Text))
            && p.text[p.current_end()..].starts_with(':')
        {
            p.convert(SyntaxKind::Ident);
            p.convert(SyntaxKind::Colon);
            named = Some(arg);
            arg = p.marker();
            array = p.marker();
        }

        match p.current_text() {
            ")" => break,
            ";" => {
                maybe_wrap_in_math(p, arg, named);
                p.wrap(array, SyntaxKind::Array);
                p.convert(SyntaxKind::Semicolon);
                array = p.marker();
                arg = p.marker();
                namable = true;
                named = None;
                has_arrays = true;
                continue;
            }
            "," => {
                maybe_wrap_in_math(p, arg, named);
                p.convert(SyntaxKind::Comma);
                arg = p.marker();
                namable = true;
                if named.is_some() {
                    array = p.marker();
                    named = None;
                }
                continue;
            }
            _ => {}
        }

        if p.at_set(set::MATH_EXPR) {
            math_expr(p);
        } else {
            p.unexpected();
        }

        namable = false;
    }

    if arg != p.marker() {
        maybe_wrap_in_math(p, arg, named);
        if named.is_some() {
            array = p.marker();
        }
    }

    if has_arrays && array != p.marker() {
        p.wrap(array, SyntaxKind::Array);
    }

    if p.at(SyntaxKind::Text) && p.current_text() == ")" {
        p.convert(SyntaxKind::RightParen);
    } else {
        p.expected("closing paren");
        p.balanced = false;
    }

    p.wrap(m, SyntaxKind::Args);
}

/// Wrap math function arguments in a "Math" SyntaxKind to combine adjacent expressions
/// or create blank content.
///
/// We don't wrap when `exprs == 1`, as there is only one expression, so the grouping
/// isn't needed, and this would change the type of the expression from potentially
/// non-content to content.
///
/// Note that `exprs` might be 0 if we have whitespace or trivia before a comma i.e.
/// `mat(; ,)` or `sin(x, , , ,)`. This would create an empty Math element before that
/// trivia if we called `p.wrap()` -- breaking the expected AST for 2-d arguments -- so
/// we instead manually wrap to our current marker using `p.wrap_within()`.
fn maybe_wrap_in_math(p: &mut Parser, arg: Marker, named: Option<Marker>) {
    let exprs = p.post_process(arg).filter(|node| node.is::<ast::Expr>()).count();
    if exprs != 1 {
        // Convert 0 exprs into a blank math element (so empty arguments are allowed).
        // Convert 2+ exprs into a math element (so they become a joined sequence).
        p.wrap_within(arg, p.marker(), SyntaxKind::Math);
    }

    if let Some(m) = named {
        p.wrap(m, SyntaxKind::Named);
    }
}

/// Parses the contents of a code block.
fn code(p: &mut Parser, stop: impl FnMut(&Parser) -> bool) {
    let m = p.marker();
    code_exprs(p, stop);
    p.wrap(m, SyntaxKind::Code);
}

/// Parses a sequence of code expressions.
fn code_exprs(p: &mut Parser, mut stop: impl FnMut(&Parser) -> bool) {
    while !p.end() && !stop(p) {
        p.enter_newline_mode(NewlineMode::Contextual);

        let at_expr = p.at_set(set::CODE_EXPR);
        if at_expr {
            code_expr(p);
            if !p.end() && !stop(p) && !p.eat_if(SyntaxKind::Semicolon) {
                p.expected("semicolon or line break");
                if p.at(SyntaxKind::Label) {
                    p.hint("labels can only be applied in markup mode");
                    p.hint("try wrapping your code in a markup block (`[ ]`)");
                }
            }
        }

        p.exit_newline_mode();
        if !at_expr && !p.end() {
            p.unexpected();
        }
    }
}

/// Parses a single code expression.
fn code_expr(p: &mut Parser) {
    code_expr_prec(p, false, 0)
}

/// Parses a code expression embedded in markup or math.
fn embedded_code_expr(p: &mut Parser) {
    p.enter_newline_mode(NewlineMode::Stop);
    p.enter(LexMode::Code);
    p.assert(SyntaxKind::Hash);
    p.unskip();

    let stmt = p.at_set(set::STMT);
    let at = p.at_set(set::ATOMIC_CODE_EXPR);
    code_expr_prec(p, true, 0);

    // Consume error for things like `#12p` or `#"abc\"`.#
    if !at && !p.current().is_trivia() && !p.end() {
        p.unexpected();
    }

    let semi =
        (stmt || p.directly_at(SyntaxKind::Semicolon)) && p.eat_if(SyntaxKind::Semicolon);

    if stmt && !semi && !p.end() && !p.at(SyntaxKind::RightBracket) {
        p.expected("semicolon or line break");
    }

    p.exit();
    p.exit_newline_mode();
}

/// Parses a code expression with at least the given precedence.
fn code_expr_prec(p: &mut Parser, atomic: bool, min_prec: usize) {
    let m = p.marker();
    if !atomic && p.at_set(set::UNARY_OP) {
        let op = ast::UnOp::from_kind(p.current()).unwrap();
        p.eat();
        code_expr_prec(p, atomic, op.precedence());
        p.wrap(m, SyntaxKind::Unary);
    } else {
        code_primary(p, atomic);
    }

    loop {
        if p.directly_at(SyntaxKind::LeftParen) || p.directly_at(SyntaxKind::LeftBracket)
        {
            args(p);
            p.wrap(m, SyntaxKind::FuncCall);
            continue;
        }

        let at_field_or_method =
            p.directly_at(SyntaxKind::Dot) && p.lexer.clone().next() == SyntaxKind::Ident;

        if atomic && !at_field_or_method {
            break;
        }

        if p.eat_if(SyntaxKind::Dot) {
            p.expect(SyntaxKind::Ident);
            p.wrap(m, SyntaxKind::FieldAccess);
            continue;
        }

        let binop = if p.at_set(set::BINARY_OP) {
            ast::BinOp::from_kind(p.current())
        } else if min_prec <= ast::BinOp::NotIn.precedence() && p.eat_if(SyntaxKind::Not)
        {
            if p.at(SyntaxKind::In) {
                Some(ast::BinOp::NotIn)
            } else {
                p.expected("keyword `in`");
                break;
            }
        } else {
            None
        };

        if let Some(op) = binop {
            let mut prec = op.precedence();
            if prec < min_prec {
                break;
            }

            match op.assoc() {
                ast::Assoc::Left => prec += 1,
                ast::Assoc::Right => {}
            }

            p.eat();
            code_expr_prec(p, false, prec);
            p.wrap(m, SyntaxKind::Binary);
            continue;
        }

        break;
    }
}

/// Parses an primary in a code expression. These are the atoms that unary and
/// binary operations, functions calls, and field accesses start with / are
/// composed of.
fn code_primary(p: &mut Parser, atomic: bool) {
    let m = p.marker();
    match p.current() {
        SyntaxKind::Ident => {
            p.eat();
            if !atomic && p.at(SyntaxKind::Arrow) {
                p.wrap(m, SyntaxKind::Params);
                p.assert(SyntaxKind::Arrow);
                code_expr(p);
                p.wrap(m, SyntaxKind::Closure);
            }
        }
        SyntaxKind::Underscore if !atomic => {
            p.eat();
            if p.at(SyntaxKind::Arrow) {
                p.wrap(m, SyntaxKind::Params);
                p.eat();
                code_expr(p);
                p.wrap(m, SyntaxKind::Closure);
            } else if p.eat_if(SyntaxKind::Eq) {
                code_expr(p);
                p.wrap(m, SyntaxKind::DestructAssignment);
            } else {
                p[m].expected("expression");
            }
        }

        SyntaxKind::LeftBrace => code_block(p),
        SyntaxKind::LeftBracket => content_block(p),
        SyntaxKind::LeftParen => expr_with_paren(p, atomic),
        SyntaxKind::RawDelim => raw(p),
        SyntaxKind::Dollar => equation(p),
        SyntaxKind::Let => let_binding(p),
        SyntaxKind::Set => set_rule(p),
        SyntaxKind::Show => show_rule(p),
        SyntaxKind::Context => contextual(p, atomic),
        SyntaxKind::If => conditional(p),
        SyntaxKind::While => while_loop(p),
        SyntaxKind::For => for_loop(p),
        SyntaxKind::Import => module_import(p),
        SyntaxKind::Include => module_include(p),
        SyntaxKind::Break => break_stmt(p),
        SyntaxKind::Continue => continue_stmt(p),
        SyntaxKind::Return => return_stmt(p),

        SyntaxKind::None
        | SyntaxKind::Auto
        | SyntaxKind::Int
        | SyntaxKind::Float
        | SyntaxKind::Bool
        | SyntaxKind::Numeric
        | SyntaxKind::Str
        | SyntaxKind::Label => p.eat(),

        _ => p.expected("expression"),
    }
}

/// Parses a content or code block.
fn block(p: &mut Parser) {
    match p.current() {
        SyntaxKind::LeftBracket => content_block(p),
        SyntaxKind::LeftBrace => code_block(p),
        _ => p.expected("block"),
    }
}

/// Reparses a full content or code block.
pub(super) fn reparse_block(text: &str, range: Range<usize>) -> Option<SyntaxNode> {
    let mut p = Parser::new(text, range.start, LexMode::Code);
    assert!(p.at(SyntaxKind::LeftBracket) || p.at(SyntaxKind::LeftBrace));
    block(&mut p);
    (p.balanced && p.prev_end() == range.end)
        .then(|| p.finish().into_iter().next().unwrap())
}

/// Parses a code block: `{ let x = 1; x + 2 }`.
fn code_block(p: &mut Parser) {
    const END: SyntaxSet = SyntaxSet::new()
        .add(SyntaxKind::RightBrace)
        .add(SyntaxKind::RightBracket)
        .add(SyntaxKind::RightParen);

    let m = p.marker();
    p.enter(LexMode::Code);
    p.enter_newline_mode(NewlineMode::Continue);
    p.assert(SyntaxKind::LeftBrace);
    code(p, |p| p.at_set(END));
    p.expect_closing_delimiter(m, SyntaxKind::RightBrace);
    p.exit();
    p.exit_newline_mode();
    p.wrap(m, SyntaxKind::CodeBlock);
}

/// Parses a content block: `[*Hi* there!]`.
fn content_block(p: &mut Parser) {
    let m = p.marker();
    p.enter(LexMode::Markup);
    p.assert(SyntaxKind::LeftBracket);
    markup(p, true, 0, |p| p.at(SyntaxKind::RightBracket));
    p.expect_closing_delimiter(m, SyntaxKind::RightBracket);
    p.exit();
    p.wrap(m, SyntaxKind::ContentBlock);
}

/// Parses a let binding: `let x = 1`.
fn let_binding(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Let);

    let m2 = p.marker();
    let mut closure = false;
    let mut other = false;

    if p.eat_if(SyntaxKind::Ident) {
        if p.directly_at(SyntaxKind::LeftParen) {
            params(p);
            closure = true;
        }
    } else {
        pattern(p, false, &mut HashSet::new(), None);
        other = true;
    }

    let f = if closure || other { Parser::expect } else { Parser::eat_if };
    if f(p, SyntaxKind::Eq) {
        code_expr(p);
    }

    if closure {
        p.wrap(m2, SyntaxKind::Closure);
    }

    p.wrap(m, SyntaxKind::LetBinding);
}

/// Parses a set rule: `set text(...)`.
fn set_rule(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Set);

    let m2 = p.marker();
    p.expect(SyntaxKind::Ident);
    while p.eat_if(SyntaxKind::Dot) {
        p.expect(SyntaxKind::Ident);
        p.wrap(m2, SyntaxKind::FieldAccess);
    }

    args(p);
    if p.eat_if(SyntaxKind::If) {
        code_expr(p);
    }
    p.wrap(m, SyntaxKind::SetRule);
}

/// Parses a show rule: `show heading: it => emph(it.body)`.
fn show_rule(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Show);
    let m2 = p.before_trivia();

    if !p.at(SyntaxKind::Colon) {
        code_expr(p);
    }

    if p.eat_if(SyntaxKind::Colon) {
        code_expr(p);
    } else {
        p.expected_at(m2, "colon");
    }

    p.wrap(m, SyntaxKind::ShowRule);
}

/// Parses a contextual expression: `context text.lang`.
fn contextual(p: &mut Parser, atomic: bool) {
    let m = p.marker();
    p.assert(SyntaxKind::Context);
    code_expr_prec(p, atomic, 0);
    p.wrap(m, SyntaxKind::Contextual);
}

/// Parses an if-else conditional: `if x { y } else { z }`.
fn conditional(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::If);
    code_expr(p);
    block(p);
    if p.eat_if(SyntaxKind::Else) {
        if p.at(SyntaxKind::If) {
            conditional(p);
        } else {
            block(p);
        }
    }
    p.wrap(m, SyntaxKind::Conditional);
}

/// Parses a while loop: `while x { y }`.
fn while_loop(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::While);
    code_expr(p);
    block(p);
    p.wrap(m, SyntaxKind::WhileLoop);
}

/// Parses a for loop: `for x in y { z }`.
fn for_loop(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::For);

    let mut seen = HashSet::new();
    pattern(p, false, &mut seen, None);

    let m2 = p.marker();
    if p.eat_if(SyntaxKind::Comma) {
        let node = &mut p[m2];
        node.unexpected();
        node.hint("destructuring patterns must be wrapped in parentheses");
        if p.at_set(set::PATTERN) {
            pattern(p, false, &mut seen, None);
        }
    }

    p.expect(SyntaxKind::In);
    code_expr(p);
    block(p);
    p.wrap(m, SyntaxKind::ForLoop);
}

/// Parses a module import: `import "utils.typ": a, b, c`.
fn module_import(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Import);
    code_expr(p);
    if p.eat_if(SyntaxKind::As) {
        // Allow renaming a full module import.
        // If items are included, both the full module and the items are
        // imported at the same time.
        p.expect(SyntaxKind::Ident);
    }

    if p.eat_if(SyntaxKind::Colon) {
        if p.at(SyntaxKind::LeftParen) {
            let m1 = p.marker();
            p.enter_newline_mode(NewlineMode::Continue);
            p.assert(SyntaxKind::LeftParen);

            import_items(p);

            p.expect_closing_delimiter(m1, SyntaxKind::RightParen);
            p.exit_newline_mode();
        } else if !p.eat_if(SyntaxKind::Star) {
            import_items(p);
        }
    }

    p.wrap(m, SyntaxKind::ModuleImport);
}

/// Parses items to import from a module: `a, b, c`.
fn import_items(p: &mut Parser) {
    let m = p.marker();
    while !p.current().is_terminator() {
        let item_marker = p.marker();
        if !p.eat_if(SyntaxKind::Ident) {
            p.unexpected();
        }

        // Nested import path: `a.b.c`
        while p.eat_if(SyntaxKind::Dot) {
            p.expect(SyntaxKind::Ident);
        }

        p.wrap(item_marker, SyntaxKind::ImportItemPath);

        // Rename imported item.
        if p.eat_if(SyntaxKind::As) {
            p.expect(SyntaxKind::Ident);
            p.wrap(item_marker, SyntaxKind::RenamedImportItem);
        }

        if !p.current().is_terminator() {
            p.expect(SyntaxKind::Comma);
        }
    }

    p.wrap(m, SyntaxKind::ImportItems);
}

/// Parses a module include: `include "chapter1.typ"`.
fn module_include(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Include);
    code_expr(p);
    p.wrap(m, SyntaxKind::ModuleInclude);
}

/// Parses a break from a loop: `break`.
fn break_stmt(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Break);
    p.wrap(m, SyntaxKind::LoopBreak);
}

/// Parses a continue in a loop: `continue`.
fn continue_stmt(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Continue);
    p.wrap(m, SyntaxKind::LoopContinue);
}

/// Parses a return from a function: `return`, `return x + 1`.
fn return_stmt(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Return);
    if p.at_set(set::CODE_EXPR) {
        code_expr(p);
    }
    p.wrap(m, SyntaxKind::FuncReturn);
}

/// An expression that starts with a parenthesis.
fn expr_with_paren(p: &mut Parser, atomic: bool) {
    // If we've seen this position before and have a memoized result, just use
    // it. See below for more explanation about this memoization.
    let start = p.current_start();
    if let Some((range, end_point)) = p.memo.get(&start).cloned() {
        // Restore the end point first, so that it doesn't truncate our freshly
        // pushed nodes. If the current length of `p.nodes` doesn't match what
        // we had in the memoized run, this might otherwise happen.
        p.restore(end_point);
        p.nodes.extend(p.memo_arena[range].iter().cloned());
        return;
    }

    let m = p.marker();
    let checkpoint = p.checkpoint();

    // When we reach a '(', we can't be sure what it is. First, we attempt to
    // parse as a simple parenthesized expression, array, or dictionary as
    // these are the most likely things. We can handle all of those in a single
    // pass.
    let kind = parenthesized_or_array_or_dict(p);
    if atomic {
        return;
    }

    // If, however, '=>' or '=' follows, we must backtrack and reparse as either
    // a parameter list or a destructuring. To be able to do that, we created a
    // parser checkpoint before our speculative parse, which we can restore.
    //
    // However, naive backtracking has a fatal flaw: It can lead to exponential
    // parsing time if we are constantly getting things wrong in a nested
    // scenario. The particular failure case for parameter parsing is the
    // following: `(x: (x: (x) => y) => y) => y`
    //
    // Such a structure will reparse over and over again recursively, leading to
    // a running time of O(2^n) for nesting depth n. To prevent this, we perform
    // a simple trick: When we have done the mistake of picking the wrong path
    // once and have subsequently parsed correctly, we save the result of that
    // correct parsing in the `p.memo` map. When we reach the same position
    // again, we can then just restore this result. In this way, no
    // parenthesized expression is parsed more than twice, leading to a worst
    // case running time of O(2n).
    if p.at(SyntaxKind::Arrow) {
        p.restore(checkpoint);
        params(p);
        if !p.expect(SyntaxKind::Arrow) {
            return;
        }
        code_expr(p);
        p.wrap(m, SyntaxKind::Closure);
    } else if p.at(SyntaxKind::Eq) && kind != SyntaxKind::Parenthesized {
        p.restore(checkpoint);
        destructuring_or_parenthesized(p, true, &mut HashSet::new());
        if !p.expect(SyntaxKind::Eq) {
            return;
        }
        code_expr(p);
        p.wrap(m, SyntaxKind::DestructAssignment);
    } else {
        return;
    }

    // Memoize result if we backtracked.
    let offset = p.memo_arena.len();
    p.memo_arena.extend(p.nodes[m.0..].iter().cloned());
    p.memo.insert(start, (offset..p.memo_arena.len(), p.checkpoint()));
}

/// Parses either
/// - a parenthesized expression: `(1 + 2)`, or
/// - an array: `(1, "hi", 12cm)`, or
/// - a dictionary: `(thickness: 3pt, pattern: dashed)`.
fn parenthesized_or_array_or_dict(p: &mut Parser) -> SyntaxKind {
    let m = p.marker();
    p.enter_newline_mode(NewlineMode::Continue);
    p.assert(SyntaxKind::LeftParen);

    let mut state = GroupState {
        count: 0,
        maybe_just_parens: true,
        kind: None,
        seen: HashSet::new(),
    };

    if p.eat_if(SyntaxKind::Colon) {
        state.kind = Some(SyntaxKind::Dict);
        state.maybe_just_parens = false;
    }

    while !p.current().is_terminator() {
        if !p.at_set(set::ARRAY_OR_DICT_ITEM) {
            p.unexpected();
            continue;
        }

        array_or_dict_item(p, &mut state);
        state.count += 1;

        if !p.current().is_terminator() && p.expect(SyntaxKind::Comma) {
            state.maybe_just_parens = false;
        }
    }

    p.expect_closing_delimiter(m, SyntaxKind::RightParen);
    p.exit_newline_mode();

    let kind = if state.maybe_just_parens && state.count == 1 {
        SyntaxKind::Parenthesized
    } else {
        state.kind.unwrap_or(SyntaxKind::Array)
    };

    p.wrap(m, kind);
    kind
}

/// State for array/dictionary parsing.
struct GroupState {
    count: usize,
    maybe_just_parens: bool,
    kind: Option<SyntaxKind>,
    seen: HashSet<EcoString>,
}

/// Parses a single item in an array or dictionary.
fn array_or_dict_item(p: &mut Parser, state: &mut GroupState) {
    let m = p.marker();

    if p.eat_if(SyntaxKind::Dots) {
        // Parses a spread item: `..item`.
        code_expr(p);
        p.wrap(m, SyntaxKind::Spread);
        state.maybe_just_parens = false;
        return;
    }

    code_expr(p);

    if p.eat_if(SyntaxKind::Colon) {
        // Parses a named/keyed pair: `name: item` or `"key": item`.
        code_expr(p);

        let node = &mut p[m];
        let pair_kind = match node.kind() {
            SyntaxKind::Ident => SyntaxKind::Named,
            _ => SyntaxKind::Keyed,
        };

        if let Some(key) = match node.cast::<ast::Expr>() {
            Some(ast::Expr::Ident(ident)) => Some(ident.get().clone()),
            Some(ast::Expr::Str(s)) => Some(s.get()),
            _ => None,
        } {
            if !state.seen.insert(key.clone()) {
                node.convert_to_error(eco_format!("duplicate key: {key}"));
            }
        }

        p.wrap(m, pair_kind);
        state.maybe_just_parens = false;

        if state.kind == Some(SyntaxKind::Array) {
            p[m].expected("expression");
        } else {
            state.kind = Some(SyntaxKind::Dict);
        }
    } else {
        // Parses a positional item.
        if state.kind == Some(SyntaxKind::Dict) {
            p[m].expected("named or keyed pair");
        } else {
            state.kind = Some(SyntaxKind::Array)
        }
    }
}

/// Parses a function call's argument list: `(12pt, y)`.
fn args(p: &mut Parser) {
    if !p.at(SyntaxKind::LeftParen) && !p.at(SyntaxKind::LeftBracket) {
        p.expected("argument list");
    }

    let m = p.marker();
    if p.at(SyntaxKind::LeftParen) {
        let m2 = p.marker();
        p.enter_newline_mode(NewlineMode::Continue);
        p.assert(SyntaxKind::LeftParen);

        let mut seen = HashSet::new();
        while !p.current().is_terminator() {
            if !p.at_set(set::ARG) {
                p.unexpected();
                continue;
            }

            arg(p, &mut seen);

            if !p.current().is_terminator() {
                p.expect(SyntaxKind::Comma);
            }
        }

        p.expect_closing_delimiter(m2, SyntaxKind::RightParen);
        p.exit_newline_mode();
    }

    while p.directly_at(SyntaxKind::LeftBracket) {
        content_block(p);
    }

    p.wrap(m, SyntaxKind::Args);
}

/// Parses a single argument in an argument list.
fn arg<'s>(p: &mut Parser<'s>, seen: &mut HashSet<&'s str>) {
    let m = p.marker();

    // Parses a spread argument: `..args`.
    if p.eat_if(SyntaxKind::Dots) {
        code_expr(p);
        p.wrap(m, SyntaxKind::Spread);
        return;
    }

    // Parses a normal positional argument or an argument name.
    let was_at_expr = p.at_set(set::CODE_EXPR);
    let text = p.current_text();
    code_expr(p);

    // Parses a named argument: `thickness: 12pt`.
    if p.eat_if(SyntaxKind::Colon) {
        // Recover from bad argument name.
        if was_at_expr {
            if p[m].kind() != SyntaxKind::Ident {
                p[m].expected("identifier");
            } else if !seen.insert(text) {
                p[m].convert_to_error(eco_format!("duplicate argument: {text}"));
            }
        }

        code_expr(p);
        p.wrap(m, SyntaxKind::Named);
    }
}

/// Parses a closure's parameters: `(x, y)`.
fn params(p: &mut Parser) {
    let m = p.marker();
    p.enter_newline_mode(NewlineMode::Continue);
    p.assert(SyntaxKind::LeftParen);

    let mut seen = HashSet::new();
    let mut sink = false;

    while !p.current().is_terminator() {
        if !p.at_set(set::PARAM) {
            p.unexpected();
            continue;
        }

        param(p, &mut seen, &mut sink);

        if !p.current().is_terminator() {
            p.expect(SyntaxKind::Comma);
        }
    }

    p.expect_closing_delimiter(m, SyntaxKind::RightParen);
    p.exit_newline_mode();
    p.wrap(m, SyntaxKind::Params);
}

/// Parses a single parameter in a parameter list.
fn param<'s>(p: &mut Parser<'s>, seen: &mut HashSet<&'s str>, sink: &mut bool) {
    let m = p.marker();

    // Parses argument sink: `..sink`.
    if p.eat_if(SyntaxKind::Dots) {
        if p.at_set(set::PATTERN_LEAF) {
            pattern_leaf(p, false, seen, Some("parameter"));
        }
        p.wrap(m, SyntaxKind::Spread);
        if mem::replace(sink, true) {
            p[m].convert_to_error("only one argument sink is allowed");
        }
        return;
    }

    // Parses a normal positional parameter or a parameter name.
    let was_at_pat = p.at_set(set::PATTERN);
    pattern(p, false, seen, Some("parameter"));

    // Parses a named parameter: `thickness: 12pt`.
    if p.eat_if(SyntaxKind::Colon) {
        // Recover from bad parameter name.
        if was_at_pat && p[m].kind() != SyntaxKind::Ident {
            p[m].expected("identifier");
        }

        code_expr(p);
        p.wrap(m, SyntaxKind::Named);
    }
}

/// Parses a binding or reassignment pattern.
fn pattern<'s>(
    p: &mut Parser<'s>,
    reassignment: bool,
    seen: &mut HashSet<&'s str>,
    dupe: Option<&'s str>,
) {
    match p.current() {
        SyntaxKind::Underscore => p.eat(),
        SyntaxKind::LeftParen => destructuring_or_parenthesized(p, reassignment, seen),
        _ => pattern_leaf(p, reassignment, seen, dupe),
    }
}

/// Parses a destructuring pattern or just a parenthesized pattern.
fn destructuring_or_parenthesized<'s>(
    p: &mut Parser<'s>,
    reassignment: bool,
    seen: &mut HashSet<&'s str>,
) {
    let mut sink = false;
    let mut count = 0;
    let mut maybe_just_parens = true;

    let m = p.marker();
    p.enter_newline_mode(NewlineMode::Continue);
    p.assert(SyntaxKind::LeftParen);

    while !p.current().is_terminator() {
        if !p.at_set(set::DESTRUCTURING_ITEM) {
            p.unexpected();
            continue;
        }

        destructuring_item(p, reassignment, seen, &mut maybe_just_parens, &mut sink);
        count += 1;

        if !p.current().is_terminator() && p.expect(SyntaxKind::Comma) {
            maybe_just_parens = false;
        }
    }

    p.expect_closing_delimiter(m, SyntaxKind::RightParen);
    p.exit_newline_mode();

    if maybe_just_parens && count == 1 && !sink {
        p.wrap(m, SyntaxKind::Parenthesized);
    } else {
        p.wrap(m, SyntaxKind::Destructuring);
    }
}

/// Parses an item in a destructuring pattern.
fn destructuring_item<'s>(
    p: &mut Parser<'s>,
    reassignment: bool,
    seen: &mut HashSet<&'s str>,
    maybe_just_parens: &mut bool,
    sink: &mut bool,
) {
    let m = p.marker();

    // Parse destructuring sink: `..rest`.
    if p.eat_if(SyntaxKind::Dots) {
        if p.at_set(set::PATTERN_LEAF) {
            pattern_leaf(p, reassignment, seen, None);
        }
        p.wrap(m, SyntaxKind::Spread);
        if mem::replace(sink, true) {
            p[m].convert_to_error("only one destructuring sink is allowed");
        }
        return;
    }

    // Parse a normal positional pattern or a destructuring key.
    let was_at_pat = p.at_set(set::PATTERN);
    let checkpoint = p.checkpoint();
    if !(p.eat_if(SyntaxKind::Ident) && p.at(SyntaxKind::Colon)) {
        p.restore(checkpoint);
        pattern(p, reassignment, seen, None);
    }

    // Parse named destructuring item.
    if p.eat_if(SyntaxKind::Colon) {
        // Recover from bad named destructuring.
        if was_at_pat && p[m].kind() != SyntaxKind::Ident {
            p[m].expected("identifier");
        }

        pattern(p, reassignment, seen, None);
        p.wrap(m, SyntaxKind::Named);
        *maybe_just_parens = false;
    }
}

/// Parses a leaf in a pattern - either an identifier or an expression
/// depending on whether it's a binding or reassignment pattern.
fn pattern_leaf<'s>(
    p: &mut Parser<'s>,
    reassignment: bool,
    seen: &mut HashSet<&'s str>,
    dupe: Option<&'s str>,
) {
    if p.current().is_keyword() {
        p.eat_and_get().expected("pattern");
        return;
    } else if !p.at_set(set::PATTERN_LEAF) {
        p.expected("pattern");
        return;
    }

    let m = p.marker();
    let text = p.current_text();

    // We parse an atomic expression even though we only want an identifier for
    // better error recovery. We can mark the whole expression as unexpected
    // instead of going through its pieces one by one.
    code_expr_prec(p, true, 0);

    if !reassignment {
        let node = &mut p[m];
        if node.kind() == SyntaxKind::Ident {
            if !seen.insert(text) {
                node.convert_to_error(eco_format!(
                    "duplicate {}: {text}",
                    dupe.unwrap_or("binding"),
                ));
            }
        } else {
            node.expected("pattern");
        }
    }
}

/// Manages parsing of a stream of tokens.
struct Parser<'s> {
    text: &'s str,
    lexer: Lexer<'s>,
    prev_end: usize,
    current_start: usize,
    current: SyntaxKind,
    balanced: bool,
    nodes: Vec<SyntaxNode>,
    modes: Vec<LexMode>,
    newline_modes: Vec<NewlineMode>,
    memo: HashMap<usize, (Range<usize>, Checkpoint<'s>)>,
    memo_arena: Vec<SyntaxNode>,
}

/// How to proceed with parsing when seeing a newline.
#[derive(Clone)]
enum NewlineMode {
    /// Stop always.
    Stop,
    /// Proceed if there is no continuation with `else` or `.`
    Contextual,
    /// Just proceed like with normal whitespace.
    Continue,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Marker(usize);

#[derive(Clone)]
struct Checkpoint<'s> {
    lexer: Lexer<'s>,
    prev_end: usize,
    current_start: usize,
    current: SyntaxKind,
    nodes: usize,
}

impl<'s> Parser<'s> {
    fn new(text: &'s str, offset: usize, mode: LexMode) -> Self {
        let mut lexer = Lexer::new(text, mode);
        lexer.jump(offset);
        let current = lexer.next();
        Self {
            lexer,
            text,
            prev_end: offset,
            current_start: offset,
            current,
            balanced: true,
            nodes: vec![],
            modes: vec![],
            newline_modes: vec![],
            memo: HashMap::new(),
            memo_arena: vec![],
        }
    }

    fn finish(self) -> Vec<SyntaxNode> {
        self.nodes
    }

    fn prev_end(&self) -> usize {
        self.prev_end
    }

    fn current(&self) -> SyntaxKind {
        self.current
    }

    fn current_start(&self) -> usize {
        self.current_start
    }

    fn current_end(&self) -> usize {
        self.lexer.cursor()
    }

    fn current_text(&self) -> &'s str {
        &self.text[self.current_start..self.current_end()]
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current == kind
    }

    fn at_set(&self, set: SyntaxSet) -> bool {
        set.contains(self.current)
    }

    fn end(&self) -> bool {
        self.at(SyntaxKind::End)
    }

    fn directly_at(&self, kind: SyntaxKind) -> bool {
        self.current == kind && self.prev_end == self.current_start
    }

    fn eat(&mut self) {
        self.save();
        self.lex();
        self.skip();
    }

    #[track_caller]
    fn eat_and_get(&mut self) -> &mut SyntaxNode {
        let offset = self.nodes.len();
        self.save();
        self.lex();
        self.skip();
        &mut self.nodes[offset]
    }

    /// Eats if at `kind`.
    ///
    /// Note: In math and code mode, this will ignore trivia in front of the
    /// `kind`, To forbid skipping trivia, consider using `eat_if_direct`.
    fn eat_if(&mut self, kind: SyntaxKind) -> bool {
        let at = self.at(kind);
        if at {
            self.eat();
        }
        at
    }

    /// Eats only if currently at the start of `kind`.
    fn eat_if_direct(&mut self, kind: SyntaxKind) -> bool {
        let at = self.directly_at(kind);
        if at {
            self.eat();
        }
        at
    }

    #[track_caller]
    fn assert(&mut self, kind: SyntaxKind) {
        assert_eq!(self.current, kind);
        self.eat();
    }

    fn convert(&mut self, kind: SyntaxKind) {
        self.current = kind;
        self.eat();
    }

    fn newline(&mut self) -> bool {
        self.lexer.newline()
    }

    fn column(&self, at: usize) -> usize {
        self.text[..at].chars().rev().take_while(|&c| !is_newline(c)).count()
    }

    fn marker(&self) -> Marker {
        Marker(self.nodes.len())
    }

    /// Get a marker after the last non-trivia node.
    fn before_trivia(&self) -> Marker {
        let mut i = self.nodes.len();
        if self.lexer.mode() != LexMode::Markup && self.prev_end != self.current_start {
            while i > 0 && self.nodes[i - 1].kind().is_trivia() {
                i -= 1;
            }
        }
        Marker(i)
    }

    /// Whether the last non-trivia node is an error.
    fn after_error(&mut self) -> bool {
        let m = self.before_trivia();
        m.0 > 0 && self.nodes[m.0 - 1].kind().is_error()
    }

    #[track_caller]
    fn post_process(&mut self, m: Marker) -> impl Iterator<Item = &mut SyntaxNode> {
        self.nodes[m.0..]
            .iter_mut()
            .filter(|child| !child.kind().is_error() && !child.kind().is_trivia())
    }

    fn wrap(&mut self, from: Marker, kind: SyntaxKind) {
        self.wrap_within(from, self.before_trivia(), kind);
    }

    fn wrap_all(&mut self, from: Marker, kind: SyntaxKind) {
        self.wrap_within(from, Marker(self.nodes.len()), kind)
    }

    fn wrap_within(&mut self, from: Marker, to: Marker, kind: SyntaxKind) {
        let len = self.nodes.len();
        let to = to.0.min(len);
        let from = from.0.min(to);
        let children = self.nodes.drain(from..to).collect();
        self.nodes.insert(from, SyntaxNode::inner(kind, children));
    }

    fn enter(&mut self, mode: LexMode) {
        self.modes.push(self.lexer.mode());
        self.lexer.set_mode(mode);
    }

    fn exit(&mut self) {
        let mode = self.modes.pop().unwrap();
        if mode != self.lexer.mode() {
            self.unskip();
            self.lexer.set_mode(mode);
            self.lexer.jump(self.current_start);
            self.lex();
            self.skip();
        }
    }

    fn enter_newline_mode(&mut self, stop: NewlineMode) {
        self.newline_modes.push(stop);
    }

    fn exit_newline_mode(&mut self) {
        self.unskip();
        self.newline_modes.pop();
        self.lexer.jump(self.prev_end);
        self.lex();
        self.skip();
    }

    fn checkpoint(&self) -> Checkpoint<'s> {
        Checkpoint {
            lexer: self.lexer.clone(),
            prev_end: self.prev_end,
            current_start: self.current_start,
            current: self.current,
            nodes: self.nodes.len(),
        }
    }

    fn restore(&mut self, checkpoint: Checkpoint<'s>) {
        self.lexer = checkpoint.lexer;
        self.prev_end = checkpoint.prev_end;
        self.current_start = checkpoint.current_start;
        self.current = checkpoint.current;
        self.nodes.truncate(checkpoint.nodes);
    }

    fn skip(&mut self) {
        if self.lexer.mode() != LexMode::Markup {
            while self.current.is_trivia() {
                self.save();
                self.lex();
            }
        }
    }

    fn unskip(&mut self) {
        if self.lexer.mode() != LexMode::Markup && self.prev_end != self.current_start {
            while self.nodes.last().is_some_and(|last| last.kind().is_trivia()) {
                self.nodes.pop();
            }

            self.lexer.jump(self.prev_end);
            self.lex();
        }
    }

    fn save(&mut self) {
        let text = self.current_text();
        if self.at(SyntaxKind::Error) {
            let error = self.lexer.take_error().unwrap();
            self.nodes.push(SyntaxNode::error(error, text));
        } else {
            self.nodes.push(SyntaxNode::leaf(self.current, text));
        }

        if self.lexer.mode() == LexMode::Markup || !self.current.is_trivia() {
            self.prev_end = self.current_end();
        }
    }

    fn next_non_trivia(lexer: &mut Lexer<'s>) -> SyntaxKind {
        loop {
            let next = lexer.next();
            // Loop is terminable, because SyntaxKind::End is not a trivia.
            if !next.is_trivia() {
                break next;
            }
        }
    }

    fn lex(&mut self) {
        self.current_start = self.lexer.cursor();
        self.current = self.lexer.next();

        // Special cases to handle newlines in code mode.
        if self.lexer.mode() == LexMode::Code
            && self.lexer.newline()
            && match self.newline_modes.last() {
                Some(NewlineMode::Continue) => false,
                Some(NewlineMode::Contextual) => !matches!(
                    Self::next_non_trivia(&mut self.lexer.clone()),
                    SyntaxKind::Else | SyntaxKind::Dot
                ),
                Some(NewlineMode::Stop) => true,
                None => false,
            }
        {
            self.current = SyntaxKind::End;
        }
    }
}

impl<'s> Parser<'s> {
    /// Consume the given syntax `kind` or produce an error.
    fn expect(&mut self, kind: SyntaxKind) -> bool {
        let at = self.at(kind);
        if at {
            self.eat();
        } else if kind == SyntaxKind::Ident && self.current.is_keyword() {
            self.trim_errors();
            self.eat_and_get().expected(kind.name());
        } else {
            self.balanced &= !kind.is_grouping();
            self.expected(kind.name());
        }
        at
    }

    /// Consume the given closing delimiter or produce an error for the matching
    /// opening delimiter at `open`.
    #[track_caller]
    fn expect_closing_delimiter(&mut self, open: Marker, kind: SyntaxKind) {
        if !self.eat_if(kind) {
            self.nodes[open.0].convert_to_error("unclosed delimiter");
        }
    }

    /// Produce an error that the given `thing` was expected.
    fn expected(&mut self, thing: &str) {
        if !self.after_error() {
            self.expected_at(self.before_trivia(), thing);
        }
    }

    /// Produce an error that the given `thing` was expected at the position
    /// of the marker `m`.
    fn expected_at(&mut self, m: Marker, thing: &str) {
        let error =
            SyntaxNode::error(SyntaxError::new(eco_format!("expected {thing}")), "");
        self.nodes.insert(m.0, error);
    }

    /// Produce a hint.
    fn hint(&mut self, hint: &str) {
        let m = self.before_trivia();
        if let Some(error) = self.nodes.get_mut(m.0 - 1) {
            error.hint(hint);
        }
    }

    /// Consume the next token (if any) and produce an error stating that it was
    /// unexpected.
    fn unexpected(&mut self) {
        self.trim_errors();
        self.balanced &= !self.current.is_grouping();
        self.eat_and_get().unexpected();
    }

    /// Remove trailing errors with zero length.
    fn trim_errors(&mut self) {
        let Marker(end) = self.before_trivia();
        let mut start = end;
        while start > 0
            && self.nodes[start - 1].kind().is_error()
            && self.nodes[start - 1].is_empty()
        {
            start -= 1;
        }
        self.nodes.drain(start..end);
    }
}

impl Index<Marker> for Parser<'_> {
    type Output = SyntaxNode;

    fn index(&self, m: Marker) -> &Self::Output {
        &self.nodes[m.0]
    }
}

impl IndexMut<Marker> for Parser<'_> {
    fn index_mut(&mut self, m: Marker) -> &mut Self::Output {
        &mut self.nodes[m.0]
    }
}
