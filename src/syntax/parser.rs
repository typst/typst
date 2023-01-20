use std::collections::HashSet;
use std::ops::Range;

use super::{ast, is_newline, ErrorPos, LexMode, Lexer, SyntaxKind, SyntaxNode};
use crate::util::{format_eco, EcoString};

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
    let m = p.marker();
    code(&mut p, |_| false);
    p.wrap(m, SyntaxKind::CodeBlock);
    p.finish().into_iter().next().unwrap()
}

fn markup(
    p: &mut Parser,
    mut at_start: bool,
    min_indent: usize,
    mut stop: impl FnMut(SyntaxKind) -> bool,
) {
    let m = p.marker();
    while !p.eof() && !stop(p.current) {
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
    mut stop: impl FnMut(SyntaxKind) -> bool,
) -> Option<Vec<SyntaxNode>> {
    let mut p = Parser::new(&text, range.start, LexMode::Markup);
    while !p.eof() && !stop(p.current) && p.current_start() < range.end {
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
        SyntaxKind::Star => strong(p),
        SyntaxKind::Underscore => emph(p),
        SyntaxKind::HeadingMarker if *at_start => heading(p),
        SyntaxKind::ListMarker if *at_start => list_item(p),
        SyntaxKind::EnumMarker if *at_start => enum_item(p),
        SyntaxKind::TermMarker if *at_start => term_item(p),
        SyntaxKind::Dollar => equation(p),

        SyntaxKind::HeadingMarker
        | SyntaxKind::ListMarker
        | SyntaxKind::EnumMarker
        | SyntaxKind::TermMarker
        | SyntaxKind::Colon => p.convert(SyntaxKind::Text),

        SyntaxKind::Ident
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
        | SyntaxKind::Return
        | SyntaxKind::LeftBrace
        | SyntaxKind::LeftBracket => embedded_code_expr(p),

        SyntaxKind::Text
        | SyntaxKind::Linebreak
        | SyntaxKind::Escape
        | SyntaxKind::Shorthand
        | SyntaxKind::Symbol
        | SyntaxKind::SmartQuote
        | SyntaxKind::Raw
        | SyntaxKind::Link
        | SyntaxKind::Label
        | SyntaxKind::Ref => p.eat(),

        SyntaxKind::Space
        | SyntaxKind::Parbreak
        | SyntaxKind::LineComment
        | SyntaxKind::BlockComment => {
            p.eat();
            return;
        }
        _ => {}
    }

    *at_start = false;
}

fn strong(p: &mut Parser) {
    let m = p.marker();
    p.expect(SyntaxKind::Star);
    markup(p, false, 0, |kind| {
        kind == SyntaxKind::Star
            || kind == SyntaxKind::Parbreak
            || kind == SyntaxKind::RightBracket
    });
    p.expect(SyntaxKind::Star);
    p.wrap(m, SyntaxKind::Strong);
}

fn emph(p: &mut Parser) {
    let m = p.marker();
    p.expect(SyntaxKind::Underscore);
    markup(p, false, 0, |kind| {
        kind == SyntaxKind::Underscore
            || kind == SyntaxKind::Parbreak
            || kind == SyntaxKind::RightBracket
    });
    p.expect(SyntaxKind::Underscore);
    p.wrap(m, SyntaxKind::Emph);
}

fn heading(p: &mut Parser) {
    let m = p.marker();
    p.expect(SyntaxKind::HeadingMarker);
    whitespace(p);
    markup(p, false, usize::MAX, |kind| {
        kind == SyntaxKind::Label || kind == SyntaxKind::RightBracket
    });
    p.wrap(m, SyntaxKind::Heading);
}

fn list_item(p: &mut Parser) {
    let m = p.marker();
    p.expect(SyntaxKind::ListMarker);
    let min_indent = p.column(p.prev_end());
    whitespace(p);
    markup(p, false, min_indent, |kind| kind == SyntaxKind::RightBracket);
    p.wrap(m, SyntaxKind::ListItem);
}

fn enum_item(p: &mut Parser) {
    let m = p.marker();
    p.expect(SyntaxKind::EnumMarker);
    let min_indent = p.column(p.prev_end());
    whitespace(p);
    markup(p, false, min_indent, |kind| kind == SyntaxKind::RightBracket);
    p.wrap(m, SyntaxKind::EnumItem);
}

fn term_item(p: &mut Parser) {
    let m = p.marker();
    p.expect(SyntaxKind::TermMarker);
    let min_indent = p.column(p.prev_end());
    whitespace(p);
    markup(p, false, usize::MAX, |kind| {
        kind == SyntaxKind::Colon || kind == SyntaxKind::RightBracket
    });
    p.expect(SyntaxKind::Colon);
    whitespace(p);
    markup(p, false, min_indent, |kind| kind == SyntaxKind::RightBracket);
    p.wrap(m, SyntaxKind::TermItem);
}

fn whitespace(p: &mut Parser) {
    while p.current().is_trivia() {
        p.eat();
    }
}

fn equation(p: &mut Parser) {
    let m = p.marker();
    p.enter(LexMode::Math);
    p.expect(SyntaxKind::Dollar);
    math(p, |kind| kind == SyntaxKind::Dollar);
    p.expect(SyntaxKind::Dollar);
    p.exit();
    p.wrap(m, SyntaxKind::Math);
}

fn math(p: &mut Parser, mut stop: impl FnMut(SyntaxKind) -> bool) {
    while !p.eof() && !stop(p.current()) {
        let prev = p.prev_end();
        math_expr(p);
        if !p.progress(prev) {
            p.unexpected();
        }
    }
}

fn math_expr(p: &mut Parser) {
    math_expr_prec(p, 0, SyntaxKind::Eof)
}

fn math_expr_prec(p: &mut Parser, min_prec: usize, stop: SyntaxKind) {
    let m = p.marker();
    match p.current() {
        SyntaxKind::Ident => {
            p.eat();
            if p.directly_at(SyntaxKind::Atom) && p.current_text() == "(" {
                math_args(p);
                p.wrap(m, SyntaxKind::FuncCall);
            }
        }

        SyntaxKind::Atom => match p.current_text() {
            "(" => math_delimited(p, ")"),
            "{" => math_delimited(p, "}"),
            "[" => math_delimited(p, "]"),
            _ => p.eat(),
        },

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
        | SyntaxKind::Return
        | SyntaxKind::LeftBrace
        | SyntaxKind::LeftBracket => embedded_code_expr(p),

        SyntaxKind::Linebreak
        | SyntaxKind::Escape
        | SyntaxKind::Shorthand
        | SyntaxKind::Symbol
        | SyntaxKind::AlignPoint
        | SyntaxKind::Str => p.eat(),

        _ => return,
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

        p.eat();
        math_expr_prec(p, prec, stop);
        if p.eat_if(SyntaxKind::Underscore) || p.eat_if(SyntaxKind::Hat) {
            math_expr_prec(p, prec, SyntaxKind::Eof);
        }

        p.wrap(m, kind);
    }
}

fn math_delimited(p: &mut Parser, closing: &str) {
    let m = p.marker();
    p.expect(SyntaxKind::Atom);
    while !p.eof()
        && !p.at(SyntaxKind::Dollar)
        && (!p.at(SyntaxKind::Atom) || p.current_text() != closing)
    {
        let prev = p.prev_end();
        math_expr(p);
        if !p.progress(prev) {
            p.unexpected();
        }
    }
    p.expect(SyntaxKind::Atom);
    p.wrap(m, SyntaxKind::Math);
}

fn math_op(kind: SyntaxKind) -> Option<(SyntaxKind, SyntaxKind, ast::Assoc, usize)> {
    match kind {
        SyntaxKind::Underscore => {
            Some((SyntaxKind::Script, SyntaxKind::Hat, ast::Assoc::Right, 2))
        }
        SyntaxKind::Hat => {
            Some((SyntaxKind::Script, SyntaxKind::Underscore, ast::Assoc::Right, 2))
        }
        SyntaxKind::Slash => {
            Some((SyntaxKind::Frac, SyntaxKind::Eof, ast::Assoc::Left, 1))
        }
        _ => None,
    }
}

fn math_args(p: &mut Parser) {
    p.expect(SyntaxKind::Atom);
    let m = p.marker();
    let mut m2 = p.marker();
    while !p.eof() {
        match p.current_text() {
            ")" => break,
            "," => {
                p.wrap(m2, SyntaxKind::Math);
                p.convert(SyntaxKind::Comma);
                m2 = p.marker();
                continue;
            }
            _ => {}
        }

        let prev = p.prev_end();
        math_expr(p);
        if !p.progress(prev) {
            p.unexpected();
        }
    }
    if m2 != p.marker() {
        p.wrap(m2, SyntaxKind::Math);
    }
    p.wrap(m, SyntaxKind::Args);
    p.expect(SyntaxKind::Atom);
}

fn code(p: &mut Parser, mut stop: impl FnMut(SyntaxKind) -> bool) {
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
}

fn code_expr(p: &mut Parser) {
    code_expr_prec(p, false, 0)
}

fn embedded_code_expr(p: &mut Parser) {
    let stmt = matches!(
        p.current(),
        SyntaxKind::Let
            | SyntaxKind::Set
            | SyntaxKind::Show
            | SyntaxKind::Import
            | SyntaxKind::Include
    );

    p.stop_at_newline(true);
    p.enter(LexMode::Code);
    code_expr_prec(p, true, 0);
    let semi = p.eat_if(SyntaxKind::Semicolon);
    if stmt && !semi && !p.eof() && !p.at(SyntaxKind::RightBracket) {
        p.expected("semicolon or line break");
    }
    p.exit();
    p.unstop();
}

fn code_expr_prec(p: &mut Parser, atomic: bool, min_prec: usize) {
    let m = p.marker();
    if let Some(op) = ast::UnOp::from_kind(p.current()) {
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

        if atomic {
            break;
        }

        if p.eat_if(SyntaxKind::Dot) {
            p.expect(SyntaxKind::Ident);
            if p.directly_at(SyntaxKind::LeftParen)
                || p.directly_at(SyntaxKind::LeftBracket)
            {
                args(p);
                p.wrap(m, SyntaxKind::MethodCall);
            } else {
                p.wrap(m, SyntaxKind::FieldAccess)
            }
            continue;
        }

        let binop = if p.eat_if(SyntaxKind::Not) {
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
                p.expect(SyntaxKind::Arrow);
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
    let mut p = Parser::new(&text, range.start, LexMode::Code);
    assert!(p.at(SyntaxKind::LeftBracket) || p.at(SyntaxKind::LeftBrace));
    block(&mut p);
    (p.balanced && p.prev_end() == range.end)
        .then(|| p.finish().into_iter().next().unwrap())
}

fn code_block(p: &mut Parser) {
    let m = p.marker();
    p.enter(LexMode::Code);
    p.stop_at_newline(false);
    p.expect(SyntaxKind::LeftBrace);
    code(p, |kind| kind == SyntaxKind::RightBrace);
    p.expect(SyntaxKind::RightBrace);
    p.exit();
    p.unstop();
    p.wrap(m, SyntaxKind::CodeBlock);
}

fn content_block(p: &mut Parser) {
    let m = p.marker();
    p.enter(LexMode::Markup);
    p.expect(SyntaxKind::LeftBracket);
    markup(p, true, 0, |kind| kind == SyntaxKind::RightBracket);
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
        p.expect(SyntaxKind::Arrow);
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
    p.expect(SyntaxKind::LeftParen);

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
            for child in p.post_process(m).next() {
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
    p.expect(SyntaxKind::Ident);
    args(p);
    if p.eat_if(SyntaxKind::If) {
        code_expr(p);
    }
    p.wrap(m, SyntaxKind::SetRule);
}

fn show_rule(p: &mut Parser) {
    let m = p.marker();
    p.assert(SyntaxKind::Show);
    code_expr(p);
    if p.eat_if(SyntaxKind::Colon) {
        code_expr(p);
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
            child.convert_to_error(format_eco!(
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
                child.convert_to_error(format_eco!(
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
                    within.convert_to_error(format_eco!(
                        "expected identifier, found {}",
                        within.kind().name(),
                    ));
                    child.make_erroneous();
                }
            }
            SyntaxKind::LeftParen | SyntaxKind::RightParen | SyntaxKind::Comma => {}
            kind => {
                child.convert_to_error(format_eco!(
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
        if self.at(SyntaxKind::Error) {
            let (message, pos) = self.lexer.take_error().unwrap();
            let len = self.current_end() - self.current_start;
            self.nodes.push(SyntaxNode::error(message, pos, len));
        } else {
            let text = self.current_text();
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
            let message = format_eco!("expected {}", thing);
            self.nodes.push(SyntaxNode::error(message, ErrorPos::Full, 0));
        }
        self.skip();
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
                .convert_to_error(format_eco!("unexpected {}", kind.name()));
        }
    }
}
