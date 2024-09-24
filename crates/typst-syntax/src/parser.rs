use std::collections::{HashMap, HashSet};
use std::mem;
use std::ops::{Index, IndexMut, Range};

use ecow::{eco_format, EcoString};
use unicode_math_class::MathClass;

use crate::set::{syntax_set, SyntaxSet};
use crate::{ast, set, LexMode, Lexer, SyntaxError, SyntaxKind, SyntaxNode};

/// Parses a source file.
pub fn parse(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, 0, LexMode::Markup);
    markup(&mut p, true, None);
    p.finish_into(SyntaxKind::Markup)
}

/// Parses top-level code.
pub fn parse_code(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, 0, LexMode::Code);
    code(&mut p, |_| false);
    p.finish_into(SyntaxKind::Code)
}

/// Parses top-level math.
pub fn parse_math(text: &str) -> SyntaxNode {
    let mut p = Parser::new(text, 0, LexMode::Math);
    math(&mut p, |_| false);
    p.finish_into(SyntaxKind::Math)
}

/// Parses the contents of a file or content block.
fn markup(p: &mut Parser, mut at_start: bool, stop: Option<SyntaxKind>) {
    let mut nesting: usize = 0;
    while !p.end() {
        match p.current_kind() {
            SyntaxKind::LeftBracket => nesting += 1,
            SyntaxKind::RightBracket if nesting > 0 => nesting -= 1,
            SyntaxKind::RightBracket => break,
            kind if stop == Some(kind) => break,
            _ => {}
        }
        markup_expr(p, &mut at_start);
    }
}

/// Reparses a subsection of markup incrementally.
pub(super) fn reparse_markup(
    text: &str,
    range: Range<usize>,
    at_start: &mut bool,
    nesting: &mut usize,
) -> Option<Vec<SyntaxNode>> {
    dbg!(&text[range.clone()]);
    let mut p = Parser::new(text, range.start, LexMode::Markup);
    while !p.end() && p.current_start() < range.end {
        match p.current_kind() {
            SyntaxKind::LeftBracket => *nesting += 1,
            SyntaxKind::RightBracket if *nesting > 0 => *nesting -= 1,
            SyntaxKind::RightBracket => break,
            _ => {}
        }
        markup_expr(&mut p, at_start);
    }
    (p.balanced && p.current_start() == range.end).then_some(p.nodes)
}

/// Parses a single markup expression: This includes markup elements like
/// spaces, text, and headings, and embedded code expressions.
fn markup_expr(p: &mut Parser, at_start: &mut bool) {
    match p.current_kind() {
        kind if syntax_set!(
            Text, Linebreak, Escape, Shorthand, SmartQuote, Link, Label,
        )
        .contains(kind) =>
        {
            p.eat()
        }

        SyntaxKind::Raw => p.eat(), // Raw is handled entirely in the lexer.

        SyntaxKind::Hash => embedded_code_expr(p),
        SyntaxKind::Star => strong(p),
        SyntaxKind::Underscore => emph(p),
        SyntaxKind::HeadingMarker if *at_start => heading(p),
        SyntaxKind::ListMarker if *at_start => list_item(p),
        SyntaxKind::EnumMarker if *at_start => enum_item(p),
        SyntaxKind::TermMarker if *at_start => term_item(p),
        SyntaxKind::RefMarker => reference(p),
        SyntaxKind::Dollar => equation(p),

        kind if syntax_set!(
            LeftBracket,
            RightBracket,
            HeadingMarker,
            ListMarker,
            EnumMarker,
            TermMarker,
            Colon,
        )
        .contains(kind) =>
        {
            p.convert_and_eat(SyntaxKind::Text)
        }

        _ => p.unexpected(),
    }

    *at_start = p.newline();
}

/// Parse a set of delimiters with common options.
#[inline]
#[allow(clippy::too_many_arguments)]
fn delims(
    p: &mut Parser,
    open: SyntaxKind,
    close: Option<SyntaxKind>,
    wrap_inner: SyntaxKind,
    mode: NewlineMode,
    keep_trivia: bool,
    func: impl FnOnce(&mut Parser),
    wrap_outer: SyntaxKind,
) {
    let m_outer = p.marker();
    p.enter_newline_mode(mode, |p| {
        p.assert(open);
        let m_inner = if keep_trivia { p.before_trivia() } else { p.marker() };
        func(p);
        if keep_trivia {
            p.flush_trivia();
        }
        p.wrap(m_inner, wrap_inner);
        if let Some(closing) = close {
            p.expect_closing_delimiter(m_outer, closing);
        }
    });
    p.wrap(m_outer, wrap_outer);
}

/// Parses strong content: `*Strong*`.
fn strong(p: &mut Parser) {
    delims(
        p,
        SyntaxKind::Star,
        Some(SyntaxKind::Star),
        SyntaxKind::Markup,
        NewlineMode::NoParBreak,
        true,
        |p| markup(p, false, Some(SyntaxKind::Star)),
        SyntaxKind::Strong,
    );
}

/// Parses emphasized content: `_Emphasized_`.
fn emph(p: &mut Parser) {
    delims(
        p,
        SyntaxKind::Underscore,
        Some(SyntaxKind::Underscore),
        SyntaxKind::Markup,
        NewlineMode::NoParBreak,
        true,
        |p| markup(p, false, Some(SyntaxKind::Underscore)),
        SyntaxKind::Emph,
    );
}

/// Parses a section heading: `= Introduction`.
fn heading(p: &mut Parser) {
    delims(
        p,
        SyntaxKind::HeadingMarker,
        None,
        SyntaxKind::Markup,
        NewlineMode::Stop,
        false,
        |p| markup(p, false, Some(SyntaxKind::Label)),
        SyntaxKind::Heading,
    );
}

/// Parses an item in a bullet list: `- ...`.
fn list_item(p: &mut Parser) {
    delims(
        p,
        SyntaxKind::ListMarker,
        None,
        SyntaxKind::Markup,
        NewlineMode::Indented(p.next_indent()),
        false,
        |p| markup(p, false, None),
        SyntaxKind::ListItem,
    );
}

/// Parses an item in an enumeration (numbered list): `+ ...` or `1. ...`.
fn enum_item(p: &mut Parser) {
    delims(
        p,
        SyntaxKind::EnumMarker,
        None,
        SyntaxKind::Markup,
        NewlineMode::Indented(p.next_indent()),
        false,
        |p| markup(p, false, None),
        SyntaxKind::EnumItem,
    );
}

/// Parses an item in a term list: `/ Term: Details`.
fn term_item(p: &mut Parser) {
    let m = p.marker();
    let min_indent = p.next_indent();
    p.enter_newline_mode(NewlineMode::Stop, |p| {
        p.assert(SyntaxKind::TermMarker);
        let m = p.marker();
        markup(p, false, Some(SyntaxKind::Colon));
        p.wrap(m, SyntaxKind::Markup);
    });
    p.enter_newline_mode(NewlineMode::Indented(min_indent), |p| {
        p.expect(SyntaxKind::Colon);
        let m = p.marker();
        markup(p, false, None);
        p.wrap(m, SyntaxKind::Markup);
    });
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

/// Parses a mathematical equation: `$x$`, `$ x^2 $`.
fn equation(p: &mut Parser) {
    p.enter_mode(LexMode::Math, |p| {
        delims(
            p,
            SyntaxKind::Dollar,
            Some(SyntaxKind::Dollar),
            SyntaxKind::Math,
            NewlineMode::Continue,
            false,
            |p| math(p, |p| p.at(SyntaxKind::Dollar)),
            SyntaxKind::Equation,
        );
    });
}

/// Parses the contents of a mathematical equation: `x^2 + 1`.
fn math(p: &mut Parser, mut stop: impl FnMut(&Parser) -> bool) {
    while !p.end() && !stop(p) {
        if p.at_set(set::MATH_EXPR) {
            math_expr(p);
        } else {
            p.unexpected();
        }
    }
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
    match p.current_kind() {
        SyntaxKind::Hash => embedded_code_expr(p),
        // The lexer manages creating full FieldAccess SyntaxNodes if needed
        SyntaxKind::MathIdent | SyntaxKind::FieldAccess => {
            continuable = true;
            p.eat();
            // Parse a function call for an identifier or field access.
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

    if continuable && min_prec < 3 && !p.had_trivia() && maybe_delimited(p) {
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

        let Some((kind, stop, assoc, mut prec)) = math_op(p.current_kind()) else {
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
    p.convert_and_eat(SyntaxKind::LeftParen);

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
            p.convert_and_eat(SyntaxKind::Ident);
            p.convert_and_eat(SyntaxKind::Colon);
            named = Some(arg);
            arg = p.marker();
            array = p.marker();
        }

        match p.current_text() {
            ")" => break,
            ";" => {
                maybe_wrap_in_math(p, arg, named);
                p.wrap(array, SyntaxKind::Array);
                p.convert_and_eat(SyntaxKind::Semicolon);
                array = p.marker();
                arg = p.marker();
                namable = true;
                named = None;
                has_arrays = true;
                continue;
            }
            "," => {
                maybe_wrap_in_math(p, arg, named);
                p.convert_and_eat(SyntaxKind::Comma);
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
        p.convert_and_eat(SyntaxKind::RightParen);
    } else {
        p.expected("closing paren");
        p.balanced = false;
    }

    p.wrap(m, SyntaxKind::Args);
}

/// Wrap math function arguments in a "Math" SyntaxKind to combine adjacent
/// expressions or create blank content.
///
/// We don't wrap when `exprs == 1`, as there is only one expression, so the
/// grouping isn't needed, and this would change the type of the expression from
/// potentially non-content to content.
///
/// Note that `exprs` might be 0 if we have whitespace or trivia before a comma
/// i.e. `mat(; ,)` or `sin(x, , , ,)`. This would create an empty Math element
/// before that trivia if we called `p.wrap()` -- breaking the expected AST for
/// 2-d arguments -- so we instead manually flush the trivia.
fn maybe_wrap_in_math(p: &mut Parser, arg: Marker, named: Option<Marker>) {
    let exprs = p.post_process(arg).filter(|node| node.is::<ast::Expr>()).count();
    if exprs != 1 {
        // Convert 0 exprs into a blank math element (so empty arguments are
        // allowed). Convert 2+ exprs into a math element (so they become a
        // joined sequence).
        p.flush_trivia();
        p.wrap(arg, SyntaxKind::Math);
    }

    if let Some(m) = named {
        p.wrap(m, SyntaxKind::Named);
    }
}

/// Parses a sequence of code expressions.
fn code(p: &mut Parser, mut stop: impl FnMut(&Parser) -> bool) {
    if p.end() || stop(p) {
        return;
    }
    loop {
        if !p.at_set(set::CODE_EXPR) {
            p.unexpected();
            return;
        }
        p.enter_newline_mode(NewlineMode::MaybeContinue(syntax_set!(Else, Dot)), |p| {
            code_expr(p)
        });
        if p.end() || stop(p) {
            return;
        }
        if !p.eat_if(SyntaxKind::Semicolon) && !p.newline() {
            p.expected("semicolon or line break");
            if p.at(SyntaxKind::Label) {
                p.hint("labels can only be applied in markup mode");
                p.hint("try wrapping your code in a markup block (`[ ]`)");
            }
        }
    }
}

/// Parses a single code expression.
fn code_expr(p: &mut Parser) {
    code_expr_prec(p, false, 0)
}

/// Parses a code expression embedded in markup or math.
fn embedded_code_expr(p: &mut Parser) {
    p.enter_mode(LexMode::Code, |p| {
        p.enter_newline_mode(NewlineMode::Stop, |p| {
            p.assert(SyntaxKind::Hash);
            if p.had_trivia() {
                // Embedded code expressions must not have trivia immediately after
                // the hash. Ex:`#/**/var`, `# var` are both invalid.
                p.expected("expression");
                return;
            }
            let stmt = p.at_set(syntax_set!(Let, Set, Show, Import, Include, Return));
            let at = p.at_set(set::ATOMIC_CODE_EXPR);
            code_expr_prec(p, true, 0);

            // Consume error for things like `#12p` or `#"abc\"`.#
            if !at && !p.end() {
                p.unexpected();
            }

            // Eat semicolons following statements even if they have spaces.
            // Eats the semicolon in `#let x = 5 ;` and `#();`, but not `#() ;`
            let semi = (stmt || p.directly_at(SyntaxKind::Semicolon))
                && p.eat_if(SyntaxKind::Semicolon);

            // Parsed statements must end in a newline, semicolon, or bracket.
            // Ex: `#let x = 5 hello` is an error, but `#let x = 5]hello` isn't.
            if stmt && !semi && !p.end() && !p.at(SyntaxKind::RightBracket) {
                p.expected("semicolon or line break");
            }
        })
    });
}

/// Parses a code expression with at least the given precedence.
fn code_expr_prec(p: &mut Parser, atomic: bool, min_prec: usize) {
    let m = p.marker();
    if !atomic && p.at_set(set::UNARY_OP) {
        let op = ast::UnOp::from_kind(p.current_kind()).unwrap();
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

        let at_field_or_method = p.directly_at(SyntaxKind::Dot)
            && p.lexer.clone().next().0 == SyntaxKind::Ident;

        if atomic && !at_field_or_method {
            break;
        }

        if p.eat_if(SyntaxKind::Dot) {
            p.expect(SyntaxKind::Ident);
            p.wrap(m, SyntaxKind::FieldAccess);
            continue;
        }

        let binop = if p.at_set(set::BINARY_OP) {
            ast::BinOp::from_kind(p.current_kind())
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
    match p.current_kind() {
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

        SyntaxKind::Raw => p.eat(), // Raw is handled entirely in the lexer.

        kind if syntax_set!(None, Auto, Int, Float, Bool, Numeric, Str, Label)
            .contains(kind) =>
        {
            p.eat()
        }

        _ => p.expected("expression"),
    }
}

/// Parses a content or code block.
fn block(p: &mut Parser) {
    match p.current_kind() {
        SyntaxKind::LeftBracket => content_block(p),
        SyntaxKind::LeftBrace => code_block(p),
        _ => p.expected("block"),
    }
}

/// Reparses a full content or code block.
pub(super) fn reparse_block(text: &str, range: Range<usize>) -> Option<Vec<SyntaxNode>> {
    // dbg!(&text[range.clone()]);
    let mut p = Parser::new(text, range.start, LexMode::Code);
    assert!(p.at(SyntaxKind::LeftBracket) || p.at(SyntaxKind::LeftBrace));
    block(&mut p);
    p.nodes.truncate(p.nodes.len() - p.current.n_trivia);
    (p.balanced && p.prev_end() == range.end).then_some(p.nodes)
}

/// Parses a code block: `{ let x = 1; x + 2 }`.
fn code_block(p: &mut Parser) {
    delims(
        p,
        SyntaxKind::LeftBrace,
        Some(SyntaxKind::RightBrace),
        SyntaxKind::Code,
        NewlineMode::Continue,
        false,
        |p| code(p, |p| p.at_set(syntax_set!(RightBrace, RightBracket, RightParen))),
        SyntaxKind::CodeBlock,
    );
}

/// Parses a content block: `[*Hi* there!]`.
fn content_block(p: &mut Parser) {
    p.enter_mode(LexMode::Markup, |p| {
        delims(
            p,
            SyntaxKind::LeftBracket,
            Some(SyntaxKind::RightBracket),
            SyntaxKind::Markup,
            NewlineMode::Continue,
            true,
            |p| markup(p, true, Some(SyntaxKind::RightBracket)),
            SyntaxKind::ContentBlock,
        );
    });
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
            p.enter_newline_mode(NewlineMode::Continue, |p| {
                let m1 = p.marker();
                p.assert(SyntaxKind::LeftParen);
                import_items(p);
                p.expect_closing_delimiter(m1, SyntaxKind::RightParen);
            });
        } else if !p.eat_if(SyntaxKind::Star) {
            import_items(p);
        }
    }

    p.wrap(m, SyntaxKind::ModuleImport);
}

/// Parses items to import from a module: `a, b, c`.
fn import_items(p: &mut Parser) {
    let m = p.marker();
    while !p.current_kind().is_terminator() {
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

        if !p.current_kind().is_terminator() {
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
    let mut state = GroupState {
        count: 0,
        maybe_just_parens: true,
        kind: None,
        seen: HashSet::new(),
    };
    p.enter_newline_mode(NewlineMode::Continue, |p| {
        p.assert(SyntaxKind::LeftParen);

        if p.eat_if(SyntaxKind::Colon) {
            state.kind = Some(SyntaxKind::Dict);
            state.maybe_just_parens = false;
        }

        while !p.current_kind().is_terminator() {
            if !p.at_set(set::ARRAY_OR_DICT_ITEM) {
                p.unexpected();
                continue;
            }

            array_or_dict_item(p, &mut state);
            state.count += 1;

            if !p.current_kind().is_terminator() && p.expect(SyntaxKind::Comma) {
                state.maybe_just_parens = false;
            }
        }

        p.expect_closing_delimiter(m, SyntaxKind::RightParen);
    });

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
        p.enter_newline_mode(NewlineMode::Continue, |p| {
            p.assert(SyntaxKind::LeftParen);

            let mut seen = HashSet::new();
            while !p.current_kind().is_terminator() {
                if !p.at_set(set::ARG) {
                    p.unexpected();
                    continue;
                }

                arg(p, &mut seen);

                if !p.current_kind().is_terminator() {
                    p.expect(SyntaxKind::Comma);
                }
            }
            p.expect_closing_delimiter(m2, SyntaxKind::RightParen);
        });
    }

    while p.directly_at(SyntaxKind::LeftBracket) {
        content_block(p);
    }

    p.wrap(m, SyntaxKind::Args);
}

/// Parses a single argument in an argument list.
fn arg(p: &mut Parser, seen: &mut HashSet<EcoString>) {
    let m = p.marker();

    // Parses a spread argument: `..args`.
    if p.eat_if(SyntaxKind::Dots) {
        code_expr(p);
        p.wrap(m, SyntaxKind::Spread);
        return;
    }

    // Parses a normal positional argument or an argument name.
    let was_at_expr = p.at_set(set::CODE_EXPR);
    let text: EcoString = p.current_text().into();
    code_expr(p);

    // Parses a named argument: `thickness: 12pt`.
    if p.eat_if(SyntaxKind::Colon) {
        // Recover from bad argument name.
        if was_at_expr {
            if p[m].kind() != SyntaxKind::Ident {
                p[m].expected("identifier");
            } else if !seen.insert(text.clone()) {
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

    p.enter_newline_mode(NewlineMode::Continue, |p| {
        p.assert(SyntaxKind::LeftParen);

        let mut seen = HashSet::<EcoString>::new();
        let mut sink = false;

        while !p.current_kind().is_terminator() {
            if !p.at_set(set::PARAM) {
                p.unexpected();
                continue;
            }

            param(p, &mut seen, &mut sink);

            if !p.current_kind().is_terminator() {
                p.expect(SyntaxKind::Comma);
            }
        }

        p.expect_closing_delimiter(m, SyntaxKind::RightParen);
    });
    p.wrap(m, SyntaxKind::Params);
}

/// Parses a single parameter in a parameter list.
fn param(p: &mut Parser, seen: &mut HashSet<EcoString>, sink: &mut bool) {
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
    seen: &mut HashSet<EcoString>,
    dupe: Option<&'s str>,
) {
    match p.current_kind() {
        SyntaxKind::Underscore => p.eat(),
        SyntaxKind::LeftParen => destructuring_or_parenthesized(p, reassignment, seen),
        _ => pattern_leaf(p, reassignment, seen, dupe),
    }
}

/// Parses a destructuring pattern or just a parenthesized pattern.
fn destructuring_or_parenthesized(
    p: &mut Parser,
    reassignment: bool,
    seen: &mut HashSet<EcoString>,
) {
    let mut sink = false;
    let mut count = 0;
    let mut maybe_just_parens = true;

    let m = p.marker();
    p.enter_newline_mode(NewlineMode::Continue, |p| {
        p.assert(SyntaxKind::LeftParen);

        while !p.current_kind().is_terminator() {
            if !p.at_set(set::DESTRUCTURING_ITEM) {
                p.unexpected();
                continue;
            }

            destructuring_item(p, reassignment, seen, &mut maybe_just_parens, &mut sink);
            count += 1;

            if !p.current_kind().is_terminator() && p.expect(SyntaxKind::Comma) {
                maybe_just_parens = false;
            }
        }

        p.expect_closing_delimiter(m, SyntaxKind::RightParen);
    });

    if maybe_just_parens && count == 1 && !sink {
        p.wrap(m, SyntaxKind::Parenthesized);
    } else {
        p.wrap(m, SyntaxKind::Destructuring);
    }
}

/// Parses an item in a destructuring pattern.
fn destructuring_item(
    p: &mut Parser,
    reassignment: bool,
    seen: &mut HashSet<EcoString>,
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
    seen: &mut HashSet<EcoString>,
    dupe: Option<&'s str>,
) {
    if p.current_kind().is_keyword() {
        p.current.node.expected("pattern");
        p.eat();
        return;
    } else if !p.at_set(set::PATTERN_LEAF) {
        p.expected("pattern");
        return;
    }

    let m = p.marker();
    let text: EcoString = p.current_text().into();

    // We parse an atomic expression even though we only want an identifier for
    // better error recovery. We can mark the whole expression as unexpected
    // instead of going through its pieces one by one.
    code_expr_prec(p, true, 0);

    if !reassignment {
        let node = &mut p[m];
        if node.kind() == SyntaxKind::Ident {
            if !seen.insert(text.clone()) {
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

/// Manages parsing a stream of tokens into a tree of SyntaxNodes.
///
/// The implementation presents an interface that investigates a `current` token
/// and can take one of the following actions:
/// 1. Eat a token, placing `current` into the `nodes` vector as a `LeafNode`
///    and preparing a new `current` by calling into the lexer.
/// 2. Wrap nodes from a marker to the end of `nodes` (excluding `current`) into
///    an `InnerNode` of a specific SyntaxKind.
/// 3. Produce or convert nodes into an `ErrorNode` when something expected is
///    missing or something unexpected is found.
///
/// Overall the Parser produces a nested tree of SyntaxNodes as a *Concrete*
/// Syntax Tree. The raw Concrete Syntax Tree should contain the entire source
/// text, and is used as-is for e.g. syntax highlighting and IDE features. In
/// `ast.rs` the CST is interpreted as a lazy view over an *Abstract* Syntax
/// Tree. The AST module skips over irrelevant tokens -- whitespace, comments,
/// code parens, commas in function args, etc. -- as it iterates through the
/// tree.
struct Parser<'s> {
    /// The source text shared with the lexer.
    text: &'s str,
    /// A lexer over the source text with multiple modes. Generates our core
    /// SyntaxNodes.
    lexer: Lexer<'s>,
    mode: NewlineMode,
    /// The current token being evaluated, not yet present in `nodes`. This acts
    /// like a single item of lookahead for the parser.
    ///
    /// When wrapping this node is *not* included in the wrapped element.
    current: Token,
    /// Whether the parser is balanced over open/close delimiters when not
    /// actively matching delimiters. This only ever transitions from true to
    /// false.
    balanced: bool,
    /// Nodes representing the concrete syntax tree of previously parsed text.
    /// Does include previous trivia nodes, but does not include `current`.
    nodes: Vec<SyntaxNode>,
    /// Parser checkpoints for a given text index. Used for efficient parser
    /// backtracking similar to packrat parsing. See comments above in
    /// [`expr_with_paren`].
    memo: HashMap<usize, (Range<usize>, Checkpoint<'s>)>,
    /// The stored parse results at each checkpoint.
    memo_arena: Vec<SyntaxNode>,
}

/// A single token returned from the lexer with a cached SyntaxKind and a record
/// of previous trivia.
#[derive(Clone)]
struct Token {
    /// A SyntaxNode returned from the lexer. This should never be trivia.
    node: SyntaxNode,
    /// The SyntaxKind of `node`, separated from node to allow substituting a
    /// fake End node.
    kind: SyntaxKind,
    /// Number of trivia nodes before this token.
    n_trivia: usize,
    /// Whether the token's leading trivia contained a newline.
    had_newline: Option<NewlineInfo>,
    /// Offset into `text` of the previous node's end.
    prev_end: usize,
}

/// Extra token info from a call to Lexer::next().
pub enum TokenType {
    /// Normal tokens (includes errors).
    Normal,
    /// Comments or whitespace without newlines
    Trivia,
    /// Whitespace with a newline.
    Newline { column: u32, parbreak: bool },
}

/// Additional information about whitespace with newlines.
#[derive(Debug, Clone, Copy)]
pub struct NewlineInfo {
    pub column: u32,
    pub parbreak: bool,
}

/// How to proceed with parsing when seeing a newline.
///
/// This enum causes the parser to emit false `SyntaxKind::End` tokens by
/// converting the current tokens `kind`, but leaving the node's kind alone.
/// This simplifies parsing expressions that end arbitrarily, since the parser
/// will think it's done when newlines interrupt expressions.
#[derive(Clone, Copy)]
enum NewlineMode {
    /// Never emit a false `End`.
    Continue,
    /// Emit a false `End` at any newline.
    Stop,
    /// Emit a false `End` only if there is no continuation with `else` or `.`.
    /// Note that a continuation might come after an arbitrary amount of trivia.
    MaybeContinue(SyntaxSet),
    /// Emit a false `End` if we aren't at a minimum indent
    Indented(u32),
    /// Emit a false `End` if we aren't at a minimum indent
    NoParBreak,
}

impl NewlineMode {
    /// Whether to stop at the current token based on the newline info.
    fn stop_at(&self, lines: NewlineInfo, next: impl FnOnce() -> SyntaxKind) -> bool {
        match self {
            NewlineMode::Continue => false,
            NewlineMode::Stop => true,
            NewlineMode::NoParBreak => lines.parbreak,
            NewlineMode::Indented(min_indent) => lines.column < *min_indent,
            NewlineMode::MaybeContinue(set) => !set.contains(next()),
        }
    }
}

impl PartialEq for NewlineMode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (NewlineMode::Continue, NewlineMode::Continue)
            | (NewlineMode::Stop, NewlineMode::Stop) => true,
            (NewlineMode::Indented(i), NewlineMode::Indented(j)) if i == j => true,
            // TODO: This is fine for now but isn't really proper.
            (NewlineMode::MaybeContinue(_), NewlineMode::MaybeContinue(_)) => false,
            _ => false,
        }
    }
}

/// An index into the parser's nodes vector, used as a start/stop point for
/// wrapping.
///
/// Markers are given as `nodes.len()` (which is not a valid index into nodes)
/// because the current token will take up that index when eaten.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Marker(usize);

/// Cheap checkpoint of Parser state for memoization. Doesn't track lexer modes
/// since those should never differ between two memoized parses.
#[derive(Clone)]
struct Checkpoint<'s> {
    lexer: Lexer<'s>,
    mode: NewlineMode,
    current: Token,
    node_len: usize,
}

impl<'s> Parser<'s> {
    /// Create a new parser starting at the given text offset and lexer mode.
    fn new(text: &'s str, offset: usize, lexer_mode: LexMode) -> Self {
        let mut lexer = Lexer::new(text, lexer_mode);
        lexer.jump(offset);
        let mut nodes = vec![];
        let mode = NewlineMode::Continue;
        let current = Self::lex_past_trivia(&mut lexer, mode, &mut nodes);
        Self {
            text,
            lexer,
            mode,
            current,
            balanced: true,
            nodes,
            memo: HashMap::new(),
            memo_arena: vec![],
        }
    }

    /// Consume the parser, yielding the full vector of parsed SyntaxNodes.
    fn finish_into(self, kind: SyntaxKind) -> SyntaxNode {
        SyntaxNode::inner(kind, self.nodes)
    }

    fn prev_end(&self) -> usize {
        self.current.prev_end
    }

    /// Similar to a 'peek()' function: returns the 'kind' of the next token to
    /// be eaten.
    fn current_kind(&self) -> SyntaxKind {
        self.current.kind
    }

    /// The offset into `text` of the current token's start.
    fn current_start(&self) -> usize {
        self.lexer.cursor() - self.current.node.len()
    }

    /// The offset into `text` after current token's text.
    fn current_end(&self) -> usize {
        self.lexer.cursor()
    }

    /// The current token's text.
    fn current_text(&self) -> &str {
        self.current.node.text()
    }

    /// Whether the current token is a given SyntaxKind.
    fn at(&self, kind: SyntaxKind) -> bool {
        self.current.kind == kind
    }

    /// Whether the current token matches a SyntaxSet.
    fn at_set(&self, set: SyntaxSet) -> bool {
        set.contains(self.current.kind)
    }

    /// Whether there was trivia between the current node and its predecessor.
    fn had_trivia(&self) -> bool {
        self.current.n_trivia > 0
    }

    /// If we're at a certain kind with no preceding trivia.
    fn directly_at(&self, kind: SyntaxKind) -> bool {
        self.current.kind == kind && !self.had_trivia()
    }

    /// Whether we're at the end of the token stream.
    ///
    /// Note: this might be a 'fake' end due to the NewlineMode.
    fn end(&self) -> bool {
        self.at(SyntaxKind::End)
    }

    /// Eats if at `kind`. Returns true if eaten.
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

    /// Eats only if at `kind` with no preceding trivia. Returns true if eaten.
    fn eat_if_direct(&mut self, kind: SyntaxKind) -> bool {
        let at = self.directly_at(kind);
        if at {
            self.eat();
        }
        at
    }

    /// Assert that we are at the given SyntaxKind and eat it. This should be
    /// used when moving between functions that expect to start with a specific
    /// token.
    #[track_caller]
    fn assert(&mut self, kind: SyntaxKind) {
        assert_eq!(self.current.kind, kind);
        self.eat();
    }

    /// Convert the current token's SyntaxKind and eat it.
    fn convert_and_eat(&mut self, kind: SyntaxKind) {
        self.current.node.convert_to_kind(kind);
        // We don't need to change `current.kind` since it will be overwritten
        // when eaten.
        self.eat();
    }

    /// Whether there was a newline among this token's leading trivia.
    fn newline(&mut self) -> bool {
        self.current.had_newline.is_some()
    }

    // The indent required to match the current token's column.
    fn next_indent(&self) -> u32 {
        if let Some(column) = self.current.had_newline.map(|n| n.column) {
            column + 1
        } else {
            self.lexer.column() as u32 + 1
        }
    }

    /// Get a marker of the parser's location for future wrapping.
    fn marker(&self) -> Marker {
        Marker(self.nodes.len())
    }

    /// Get a marker that includes any trivia before the current token.
    fn before_trivia(&self) -> Marker {
        Marker(self.nodes.len() - self.current.n_trivia)
    }

    /// Detach any leading trivia from the current node, but don't change
    /// newline information.
    fn flush_trivia(&mut self) {
        self.current.n_trivia = 0;
        self.current.prev_end = self.lexer.cursor() - self.current.node.len();
    }

    #[track_caller]
    fn post_process(&mut self, m: Marker) -> impl Iterator<Item = &mut SyntaxNode> {
        self.nodes[m.0..]
            .iter_mut()
            .filter(|child| !child.kind().is_error() && !child.kind().is_trivia())
    }

    /// Wrap the nodes from a marker up to the current node in a new 'Inner
    /// Node' of the given kind. This is an easy interface for creating nested
    /// SyntaxNodes of a certain kind _after_ having parsed their required
    /// children.
    fn wrap(&mut self, from: Marker, kind: SyntaxKind) {
        let to = self.before_trivia().0;
        let from = from.0.min(to);
        let children = self.nodes.drain(from..to).collect();
        self.nodes.insert(from, SyntaxNode::inner(kind, children));
    }

    /// Eats the current token by saving it to the `nodes` vector, then moves
    /// the lexer forward to prepare a new token.
    fn eat(&mut self) {
        self.nodes.push(std::mem::take(&mut self.current.node));
        self.current = Self::lex_past_trivia(&mut self.lexer, self.mode, &mut self.nodes);
    }

    /// Parse within a given Lexer mode, exiting and re-lexing if necessary.
    /// This is effectively using the call stack as a stack of lexer modes, but
    /// is safer and more ergonomic than a manual stack.
    ///
    /// Note that this doesn't change the current node on entry. Only subsequent
    /// nodes will use the new lexer mode, but on exit we will re-lex current if
    /// the mode changed.
    fn enter_mode(&mut self, mode: LexMode, func: impl FnOnce(&mut Parser<'s>)) {
        let prev_mode = self.lexer.mode();
        self.lexer.set_mode(mode);
        func(self);
        self.lexer.set_mode(prev_mode);
        if mode != prev_mode {
            self.lexer.jump(self.prev_end());
            self.nodes.truncate(self.nodes.len() - self.current.n_trivia);
            self.current =
                Self::lex_past_trivia(&mut self.lexer, self.mode, &mut self.nodes);
        }
    }

    /// Parse with a given newline mode, exiting and updating the current token
    /// if necessary.
    ///
    /// Note that this doesn't change the current node on entry. Only subsequent
    /// nodes will use the new newline mode, but on exit we may update current
    /// if the mode changed.
    fn enter_newline_mode(
        &mut self,
        mode: NewlineMode,
        func: impl FnOnce(&mut Parser<'s>),
    ) {
        let prev_mode = self.mode;
        self.mode = mode;
        func(self);
        self.mode = prev_mode;
        if mode != prev_mode {
            if let Token { node, kind, had_newline: Some(lines), .. } = &mut self.current
            {
                if self.mode.stop_at(*lines, || node.kind()) {
                    *kind = SyntaxKind::End;
                } else if *kind == SyntaxKind::End {
                    *kind = node.kind();
                }
            }
        }
    }

    /// Move the lexer forward to return the next token and lex past any trivia
    /// tokens, pushing them into `nodes`.
    ///
    /// Might convert the token's kind into a false SyntaxKind::End based on the
    /// newline mode.
    fn lex_past_trivia(
        lexer: &mut Lexer,
        mode: NewlineMode,
        nodes: &mut Vec<SyntaxNode>,
    ) -> Token {
        let mut n_trivia = 0;
        let mut had_newline: Option<NewlineInfo> = None;

        let prev_end = lexer.cursor();
        let (node, mut kind) = loop {
            let (kind, token_type, node) = lexer.next();
            match token_type {
                TokenType::Normal => break (node, kind),
                TokenType::Trivia => {
                    n_trivia += 1;
                }
                TokenType::Newline { parbreak, column } => {
                    n_trivia += 1;
                    had_newline = Some(NewlineInfo {
                        parbreak: parbreak || had_newline.is_some_and(|nl| nl.parbreak),
                        column,
                    });
                }
            }
            nodes.push(node);
        };
        if had_newline.is_some_and(|lines| mode.stop_at(lines, || node.kind())) {
            kind = SyntaxKind::End;
        }
        Token { node, kind, n_trivia, had_newline, prev_end }
    }

    /// Save a checkpoint of the parser state.
    fn checkpoint(&self) -> Checkpoint<'s> {
        Checkpoint {
            lexer: self.lexer.clone(),
            mode: self.mode,
            current: self.current.clone(),
            node_len: self.nodes.len(),
        }
    }

    /// Reset the parser from a checkpoint.
    fn restore(&mut self, checkpoint: Checkpoint<'s>) {
        self.lexer = checkpoint.lexer;
        self.mode = checkpoint.mode;
        self.current = checkpoint.current;
        self.nodes.truncate(checkpoint.node_len);
    }
}

impl<'s> Parser<'s> {
    /// Consume the given syntax `kind` or produce an error.
    fn expect(&mut self, kind: SyntaxKind) -> bool {
        let at = self.at(kind);
        if at {
            self.eat();
        } else if kind == SyntaxKind::Ident && self.current.kind.is_keyword() {
            self.trim_errors();
            self.current.node.expected(kind.name());
            self.eat();
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
        let m = self.before_trivia();
        let after_error = m.0 > 0 && self.nodes[m.0 - 1].kind().is_error();
        if !after_error {
            self.expected_at(m, thing);
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
        self.balanced &= !self.current.kind.is_grouping();
        self.current.node.unexpected();
        self.eat();
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
