use std::collections::HashSet;
use std::ops::Range;

use ecow::{eco_format, EcoString};
use unicode_math_class::MathClass;

use super::{ast, is_newline, LexMode, Lexer, SyntaxKind, SyntaxNode};

/// Parse a source file.
#[tracing::instrument(skip_all)]
pub fn parse(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, 0, LexMode::Markup);
    markup(&mut p, true, 0, |_| false);
    p.finish().into_iter().next().unwrap()
}

/// Parse top-level code.
#[tracing::instrument(skip_all)]
pub fn parse_code(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, 0, LexMode::Code);
    let m = p.marker();
    p.skip();
    code_exprs(&mut p, |_| false);
    p.wrap_all(m, SyntaxKind::Code);
    p.finish().into_iter().next().unwrap()
}

/// Parse top-level math.
#[tracing::instrument(skip_all)]
pub fn parse_math(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, 0, LexMode::Math);
    math(&mut p, |_| false);
    p.finish().into_iter().next().unwrap()
}

fn markup(
    p: &mut Parser,
    mut at_start: bool,
    min_indent: usize,
    mut stop: impl FnMut(&Parser) -> bool,
) {
    let m = p.marker();
    let mut nesting: usize = 0;
    while !p.eof() {
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

        let prev = p.prev_end();
        markup_expr(p, &mut at_start);
        if !p.progress(prev) {
            p.unexpected();
        }
    }
    p.wrap(m, SyntaxKind::Markup);
}

pub(super) fn reparse_markup(
    text: &str,
    range: Range<usize>,
    at_start: &mut bool,
    nesting: &mut usize,
    mut stop: impl FnMut(SyntaxKind) -> bool,
) -> Option<Vec<SyntaxNode>> {
    let mut p = Parser::new(text, range.start, LexMode::Markup);
    while !p.eof() && p.current_start() < range.end {
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

        let prev = p.prev_end();
        markup_expr(&mut p, at_start);
        if !p.progress(prev) {
            p.unexpected();
        }
    }
    (p.balanced && p.current_start() == range.end).then(|| p.finish())
}

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
        | SyntaxKind::Raw
        | SyntaxKind::Link
        | SyntaxKind::Label => p.eat(),

        SyntaxKind::Hash => embedded_code_expr(p),
        SyntaxKind::Star => strong(p),
        SyntaxKind::Underscore => emph(p),
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

fn strong(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Star);
    markup(p, false, 0, |p| {
        p.at(SyntaxKind::Star)
            || p.at(SyntaxKind::Parbreak)
            || p.at(SyntaxKind::RightBracket)
    });
    p.expect_closing_delimiter(m, SyntaxKind::Star);
    p.wrap(m, SyntaxKind::Strong);
}

fn emph(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Underscore);
    markup(p, false, 0, |p| {
        p.at(SyntaxKind::Underscore)
            || p.at(SyntaxKind::Parbreak)
            || p.at(SyntaxKind::RightBracket)
    });
    p.expect_closing_delimiter(m, SyntaxKind::Underscore);
    p.wrap(m, SyntaxKind::Emph);
}

fn heading(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::HeadingMarker);
    whitespace_line(p);
    markup(p, false, usize::MAX, |p| {
        p.at(SyntaxKind::Label)
            || p.at(SyntaxKind::RightBracket)
            || (p.at(SyntaxKind::Space) && p.lexer.clone().next() == SyntaxKind::Label)
    });
    p.wrap(m, SyntaxKind::Heading);
}

fn list_item(p: &mut Parser) {
    let m = p.marker();
    let min_indent = p.column(p.current_start()) + 1;
    p.assert(SyntaxKind::ListMarker);
    whitespace_line(p);
    markup(p, false, min_indent, |p| p.at(SyntaxKind::RightBracket));
    p.wrap(m, SyntaxKind::ListItem);
}

fn enum_item(p: &mut Parser) {
    let m = p.marker();
    let min_indent = p.column(p.current_start()) + 1;
    p.assert(SyntaxKind::EnumMarker);
    whitespace_line(p);
    markup(p, false, min_indent, |p| p.at(SyntaxKind::RightBracket));
    p.wrap(m, SyntaxKind::EnumItem);
}

fn term_item(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::TermMarker);
    let min_indent = p.column(p.prev_end());
    whitespace_line(p);
    markup(p, false, usize::MAX, |p| {
        p.at(SyntaxKind::Colon) || p.at(SyntaxKind::RightBracket)
    });
    p.expect(SyntaxKind::Colon);
    whitespace_line(p);
    markup(p, false, min_indent, |p| p.at(SyntaxKind::RightBracket));
    p.wrap(m, SyntaxKind::TermItem);
}

fn reference(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::RefMarker);
    if p.directly_at(SyntaxKind::LeftBracket) {
        content_block(p);
    }
    p.wrap(m, SyntaxKind::Ref);
}

fn whitespace_line(p: &mut Parser) {
    while !p.newline() && p.current().is_trivia() {
        p.eat();
    }
}

fn equation(p: &mut Parser) {
    let m = p.marker();
    p.enter(LexMode::Math);
    p.assert(SyntaxKind::Dollar);
    math(p, |p| p.at(SyntaxKind::Dollar));
    p.expect_closing_delimiter(m, SyntaxKind::Dollar);
    p.exit();
    p.wrap(m, SyntaxKind::Equation);
}

fn math(p: &mut Parser, mut stop: impl FnMut(&Parser) -> bool) {
    let m = p.marker();
    while !p.eof() && !stop(p) {
        let prev = p.prev_end();
        math_expr(p);
        if !p.progress(prev) {
            p.unexpected();
        }
    }
    p.wrap(m, SyntaxKind::Math);
}

fn math_expr(p: &mut Parser) {
    math_expr_prec(p, 0, SyntaxKind::Eof)
}

fn math_expr_prec(p: &mut Parser, min_prec: usize, stop: SyntaxKind) {
    let m = p.marker();
    let mut continuable = false;
    match p.current() {
        SyntaxKind::Hash => embedded_code_expr(p),
        SyntaxKind::MathIdent => {
            continuable = true;
            p.eat();
            while p.directly_at(SyntaxKind::Text)
                && p.current_text() == "."
                && matches!(
                    p.lexer.clone().next(),
                    SyntaxKind::MathIdent | SyntaxKind::Text
                )
            {
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

        SyntaxKind::Text | SyntaxKind::Shorthand => {
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

    while !p.eof() && !p.at(stop) {
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

        // Separate primes and superscripts to different attachments.
        if primed && p.current() == SyntaxKind::Hat {
            p.wrap(m, SyntaxKind::MathAttach);
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

        if p.eat_if(SyntaxKind::Underscore) || (!primed && p.eat_if(SyntaxKind::Hat)) {
            let m3 = p.marker();
            math_expr_prec(p, prec, SyntaxKind::Eof);
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
    while !p.eof() && !p.at(SyntaxKind::Dollar) {
        if math_class(p.current_text()) == Some(MathClass::Closing) {
            p.wrap(m2, SyntaxKind::Math);
            p.eat();
            p.wrap(m, SyntaxKind::MathDelimited);
            return;
        }

        let prev = p.prev_end();
        math_expr(p);
        if !p.progress(prev) {
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
            Some((SyntaxKind::MathFrac, SyntaxKind::Eof, ast::Assoc::Left, 1))
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

    while !p.eof() && !p.at(SyntaxKind::Dollar) {
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

        let prev = p.prev_end();
        math_expr(p);
        if !p.progress(prev) {
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

fn maybe_wrap_in_math(p: &mut Parser, arg: Marker, named: Option<Marker>) {
    let exprs = p.post_process(arg).filter(|node| node.is::<ast::Expr>()).count();
    if exprs != 1 {
        p.wrap(arg, SyntaxKind::Math);
    }

    if let Some(m) = named {
        p.wrap(m, SyntaxKind::Named);
    }
}

fn code(p: &mut Parser, stop: impl FnMut(&Parser) -> bool) {
    let m = p.marker();
    code_exprs(p, stop);
    p.wrap(m, SyntaxKind::Code);
}

fn code_exprs(p: &mut Parser, mut stop: impl FnMut(&Parser) -> bool) {
    while !p.eof() && !stop(p) {
        p.enter_newline_mode(NewlineMode::Contextual);
        let prev = p.prev_end();
        code_expr(p);
        if p.progress(prev) && !p.eof() && !stop(p) && !p.eat_if(SyntaxKind::Semicolon) {
            p.expected("semicolon or line break");
        }
        p.exit_newline_mode();
        if !p.progress(prev) && !p.eof() {
            p.unexpected();
        }
    }
}

fn code_expr(p: &mut Parser) {
    code_expr_prec(p, false, 0, false)
}

fn code_expr_or_pattern(p: &mut Parser) {
    code_expr_prec(p, false, 0, true)
}

fn embedded_code_expr(p: &mut Parser) {
    p.enter_newline_mode(NewlineMode::Stop);
    p.enter(LexMode::Code);
    p.assert(SyntaxKind::Hash);
    p.unskip();

    let stmt = matches!(
        p.current(),
        SyntaxKind::Let
            | SyntaxKind::Set
            | SyntaxKind::Show
            | SyntaxKind::Import
            | SyntaxKind::Include
    );

    let prev = p.prev_end();
    code_expr_prec(p, true, 0, false);

    // Consume error for things like `#12p` or `#"abc\"`.#
    if !p.progress(prev) && !p.current().is_trivia() && !p.eof() {
        p.unexpected();
    }

    let semi =
        (stmt || p.directly_at(SyntaxKind::Semicolon)) && p.eat_if(SyntaxKind::Semicolon);

    if stmt && !semi && !p.eof() && !p.at(SyntaxKind::RightBracket) {
        p.expected("semicolon or line break");
    }

    p.exit();
    p.exit_newline_mode();
}

fn code_expr_prec(
    p: &mut Parser,
    atomic: bool,
    min_prec: usize,
    allow_destructuring: bool,
) {
    let m = p.marker();
    if let (false, Some(op)) = (atomic, ast::UnOp::from_kind(p.current())) {
        p.eat();
        code_expr_prec(p, atomic, op.precedence(), false);
        p.wrap(m, SyntaxKind::Unary);
    } else {
        code_primary(p, atomic, allow_destructuring);
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

        let binop =
            if ast::BinOp::NotIn.precedence() >= min_prec && p.eat_if(SyntaxKind::Not) {
                if p.at(SyntaxKind::In) {
                    Some(ast::BinOp::NotIn)
                } else {
                    p.expected("keyword `in`");
                    break;
                }
            } else {
                ast::BinOp::from_kind(p.current())
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
            code_expr_prec(p, false, prec, false);
            p.wrap(m, SyntaxKind::Binary);
            continue;
        }

        break;
    }
}

fn code_primary(p: &mut Parser, atomic: bool, allow_destructuring: bool) {
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
            } else if let Some(underscore) = p.node_mut(m) {
                underscore.convert_to_error("expected expression, found underscore");
            }
        }

        SyntaxKind::LeftBrace => code_block(p),
        SyntaxKind::LeftBracket => content_block(p),
        SyntaxKind::LeftParen => with_paren(p, allow_destructuring),
        SyntaxKind::Dollar => equation(p),
        SyntaxKind::Let => let_binding(p),
        SyntaxKind::Set => set_rule(p),
        SyntaxKind::Show => show_rule(p),
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
        | SyntaxKind::Label
        | SyntaxKind::Raw => p.eat(),

        _ => p.expected("expression"),
    }
}

fn block(p: &mut Parser) {
    match p.current() {
        SyntaxKind::LeftBracket => content_block(p),
        SyntaxKind::LeftBrace => code_block(p),
        _ => p.expected("block"),
    }
}

pub(super) fn reparse_block(text: &str, range: Range<usize>) -> Option<SyntaxNode> {
    let mut p = Parser::new(text, range.start, LexMode::Code);
    assert!(p.at(SyntaxKind::LeftBracket) || p.at(SyntaxKind::LeftBrace));
    block(&mut p);
    (p.balanced && p.prev_end() == range.end)
        .then(|| p.finish().into_iter().next().unwrap())
}

fn code_block(p: &mut Parser) {
    let m = p.marker();
    p.enter(LexMode::Code);
    p.enter_newline_mode(NewlineMode::Continue);
    p.assert(SyntaxKind::LeftBrace);
    code(p, |p| {
        p.at(SyntaxKind::RightBrace)
            || p.at(SyntaxKind::RightBracket)
            || p.at(SyntaxKind::RightParen)
    });
    p.expect_closing_delimiter(m, SyntaxKind::RightBrace);
    p.exit();
    p.exit_newline_mode();
    p.wrap(m, SyntaxKind::CodeBlock);
}

fn content_block(p: &mut Parser) {
    let m = p.marker();
    p.enter(LexMode::Markup);
    p.assert(SyntaxKind::LeftBracket);
    markup(p, true, 0, |p| p.at(SyntaxKind::RightBracket));
    p.expect_closing_delimiter(m, SyntaxKind::RightBracket);
    p.exit();
    p.wrap(m, SyntaxKind::ContentBlock);
}

fn with_paren(p: &mut Parser, allow_destructuring: bool) {
    let m = p.marker();
    let mut kind = collection(p, true);
    if p.at(SyntaxKind::Arrow) {
        validate_params_at(p, m);
        p.wrap(m, SyntaxKind::Params);
        p.assert(SyntaxKind::Arrow);
        code_expr(p);
        kind = SyntaxKind::Closure;
    } else if p.at(SyntaxKind::Eq) && kind != SyntaxKind::Parenthesized {
        // TODO: add warning if p.at(SyntaxKind::Eq) && kind == SyntaxKind::Parenthesized

        validate_pattern_at(p, m, false);
        p.wrap(m, SyntaxKind::Destructuring);
        p.assert(SyntaxKind::Eq);
        code_expr(p);
        kind = SyntaxKind::DestructAssignment;
    }

    match kind {
        SyntaxKind::Array if !allow_destructuring => validate_array_at(p, m),
        SyntaxKind::Dict if !allow_destructuring => validate_dict_at(p, m),
        SyntaxKind::Parenthesized if !allow_destructuring => {
            validate_parenthesized_at(p, m)
        }
        SyntaxKind::Destructuring if !allow_destructuring => {
            invalidate_destructuring(p, m)
        }
        _ => {}
    }
    p.wrap(m, kind);
}

fn invalidate_destructuring(p: &mut Parser, m: Marker) {
    let mut collection_kind = Option::None;
    for child in p.post_process(m) {
        match child.kind() {
            SyntaxKind::Named | SyntaxKind::Keyed => match collection_kind {
                Some(SyntaxKind::Array) => child.convert_to_error(eco_format!(
                    "expected expression, found {}",
                    child.kind().name()
                )),
                _ => collection_kind = Some(SyntaxKind::Dict),
            },
            SyntaxKind::LeftParen | SyntaxKind::RightParen | SyntaxKind::Comma => {}
            kind => match collection_kind {
                Some(SyntaxKind::Dict) => child.convert_to_error(eco_format!(
                    "expected named or keyed pair, found {}",
                    kind.name()
                )),
                _ => collection_kind = Some(SyntaxKind::Array),
            },
        }
    }
}

fn collection(p: &mut Parser, keyed: bool) -> SyntaxKind {
    p.enter_newline_mode(NewlineMode::Continue);

    let m = p.marker();
    p.assert(SyntaxKind::LeftParen);

    let mut count = 0;
    let mut parenthesized = true;
    let mut kind = None;
    if keyed && p.eat_if(SyntaxKind::Colon) {
        kind = Some(SyntaxKind::Dict);
        parenthesized = false;
    }

    while !p.current().is_terminator() {
        let prev = p.prev_end();
        match item(p, keyed) {
            SyntaxKind::Spread => parenthesized = false,
            SyntaxKind::Named | SyntaxKind::Keyed => {
                match kind {
                    Some(SyntaxKind::Array) => kind = Some(SyntaxKind::Destructuring),
                    _ => kind = Some(SyntaxKind::Dict),
                }
                parenthesized = false;
            }
            SyntaxKind::Int => match kind {
                Some(SyntaxKind::Array) | None => kind = Some(SyntaxKind::Array),
                Some(_) => kind = Some(SyntaxKind::Destructuring),
            },
            _ if kind.is_none() => kind = Some(SyntaxKind::Array),
            _ => {}
        }

        if !p.progress(prev) {
            p.unexpected();
            continue;
        }

        count += 1;

        if p.current().is_terminator() {
            break;
        }

        if p.expect(SyntaxKind::Comma) {
            parenthesized = false;
        }
    }

    p.expect_closing_delimiter(m, SyntaxKind::RightParen);
    p.exit_newline_mode();

    if parenthesized && count == 1 {
        SyntaxKind::Parenthesized
    } else {
        kind.unwrap_or(SyntaxKind::Array)
    }
}

fn item(p: &mut Parser, keyed: bool) -> SyntaxKind {
    let m = p.marker();

    if p.eat_if(SyntaxKind::Dots) {
        if p.at(SyntaxKind::Comma) || p.at(SyntaxKind::RightParen) {
            p.wrap(m, SyntaxKind::Spread);
            return SyntaxKind::Spread;
        }

        code_expr(p);
        p.wrap(m, SyntaxKind::Spread);
        return SyntaxKind::Spread;
    }

    if p.at(SyntaxKind::Underscore) {
        // This is a temporary workaround to fix `v.map(_ => {})`.
        let mut lexer = p.lexer.clone();
        let next =
            std::iter::from_fn(|| Some(lexer.next())).find(|kind| !kind.is_trivia());
        if next != Some(SyntaxKind::Arrow) {
            p.eat();
            return SyntaxKind::Underscore;
        }
    }

    code_expr_or_pattern(p);

    if !p.eat_if(SyntaxKind::Colon) {
        return SyntaxKind::Int;
    }

    if !p.eat_if(SyntaxKind::Underscore) {
        code_expr(p);
    }

    let kind = match p.node(m).map(SyntaxNode::kind) {
        Some(SyntaxKind::Ident) => SyntaxKind::Named,
        Some(SyntaxKind::Str) if keyed => SyntaxKind::Keyed,
        _ => {
            for child in p.post_process(m) {
                if child.kind() == SyntaxKind::Colon {
                    break;
                }

                let mut message = EcoString::from("expected identifier");
                if keyed {
                    message.push_str(" or string");
                }
                message.push_str(", found ");
                message.push_str(child.kind().name());
                child.convert_to_error(message);
            }
            SyntaxKind::Named
        }
    };

    p.wrap(m, kind);
    kind
}

fn args(p: &mut Parser) {
    if !p.at(SyntaxKind::LeftParen) && !p.at(SyntaxKind::LeftBracket) {
        p.expected("argument list");
    }

    let m = p.marker();
    if p.at(SyntaxKind::LeftParen) {
        collection(p, false);
        validate_args_at(p, m);
    }

    while p.directly_at(SyntaxKind::LeftBracket) {
        content_block(p);
    }

    p.wrap(m, SyntaxKind::Args);
}

enum PatternKind {
    Ident,
    Placeholder,
    Destructuring,
}

fn pattern(p: &mut Parser) -> PatternKind {
    let m = p.marker();
    if p.at(SyntaxKind::LeftParen) {
        let kind = collection(p, false);
        validate_pattern_at(p, m, true);

        if kind == SyntaxKind::Parenthesized {
            PatternKind::Ident
        } else {
            p.wrap(m, SyntaxKind::Destructuring);
            PatternKind::Destructuring
        }
    } else if p.eat_if(SyntaxKind::Underscore) {
        PatternKind::Placeholder
    } else {
        p.expect(SyntaxKind::Ident);
        PatternKind::Ident
    }
}

fn let_binding(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Let);

    let m2 = p.marker();
    let mut closure = false;
    let mut destructuring = false;
    match pattern(p) {
        PatternKind::Ident => {
            closure = p.directly_at(SyntaxKind::LeftParen);
            if closure {
                let m3 = p.marker();
                collection(p, false);
                validate_params_at(p, m3);
                p.wrap(m3, SyntaxKind::Params);
            }
        }
        PatternKind::Placeholder => {}
        PatternKind::Destructuring => destructuring = true,
    }

    let f = if closure || destructuring { Parser::expect } else { Parser::eat_if };
    if f(p, SyntaxKind::Eq) {
        code_expr(p);
    }

    if closure {
        p.wrap(m2, SyntaxKind::Closure);
    }

    p.wrap(m, SyntaxKind::LetBinding);
}

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

fn while_loop(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::While);
    code_expr(p);
    block(p);
    p.wrap(m, SyntaxKind::WhileLoop);
}

fn for_loop(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::For);
    pattern(p);
    if p.at(SyntaxKind::Comma) {
        p.expected("keyword `in`");
        p.hint("did you mean to use a destructuring pattern?");
        if !p.eat_if(SyntaxKind::Ident) {
            p.eat_if(SyntaxKind::Underscore);
        }
        p.eat_if(SyntaxKind::In);
    } else {
        p.expect(SyntaxKind::In);
    }
    code_expr(p);
    block(p);
    p.wrap(m, SyntaxKind::ForLoop);
}

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
    if p.eat_if(SyntaxKind::Colon) && !p.eat_if(SyntaxKind::Star) {
        import_items(p);
    }
    p.wrap(m, SyntaxKind::ModuleImport);
}

fn import_items(p: &mut Parser) {
    let m = p.marker();
    while !p.eof() && !p.at(SyntaxKind::Semicolon) {
        let item_marker = p.marker();
        if !p.eat_if(SyntaxKind::Ident) {
            p.unexpected();
        }

        // Rename imported item.
        if p.eat_if(SyntaxKind::As) {
            p.expect(SyntaxKind::Ident);
            p.wrap(item_marker, SyntaxKind::RenamedImportItem);
        }

        if p.current().is_terminator() {
            break;
        }
        p.expect(SyntaxKind::Comma);
    }
    p.wrap(m, SyntaxKind::ImportItems);
}

fn module_include(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Include);
    code_expr(p);
    p.wrap(m, SyntaxKind::ModuleInclude);
}

fn break_stmt(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Break);
    p.wrap(m, SyntaxKind::LoopBreak);
}

fn continue_stmt(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Continue);
    p.wrap(m, SyntaxKind::LoopContinue);
}

fn return_stmt(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Return);
    if !p.current().is_terminator() && !p.at(SyntaxKind::Comma) {
        code_expr(p);
    }
    p.wrap(m, SyntaxKind::FuncReturn);
}

fn validate_parenthesized_at(p: &mut Parser, m: Marker) {
    for child in p.post_process(m) {
        let kind = child.kind();
        match kind {
            SyntaxKind::Array => validate_array(child.children_mut().iter_mut()),
            SyntaxKind::Dict => validate_dict(child.children_mut().iter_mut()),
            SyntaxKind::Underscore => {
                child.convert_to_error(eco_format!(
                    "expected expression, found {}",
                    kind.name()
                ));
            }
            _ => {}
        }
    }
}

fn validate_array_at(p: &mut Parser, m: Marker) {
    validate_array(p.post_process(m))
}

fn validate_array<'a>(children: impl Iterator<Item = &'a mut SyntaxNode>) {
    for child in children {
        let kind = child.kind();
        match kind {
            SyntaxKind::Array => validate_array(child.children_mut().iter_mut()),
            SyntaxKind::Dict => validate_dict(child.children_mut().iter_mut()),
            SyntaxKind::Named | SyntaxKind::Keyed | SyntaxKind::Underscore => {
                child.convert_to_error(eco_format!(
                    "expected expression, found {}",
                    kind.name()
                ));
            }
            _ => {}
        }
    }
}

fn validate_dict_at(p: &mut Parser, m: Marker) {
    validate_dict(p.post_process(m))
}

fn validate_dict<'a>(children: impl Iterator<Item = &'a mut SyntaxNode>) {
    let mut used = HashSet::new();
    for child in children {
        match child.kind() {
            SyntaxKind::Named | SyntaxKind::Keyed => {
                let Some(first) = child.children_mut().first_mut() else { continue };
                let key = match first.cast::<ast::Str>() {
                    Some(str) => str.get(),
                    None => first.text().clone(),
                };

                if !used.insert(key.clone()) {
                    first.convert_to_error(eco_format!("duplicate key: {key}"));
                    child.make_erroneous();
                }
            }
            SyntaxKind::Spread => {}
            SyntaxKind::LeftParen
            | SyntaxKind::RightParen
            | SyntaxKind::Comma
            | SyntaxKind::Colon
            | SyntaxKind::Space => {}
            kind => {
                child.convert_to_error(eco_format!(
                    "expected named or keyed pair, found {}",
                    kind.name()
                ));
            }
        }
    }
}

fn validate_params_at(p: &mut Parser, m: Marker) {
    let mut used_spread = false;
    let mut used = HashSet::new();
    for child in p.post_process(m) {
        match child.kind() {
            SyntaxKind::Ident => {
                if !used.insert(child.text().clone()) {
                    child.convert_to_error(eco_format!(
                        "duplicate parameter: {}",
                        child.text()
                    ));
                }
            }
            SyntaxKind::Named => {
                let Some(within) = child.children_mut().first_mut() else { return };
                if !used.insert(within.text().clone()) {
                    within.convert_to_error(eco_format!(
                        "duplicate parameter: {}",
                        within.text()
                    ));
                    child.make_erroneous();
                }
            }
            SyntaxKind::Spread => {
                let Some(within) = child.children_mut().last_mut() else { continue };
                if used_spread {
                    child.convert_to_error("only one argument sink is allowed");
                    continue;
                }
                used_spread = true;
                if within.kind() == SyntaxKind::Dots {
                    continue;
                } else if within.kind() != SyntaxKind::Ident {
                    within.convert_to_error(eco_format!(
                        "expected identifier, found {}",
                        within.kind().name(),
                    ));
                    child.make_erroneous();
                    continue;
                }
                if !used.insert(within.text().clone()) {
                    within.convert_to_error(eco_format!(
                        "duplicate parameter: {}",
                        within.text()
                    ));
                    child.make_erroneous();
                }
            }
            SyntaxKind::Array | SyntaxKind::Dict | SyntaxKind::Destructuring => {
                validate_pattern(child.children_mut().iter_mut(), &mut used, false);
                child.convert_to_kind(SyntaxKind::Destructuring);
            }
            SyntaxKind::LeftParen
            | SyntaxKind::RightParen
            | SyntaxKind::Comma
            | SyntaxKind::Underscore => {}
            kind => {
                child.convert_to_error(eco_format!(
                    "expected identifier, named pair or argument sink, found {}",
                    kind.name()
                ));
            }
        }
    }
}

fn validate_args_at(p: &mut Parser, m: Marker) {
    let mut used = HashSet::new();
    for child in p.post_process(m) {
        if child.kind() == SyntaxKind::Named {
            let Some(within) = child.children_mut().first_mut() else { return };
            if !used.insert(within.text().clone()) {
                within.convert_to_error(eco_format!(
                    "duplicate argument: {}",
                    within.text()
                ));
                child.make_erroneous();
            }
        } else if child.kind() == SyntaxKind::Underscore {
            child.convert_to_error("unexpected underscore");
        }
    }
}

fn validate_pattern_at(p: &mut Parser, m: Marker, forbid_expressions: bool) {
    let mut used = HashSet::new();
    validate_pattern(p.post_process(m), &mut used, forbid_expressions);
}

fn validate_pattern<'a>(
    children: impl Iterator<Item = &'a mut SyntaxNode>,
    used: &mut HashSet<EcoString>,
    forbid_expressions: bool,
) {
    let mut used_spread = false;
    for child in children {
        match child.kind() {
            SyntaxKind::Ident => {
                if !used.insert(child.text().clone()) {
                    child.convert_to_error(
                        "at most one binding per identifier is allowed",
                    );
                }
            }
            SyntaxKind::Spread => {
                let Some(within) = child.children_mut().last_mut() else { continue };
                if used_spread {
                    child.convert_to_error("at most one destructuring sink is allowed");
                    continue;
                }
                used_spread = true;

                if within.kind() == SyntaxKind::Dots {
                    continue;
                } else if forbid_expressions && within.kind() != SyntaxKind::Ident {
                    within.convert_to_error(eco_format!(
                        "expected identifier, found {}",
                        within.kind().name(),
                    ));
                    child.make_erroneous();
                    continue;
                }

                if !used.insert(within.text().clone()) {
                    within.convert_to_error(
                        "at most one binding per identifier is allowed",
                    );
                    child.make_erroneous();
                }
            }
            SyntaxKind::Named => {
                let Some(within) = child.children_mut().first_mut() else { return };
                if !used.insert(within.text().clone()) {
                    within.convert_to_error(
                        "at most one binding per identifier is allowed",
                    );
                    child.make_erroneous();
                }

                if forbid_expressions {
                    let Some(within) = child.children_mut().last_mut() else { return };
                    if within.kind() != SyntaxKind::Ident
                        && within.kind() != SyntaxKind::Underscore
                    {
                        within.convert_to_error(eco_format!(
                            "expected identifier, found {}",
                            within.kind().name(),
                        ));
                        child.make_erroneous();
                    }
                }
            }
            SyntaxKind::LeftParen
            | SyntaxKind::RightParen
            | SyntaxKind::Comma
            | SyntaxKind::Underscore => {}
            kind => {
                if forbid_expressions {
                    child.convert_to_error(eco_format!(
                        "expected identifier or destructuring sink, found {}",
                        kind.name()
                    ));
                }
            }
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
    modes: Vec<LexMode>,
    nodes: Vec<SyntaxNode>,
    newline_modes: Vec<NewlineMode>,
    balanced: bool,
}

/// How to proceed with parsing when seeing a newline.
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
            modes: vec![],
            nodes: vec![],
            newline_modes: vec![],
            balanced: true,
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

    #[track_caller]
    fn assert(&mut self, kind: SyntaxKind) {
        assert_eq!(self.current, kind);
        self.eat();
    }

    fn eof(&self) -> bool {
        self.at(SyntaxKind::Eof)
    }

    fn directly_at(&self, kind: SyntaxKind) -> bool {
        self.current == kind && self.prev_end == self.current_start
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

    fn node(&self, m: Marker) -> Option<&SyntaxNode> {
        self.nodes.get(m.0)
    }

    fn node_mut(&mut self, m: Marker) -> Option<&mut SyntaxNode> {
        self.nodes.get_mut(m.0)
    }

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

    fn progress(&self, offset: usize) -> bool {
        offset < self.prev_end
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

    fn eat(&mut self) {
        self.save();
        self.lex();
        self.skip();
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
            while self.nodes.last().map_or(false, |last| last.kind().is_trivia()) {
                self.nodes.pop();
            }

            self.lexer.jump(self.prev_end);
            self.lex();
        }
    }

    fn save(&mut self) {
        let text = self.current_text();
        if self.at(SyntaxKind::Error) {
            let message = self.lexer.take_error().unwrap();
            self.nodes.push(SyntaxNode::error(message, text));
        } else {
            self.nodes.push(SyntaxNode::leaf(self.current, text));
        }

        if self.lexer.mode() == LexMode::Markup || !self.current.is_trivia() {
            self.prev_end = self.current_end();
        }
    }

    fn lex(&mut self) {
        self.current_start = self.lexer.cursor();
        self.current = self.lexer.next();
        if self.lexer.mode() == LexMode::Code
            && self.lexer.newline()
            && match self.newline_modes.last() {
                Some(NewlineMode::Continue) => false,
                Some(NewlineMode::Contextual) => !matches!(
                    self.lexer.clone().next(),
                    SyntaxKind::Else | SyntaxKind::Dot
                ),
                Some(NewlineMode::Stop) => true,
                None => false,
            }
        {
            self.current = SyntaxKind::Eof;
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
            self.expected_found(kind.name(), self.current.name());
        } else {
            self.balanced &= !kind.is_grouping();
            self.expected(kind.name());
        }
        at
    }

    /// Produce an error that the given `thing` was expected.
    fn expected(&mut self, thing: &str) {
        if !self.after_error() {
            self.expected_at(self.before_trivia(), thing);
        }
    }

    /// Produce an error that the given `thing` was expected but another
    /// thing was `found` and consume the next token.
    fn expected_found(&mut self, thing: &str, found: &str) {
        self.trim_errors();
        self.convert_to_error(eco_format!("expected {thing}, found {found}"));
    }

    /// Produce an error that the given `thing` was expected at the position
    /// of the marker `m`.
    fn expected_at(&mut self, m: Marker, thing: &str) {
        let message = eco_format!("expected {}", thing);
        let error = SyntaxNode::error(message, "");
        self.nodes.insert(m.0, error);
    }

    /// Produce an error for the unclosed delimiter `kind` at the position
    /// `open`.
    fn expect_closing_delimiter(&mut self, open: Marker, kind: SyntaxKind) {
        if !self.eat_if(kind) {
            self.nodes[open.0].convert_to_error("unclosed delimiter");
        }
    }

    /// Consume the next token (if any) and produce an error stating that it was
    /// unexpected.
    fn unexpected(&mut self) {
        self.trim_errors();
        self.convert_to_error(eco_format!("unexpected {}", self.current.name()));
    }

    /// Consume the next token and turn it into an error.
    fn convert_to_error(&mut self, message: EcoString) {
        let kind = self.current;
        let offset = self.nodes.len();
        self.eat();
        self.balanced &= !kind.is_grouping();
        if !kind.is_error() {
            self.nodes[offset].convert_to_error(message);
        }
    }

    /// Adds a hint to the last node, if the last node is an error.
    fn hint(&mut self, hint: impl Into<EcoString>) {
        let m = self.before_trivia();
        if m.0 > 0 {
            self.nodes[m.0 - 1].hint(hint);
        }
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
