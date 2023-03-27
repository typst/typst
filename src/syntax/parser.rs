use std::collections::HashSet;
use std::ops::Range;

use ecow::{eco_format, EcoString};
use unicode_math_class::MathClass;

use super::{ast, is_newline, ErrorPos, LexMode, Lexer, SyntaxKind, SyntaxNode};

/// Parse a source file.
pub fn parse(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, 0, LexMode::Markup);
    markup(&mut p, true, 0, |_| false);
    p.finish().into_iter().next().unwrap()
}

/// Parse code directly.
///
/// This is only used for syntax highlighting.
pub fn parse_code(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, 0, LexMode::Code);
    code(&mut p, |_| false);
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
            _ if stop(p.current) => break,
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

        SyntaxKind::Hashtag => embedded_code_expr(p),
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
    p.expect(SyntaxKind::Star);
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
    p.expect(SyntaxKind::Underscore);
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
    p.assert(SyntaxKind::ListMarker);
    let min_indent = p.column(p.prev_end());
    whitespace_line(p);
    markup(p, false, min_indent, |p| p.at(SyntaxKind::RightBracket));
    p.wrap(m, SyntaxKind::ListItem);
}

fn enum_item(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::EnumMarker);
    let min_indent = p.column(p.prev_end());
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
    math(p, |kind| kind == SyntaxKind::Dollar);
    p.expect(SyntaxKind::Dollar);
    p.exit();
    p.wrap(m, SyntaxKind::Equation);
}

fn math(p: &mut Parser, mut stop: impl FnMut(SyntaxKind) -> bool) {
    let m = p.marker();
    while !p.eof() && !stop(p.current()) {
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
        SyntaxKind::Hashtag => embedded_code_expr(p),
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
            if !maybe_delimited(p, true) {
                p.eat();
            }
        }

        SyntaxKind::Linebreak | SyntaxKind::MathAlignPoint => p.eat(),
        SyntaxKind::Escape | SyntaxKind::Str => {
            continuable = true;
            p.eat();
        }

        _ => p.expected("expression"),
    }

    if continuable
        && min_prec < 3
        && p.prev_end() == p.current_start()
        && maybe_delimited(p, false)
    {
        p.wrap(m, SyntaxKind::Math);
    }

    while !p.eof() && !p.at(stop) {
        let Some((kind, stop, assoc, mut prec)) = math_op(p.current()) else {
            break;
        };

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
            math_expr_prec(p, prec, SyntaxKind::Eof);
            math_unparen(p, m3);
        }

        p.wrap(m, kind);
    }
}

fn maybe_delimited(p: &mut Parser, allow_fence: bool) -> bool {
    if allow_fence && math_class(p.current_text()) == Some(MathClass::Fence) {
        math_delimited(p, MathClass::Fence);
        true
    } else if math_class(p.current_text()) == Some(MathClass::Opening) {
        math_delimited(p, MathClass::Closing);
        true
    } else {
        false
    }
}

fn math_delimited(p: &mut Parser, stop: MathClass) {
    let m = p.marker();
    p.eat();
    let m2 = p.marker();
    while !p.eof() && !p.at(SyntaxKind::Dollar) {
        let class = math_class(p.current_text());
        if stop == MathClass::Fence && class == Some(MathClass::Closing) {
            break;
        }

        if class == Some(stop) {
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
            Some((SyntaxKind::MathAttach, SyntaxKind::Hat, ast::Assoc::Right, 3))
        }
        SyntaxKind::Hat => {
            Some((SyntaxKind::MathAttach, SyntaxKind::Underscore, ast::Assoc::Right, 3))
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

fn code(p: &mut Parser, mut stop: impl FnMut(SyntaxKind) -> bool) {
    let m = p.marker();
    while !p.eof() && !stop(p.current()) {
        p.stop_at_newline(true);
        let prev = p.prev_end();
        code_expr(p);
        if p.progress(prev)
            && !p.eof()
            && !stop(p.current())
            && !p.eat_if(SyntaxKind::Semicolon)
        {
            p.expected("semicolon or line break");
        }
        p.unstop();
        if !p.progress(prev) && !p.eof() {
            p.unexpected();
        }
    }
    p.wrap(m, SyntaxKind::Code);
}

fn code_expr(p: &mut Parser) {
    code_expr_prec(p, false, 0)
}

fn embedded_code_expr(p: &mut Parser) {
    p.stop_at_newline(true);
    p.enter(LexMode::Code);
    p.assert(SyntaxKind::Hashtag);
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
    code_expr_prec(p, true, 0);

    // Consume error for things like `#12p` or `#"abc\"`.
    if !p.progress(prev) {
        p.unexpected();
    }

    let semi =
        (stmt || p.directly_at(SyntaxKind::Semicolon)) && p.eat_if(SyntaxKind::Semicolon);

    if stmt && !semi && !p.eof() && !p.at(SyntaxKind::RightBracket) {
        p.expected("semicolon or line break");
    }

    p.exit();
    p.unstop();
}

fn code_expr_prec(p: &mut Parser, atomic: bool, min_prec: usize) {
    let m = p.marker();
    if let (false, Some(op)) = (atomic, ast::UnOp::from_kind(p.current())) {
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
            code_expr_prec(p, false, prec);
            p.wrap(m, SyntaxKind::Binary);
            continue;
        }

        break;
    }
}

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

        SyntaxKind::LeftBrace => code_block(p),
        SyntaxKind::LeftBracket => content_block(p),
        SyntaxKind::LeftParen => with_paren(p),
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
    p.stop_at_newline(false);
    p.assert(SyntaxKind::LeftBrace);
    code(p, |kind| kind == SyntaxKind::RightBrace);
    p.expect(SyntaxKind::RightBrace);
    p.exit();
    p.unstop();
    p.wrap(m, SyntaxKind::CodeBlock);
}

fn content_block(p: &mut Parser) {
    let m = p.marker();
    p.enter(LexMode::Markup);
    p.assert(SyntaxKind::LeftBracket);
    markup(p, true, 0, |p| p.at(SyntaxKind::RightBracket));
    p.expect(SyntaxKind::RightBracket);
    p.exit();
    p.wrap(m, SyntaxKind::ContentBlock);
}

fn with_paren(p: &mut Parser) {
    let m = p.marker();
    let mut kind = collection(p, true);
    if p.at(SyntaxKind::Arrow) {
        validate_params(p, m);
        p.wrap(m, SyntaxKind::Params);
        p.assert(SyntaxKind::Arrow);
        code_expr(p);
        kind = SyntaxKind::Closure;
    }
    match kind {
        SyntaxKind::Array => validate_array(p, m),
        SyntaxKind::Dict => validate_dict(p, m),
        _ => {}
    }
    p.wrap(m, kind);
}

fn collection(p: &mut Parser, keyed: bool) -> SyntaxKind {
    p.stop_at_newline(false);
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
            SyntaxKind::Named | SyntaxKind::Keyed if kind.is_none() => {
                kind = Some(SyntaxKind::Dict);
                parenthesized = false;
            }
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

    p.expect(SyntaxKind::RightParen);
    p.unstop();

    if parenthesized && count == 1 {
        SyntaxKind::Parenthesized
    } else {
        kind.unwrap_or(SyntaxKind::Array)
    }
}

fn item(p: &mut Parser, keyed: bool) -> SyntaxKind {
    let m = p.marker();

    if p.eat_if(SyntaxKind::Dots) {
        code_expr(p);
        p.wrap(m, SyntaxKind::Spread);
        return SyntaxKind::Spread;
    }

    code_expr(p);

    if !p.eat_if(SyntaxKind::Colon) {
        return SyntaxKind::Int;
    }

    code_expr(p);

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
        validate_args(p, m);
    }

    while p.directly_at(SyntaxKind::LeftBracket) {
        content_block(p);
    }

    p.wrap(m, SyntaxKind::Args);
}

fn let_binding(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Let);

    let m2 = p.marker();
    p.expect(SyntaxKind::Ident);

    let closure = p.directly_at(SyntaxKind::LeftParen);
    if closure {
        let m3 = p.marker();
        collection(p, false);
        validate_params(p, m3);
        p.wrap(m3, SyntaxKind::Params);
    }

    let f = if closure { Parser::expect } else { Parser::eat_if };
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
    p.unskip();
    let m2 = p.marker();
    p.skip();

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
    for_pattern(p);
    p.expect(SyntaxKind::In);
    code_expr(p);
    block(p);
    p.wrap(m, SyntaxKind::ForLoop);
}

fn for_pattern(p: &mut Parser) {
    let m = p.marker();
    if p.expect(SyntaxKind::Ident) {
        if p.eat_if(SyntaxKind::Comma) {
            p.expect(SyntaxKind::Ident);
        }
        p.wrap(m, SyntaxKind::ForPattern);
    }
}

fn module_import(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Import);
    code_expr(p);
    if p.eat_if(SyntaxKind::Colon) && !p.eat_if(SyntaxKind::Star) {
        import_items(p);
    }
    p.wrap(m, SyntaxKind::ModuleImport);
}

fn import_items(p: &mut Parser) {
    let m = p.marker();
    while !p.eof() && !p.at(SyntaxKind::Semicolon) {
        if !p.eat_if(SyntaxKind::Ident) {
            p.unexpected();
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

fn validate_array(p: &mut Parser, m: Marker) {
    for child in p.post_process(m) {
        let kind = child.kind();
        if kind == SyntaxKind::Named || kind == SyntaxKind::Keyed {
            child.convert_to_error(eco_format!(
                "expected expression, found {}",
                kind.name()
            ));
        }
    }
}

fn validate_dict(p: &mut Parser, m: Marker) {
    let mut used = HashSet::new();
    for child in p.post_process(m) {
        match child.kind() {
            SyntaxKind::Named | SyntaxKind::Keyed => {
                let Some(first) = child.children_mut().first_mut() else { continue };
                let key = match first.cast::<ast::Str>() {
                    Some(str) => str.get(),
                    None => first.text().clone(),
                };

                if !used.insert(key) {
                    first.convert_to_error("duplicate key");
                    child.make_erroneous();
                }
            }
            SyntaxKind::Spread => {}
            SyntaxKind::LeftParen
            | SyntaxKind::RightParen
            | SyntaxKind::Comma
            | SyntaxKind::Colon => {}
            kind => {
                child.convert_to_error(eco_format!(
                    "expected named or keyed pair, found {}",
                    kind.name()
                ));
            }
        }
    }
}

fn validate_params(p: &mut Parser, m: Marker) {
    let mut used = HashSet::new();
    for child in p.post_process(m) {
        match child.kind() {
            SyntaxKind::Ident => {
                if !used.insert(child.text().clone()) {
                    child.convert_to_error("duplicate parameter");
                }
            }
            SyntaxKind::Named => {
                let Some(within) = child.children_mut().first_mut() else { return };
                if !used.insert(within.text().clone()) {
                    within.convert_to_error("duplicate parameter");
                    child.make_erroneous();
                }
            }
            SyntaxKind::Spread => {
                let Some(within) = child.children_mut().last_mut() else { continue };
                if within.kind() != SyntaxKind::Ident {
                    within.convert_to_error(eco_format!(
                        "expected identifier, found {}",
                        within.kind().name(),
                    ));
                    child.make_erroneous();
                }
            }
            SyntaxKind::LeftParen | SyntaxKind::RightParen | SyntaxKind::Comma => {}
            kind => {
                child.convert_to_error(eco_format!(
                    "expected identifier, named pair or argument sink, found {}",
                    kind.name()
                ));
            }
        }
    }
}

fn validate_args(p: &mut Parser, m: Marker) {
    let mut used = HashSet::new();
    for child in p.post_process(m) {
        if child.kind() == SyntaxKind::Named {
            let Some(within) = child.children_mut().first_mut() else { return };
            if !used.insert(within.text().clone()) {
                within.convert_to_error("duplicate argument");
                child.make_erroneous();
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
    stop_at_newline: Vec<bool>,
    balanced: bool,
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
            stop_at_newline: vec![],
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

    fn eat_if(&mut self, kind: SyntaxKind) -> bool {
        let at = self.at(kind);
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

    fn post_process(&mut self, m: Marker) -> impl Iterator<Item = &mut SyntaxNode> {
        self.nodes[m.0..]
            .iter_mut()
            .filter(|child| !child.kind().is_error() && !child.kind().is_trivia())
    }

    fn wrap(&mut self, m: Marker, kind: SyntaxKind) {
        self.unskip();
        let from = m.0.min(self.nodes.len());
        let children = self.nodes.drain(from..).collect();
        self.nodes.push(SyntaxNode::inner(kind, children));
        self.skip();
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

    fn stop_at_newline(&mut self, stop: bool) {
        self.stop_at_newline.push(stop);
    }

    fn unstop(&mut self) {
        self.unskip();
        self.stop_at_newline.pop();
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
            let (message, pos) = self.lexer.take_error().unwrap();
            self.nodes.push(SyntaxNode::error(message, text, pos));
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
            && self.stop_at_newline.last().copied().unwrap_or(false)
            && !matches!(self.lexer.clone().next(), SyntaxKind::Else | SyntaxKind::Dot)
        {
            self.current = SyntaxKind::Eof;
        }
    }

    fn expect(&mut self, kind: SyntaxKind) -> bool {
        let at = self.at(kind);
        if at {
            self.eat();
        } else {
            self.balanced &= !kind.is_grouping();
            self.expected(kind.name());
        }
        at
    }

    fn expected(&mut self, thing: &str) {
        self.unskip();
        if self
            .nodes
            .last()
            .map_or(true, |child| child.kind() != SyntaxKind::Error)
        {
            let message = eco_format!("expected {}", thing);
            self.nodes.push(SyntaxNode::error(message, "", ErrorPos::Full));
        }
        self.skip();
    }

    fn expected_at(&mut self, m: Marker, thing: &str) {
        let message = eco_format!("expected {}", thing);
        let error = SyntaxNode::error(message, "", ErrorPos::Full);
        self.nodes.insert(m.0, error);
    }

    fn unexpected(&mut self) {
        self.unskip();
        while self
            .nodes
            .last()
            .map_or(false, |child| child.kind() == SyntaxKind::Error && child.len() == 0)
        {
            self.nodes.pop();
        }
        self.skip();

        let kind = self.current;
        let offset = self.nodes.len();
        self.eat();
        self.balanced &= !kind.is_grouping();

        if !kind.is_error() {
            self.nodes[offset]
                .convert_to_error(eco_format!("unexpected {}", kind.name()));
        }
    }
}
