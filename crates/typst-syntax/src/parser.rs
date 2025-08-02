use std::mem;
use std::ops::{Index, IndexMut, Range};

use ecow::{EcoString, eco_format};
use rustc_hash::{FxHashMap, FxHashSet};
use typst_utils::default_math_class;
use unicode_math_class::MathClass;

use crate::set::{SyntaxSet, syntax_set};
use crate::{Lexer, SyntaxError, SyntaxKind, SyntaxMode, SyntaxNode, ast, set};

/// Parses a source file as top-level markup.
pub fn parse(text: &str) -> SyntaxNode {
    let _scope = typst_timing::TimingScope::new("parse");
    let mut p = Parser::new(text, 0, SyntaxMode::Markup);
    markup_exprs(&mut p, true, syntax_set!(End));
    p.finish_into(SyntaxKind::Markup)
}

/// Parses top-level code.
pub fn parse_code(text: &str) -> SyntaxNode {
    let _scope = typst_timing::TimingScope::new("parse code");
    let mut p = Parser::new(text, 0, SyntaxMode::Code);
    code_exprs(&mut p, syntax_set!(End));
    p.finish_into(SyntaxKind::Code)
}

/// Parses top-level math.
pub fn parse_math(text: &str) -> SyntaxNode {
    let _scope = typst_timing::TimingScope::new("parse math");
    let mut p = Parser::new(text, 0, SyntaxMode::Math);
    math_exprs(&mut p, syntax_set!(End));
    p.finish_into(SyntaxKind::Math)
}

/// Parses markup expressions until a stop condition is met.
fn markup(p: &mut Parser, at_start: bool, wrap_trivia: bool, stop_set: SyntaxSet) {
    let m = if wrap_trivia { p.before_trivia() } else { p.marker() };
    markup_exprs(p, at_start, stop_set);
    if wrap_trivia {
        p.flush_trivia();
    }
    p.wrap(m, SyntaxKind::Markup);
}

/// Parses a sequence of markup expressions.
fn markup_exprs(p: &mut Parser, mut at_start: bool, stop_set: SyntaxSet) {
    debug_assert!(stop_set.contains(SyntaxKind::End));
    at_start |= p.had_newline();
    let mut nesting: usize = 0;
    // Keep going if we're at a nested right-bracket regardless of the stop set.
    while !p.at_set(stop_set) || (nesting > 0 && p.at(SyntaxKind::RightBracket)) {
        markup_expr(p, at_start, &mut nesting);
        at_start = p.had_newline();
    }
}

/// Reparses a subsection of markup incrementally.
pub(super) fn reparse_markup(
    text: &str,
    range: Range<usize>,
    at_start: &mut bool,
    nesting: &mut usize,
    top_level: bool,
) -> Option<Vec<SyntaxNode>> {
    let mut p = Parser::new(text, range.start, SyntaxMode::Markup);
    *at_start |= p.had_newline();
    while !p.end() && p.current_start() < range.end {
        // If not top-level and at a new RightBracket, stop the reparse.
        if !top_level && *nesting == 0 && p.at(SyntaxKind::RightBracket) {
            break;
        }
        markup_expr(&mut p, *at_start, nesting);
        *at_start = p.had_newline();
    }
    (p.balanced && p.current_start() == range.end).then(|| p.finish())
}

/// Parses a single markup expression. This includes markup elements like text,
/// headings, strong/emph, lists/enums, etc. This is also the entry point for
/// parsing math equations and embedded code expressions.
fn markup_expr(p: &mut Parser, at_start: bool, nesting: &mut usize) {
    match p.current() {
        SyntaxKind::LeftBracket => {
            *nesting += 1;
            p.convert_and_eat(SyntaxKind::Text);
        }
        SyntaxKind::RightBracket if *nesting > 0 => {
            *nesting -= 1;
            p.convert_and_eat(SyntaxKind::Text);
        }
        SyntaxKind::RightBracket => {
            p.unexpected();
            p.hint("try using a backslash escape: \\]");
        }

        SyntaxKind::Shebang => p.eat(),

        SyntaxKind::Text
        | SyntaxKind::Linebreak
        | SyntaxKind::Escape
        | SyntaxKind::Shorthand
        | SyntaxKind::SmartQuote
        | SyntaxKind::Link
        | SyntaxKind::Label => p.eat(),

        SyntaxKind::Raw => p.eat(), // Raw is handled entirely in the Lexer.

        SyntaxKind::Hash => embedded_code_expr(p),
        SyntaxKind::Star => strong(p),
        SyntaxKind::Underscore => emph(p),
        SyntaxKind::HeadingMarker if at_start => heading(p),
        SyntaxKind::ListMarker if at_start => list_item(p),
        SyntaxKind::EnumMarker if at_start => enum_item(p),
        SyntaxKind::TermMarker if at_start => term_item(p),
        SyntaxKind::RefMarker => reference(p),
        SyntaxKind::Dollar => equation(p),

        SyntaxKind::HeadingMarker
        | SyntaxKind::ListMarker
        | SyntaxKind::EnumMarker
        | SyntaxKind::TermMarker
        | SyntaxKind::Colon => p.convert_and_eat(SyntaxKind::Text),

        _ => p.unexpected(),
    }
}

/// Parses strong content: `*Strong*`.
fn strong(p: &mut Parser) {
    p.with_nl_mode(AtNewline::StopParBreak, |p| {
        let m = p.marker();
        p.assert(SyntaxKind::Star);
        markup(p, false, true, syntax_set!(Star, RightBracket, End));
        p.expect_closing_delimiter(m, SyntaxKind::Star);
        p.wrap(m, SyntaxKind::Strong);
    });
}

/// Parses emphasized content: `_Emphasized_`.
fn emph(p: &mut Parser) {
    p.with_nl_mode(AtNewline::StopParBreak, |p| {
        let m = p.marker();
        p.assert(SyntaxKind::Underscore);
        markup(p, false, true, syntax_set!(Underscore, RightBracket, End));
        p.expect_closing_delimiter(m, SyntaxKind::Underscore);
        p.wrap(m, SyntaxKind::Emph);
    });
}

/// Parses a section heading: `= Introduction`.
fn heading(p: &mut Parser) {
    p.with_nl_mode(AtNewline::Stop, |p| {
        let m = p.marker();
        p.assert(SyntaxKind::HeadingMarker);
        markup(p, false, false, syntax_set!(Label, RightBracket, End));
        p.wrap(m, SyntaxKind::Heading);
    });
}

/// Parses an item in a bullet list: `- ...`.
fn list_item(p: &mut Parser) {
    p.with_nl_mode(AtNewline::RequireColumn(p.current_column()), |p| {
        let m = p.marker();
        p.assert(SyntaxKind::ListMarker);
        markup(p, true, false, syntax_set!(RightBracket, End));
        p.wrap(m, SyntaxKind::ListItem);
    });
}

/// Parses an item in an enumeration (numbered list): `+ ...` or `1. ...`.
fn enum_item(p: &mut Parser) {
    p.with_nl_mode(AtNewline::RequireColumn(p.current_column()), |p| {
        let m = p.marker();
        p.assert(SyntaxKind::EnumMarker);
        markup(p, true, false, syntax_set!(RightBracket, End));
        p.wrap(m, SyntaxKind::EnumItem);
    });
}

/// Parses an item in a term list: `/ Term: Details`.
fn term_item(p: &mut Parser) {
    p.with_nl_mode(AtNewline::RequireColumn(p.current_column()), |p| {
        let m = p.marker();
        p.with_nl_mode(AtNewline::Stop, |p| {
            p.assert(SyntaxKind::TermMarker);
            markup(p, false, false, syntax_set!(Colon, RightBracket, End));
        });
        p.expect(SyntaxKind::Colon);
        markup(p, true, false, syntax_set!(RightBracket, End));
        p.wrap(m, SyntaxKind::TermItem);
    });
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
    let m = p.marker();
    p.enter_modes(SyntaxMode::Math, AtNewline::Continue, |p| {
        p.assert(SyntaxKind::Dollar);
        math(p, syntax_set!(Dollar, End));
        p.expect_closing_delimiter(m, SyntaxKind::Dollar);
    });
    p.wrap(m, SyntaxKind::Equation);
}

/// Parses the contents of a mathematical equation: `x^2 + 1`.
fn math(p: &mut Parser, stop_set: SyntaxSet) {
    let m = p.marker();
    math_exprs(p, stop_set);
    p.wrap(m, SyntaxKind::Math);
}

/// Parses a sequence of math expressions. Returns the number of expressions
/// parsed.
fn math_exprs(p: &mut Parser, stop_set: SyntaxSet) -> usize {
    debug_assert!(stop_set.contains(SyntaxKind::End));
    let mut count = 0;
    while !p.at_set(stop_set) {
        if p.at_set(set::MATH_EXPR) {
            math_expr(p);
            count += 1;
        } else {
            p.unexpected();
        }
    }
    count
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
        // The lexer manages creating full FieldAccess nodes if needed.
        SyntaxKind::MathIdent | SyntaxKind::FieldAccess => {
            continuable = true;
            p.eat();
            // Parse a function call for an identifier or field access.
            if min_prec < 3
                && p.directly_at(SyntaxKind::MathText)
                && p.current_text() == "("
            {
                math_args(p);
                p.wrap(m, SyntaxKind::FuncCall);
                continuable = false;
            }
        }

        SyntaxKind::Dot
        | SyntaxKind::Bang
        | SyntaxKind::Comma
        | SyntaxKind::Semicolon
        | SyntaxKind::RightParen => {
            p.convert_and_eat(SyntaxKind::MathText);
        }

        SyntaxKind::Text | SyntaxKind::MathText | SyntaxKind::MathShorthand => {
            // `a(b)/c` parses as `(a(b))/c` if `a` is continuable.
            continuable = math_class(p.current_text()) == Some(MathClass::Alphabetic)
                || p.current_text().chars().all(char::is_alphabetic);
            if !maybe_delimited(p) {
                p.eat();
            }
        }

        SyntaxKind::Linebreak | SyntaxKind::MathAlignPoint => p.eat(),
        SyntaxKind::MathPrimes | SyntaxKind::Escape | SyntaxKind::Str => {
            // If eating primes here, it means they had nothing to attach to.
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

        _ => p.expected("expression"),
    }

    if continuable && min_prec < 3 && !p.had_trivia() && maybe_delimited(p) {
        p.wrap(m, SyntaxKind::Math);
    }

    // Whether there were _any_ primes in the loop.
    let mut primed = false;

    while !p.end() && !p.at(stop) {
        if p.directly_at(SyntaxKind::Bang) {
            // Bang acts as a postfix operator with the highest possible
            // precedence, but is purely a parse-time construct and is never
            // output to the final `SyntaxNode`.
            p.convert_and_eat(SyntaxKind::MathText);
            p.wrap(m, SyntaxKind::Math);
            continue;
        }

        if !p.had_trivia() && p.eat_if(SyntaxKind::MathPrimes) {
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

/// Precedence and wrapper kinds for the binary math operators.
fn math_op(kind: SyntaxKind) -> Option<(SyntaxKind, SyntaxKind, ast::Assoc, usize)> {
    match kind {
        SyntaxKind::Underscore => {
            Some((SyntaxKind::MathAttach, SyntaxKind::Hat, ast::Assoc::Right, 3))
        }
        SyntaxKind::Hat => {
            Some((SyntaxKind::MathAttach, SyntaxKind::Underscore, ast::Assoc::Right, 3))
        }
        SyntaxKind::Slash => {
            Some((SyntaxKind::MathFrac, SyntaxKind::End, ast::Assoc::Left, 1))
        }
        _ => None,
    }
}

/// Try to parse delimiters based on the current token's unicode math class.
fn maybe_delimited(p: &mut Parser) -> bool {
    let open = math_class(p.current_text()) == Some(MathClass::Opening);
    if open {
        math_delimited(p);
    }
    open
}

/// Parse matched delimiters in math: `[x + y]`.
fn math_delimited(p: &mut Parser) {
    let m = p.marker();
    p.eat();
    let m2 = p.marker();
    while !p.at_set(syntax_set!(Dollar, End)) {
        if math_class(p.current_text()) == Some(MathClass::Closing) {
            p.wrap(m2, SyntaxKind::Math);
            // We could be at the shorthand `|]`, which shouldn't be converted
            // to a `Text` kind.
            if p.at(SyntaxKind::RightParen) {
                p.convert_and_eat(SyntaxKind::MathText);
            } else {
                p.eat();
            }
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

/// Remove one set of parentheses (if any) from a previously parsed expression
/// by converting to non-expression SyntaxKinds.
fn math_unparen(p: &mut Parser, m: Marker) {
    let Some(node) = p.nodes.get_mut(m.0) else { return };
    if node.kind() != SyntaxKind::MathDelimited {
        return;
    }

    if let [first, .., last] = node.children_mut()
        && first.text() == "("
        && last.text() == ")"
    {
        first.convert_to_kind(SyntaxKind::LeftParen);
        last.convert_to_kind(SyntaxKind::RightParen);
        // Only convert if we did have regular parens.
        node.convert_to_kind(SyntaxKind::Math);
    }
}

/// The unicode math class of a string. Only returns `Some` if `text` has
/// exactly one unicode character or is a math shorthand string (currently just
/// `[|`, `||`, `|]`) and then only returns `Some` if there is a math class
/// defined for that character.
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
        .and_then(default_math_class)
}

/// Parse an argument list in math: `(a, b; c, d; size: #50%)`.
fn math_args(p: &mut Parser) {
    let m = p.marker();
    p.convert_and_eat(SyntaxKind::LeftParen);

    let mut positional = true;
    let mut has_arrays = false;

    let mut maybe_array_start = p.marker();
    let mut seen = FxHashSet::default();
    while !p.at_set(syntax_set!(End, Dollar, RightParen)) {
        positional = math_arg(p, &mut seen);

        match p.current() {
            SyntaxKind::Comma => {
                p.eat();
                if !positional {
                    maybe_array_start = p.marker();
                }
            }
            SyntaxKind::Semicolon => {
                if !positional {
                    maybe_array_start = p.marker();
                }

                // Parses an array: `a, b, c;`.
                // The semicolon merges preceding arguments separated by commas
                // into an array argument.
                p.wrap(maybe_array_start, SyntaxKind::Array);
                p.eat();
                maybe_array_start = p.marker();
                has_arrays = true;
            }
            SyntaxKind::End | SyntaxKind::Dollar | SyntaxKind::RightParen => {}
            _ => p.expected("comma or semicolon"),
        }
    }

    // Check if we need to wrap the preceding arguments in an array.
    if maybe_array_start != p.marker() && has_arrays && positional {
        p.wrap(maybe_array_start, SyntaxKind::Array);
    }

    p.expect_closing_delimiter(m, SyntaxKind::RightParen);
    p.wrap(m, SyntaxKind::Args);
}

/// Parses a single argument in a math argument list.
///
/// Returns whether the parsed argument was positional or not.
fn math_arg<'s>(p: &mut Parser<'s>, seen: &mut FxHashSet<&'s str>) -> bool {
    let m = p.marker();
    let start = p.current_start();

    if p.at(SyntaxKind::Dot) {
        // Parses a spread argument: `..args`.
        if let Some(spread) = p.lexer.maybe_math_spread_arg(start) {
            p.token.node = spread;
            p.eat();
            math_expr(p);
            p.wrap(m, SyntaxKind::Spread);
            return true;
        }
    }

    let mut positional = true;
    if p.at_set(syntax_set!(MathText, MathIdent, Underscore)) {
        // Parses a named argument: `thickness: #12pt`.
        if let Some(named) = p.lexer.maybe_math_named_arg(start) {
            p.token.node = named;
            let text = p.current_text();
            p.eat();
            p.convert_and_eat(SyntaxKind::Colon);
            if !seen.insert(text) {
                p[m].convert_to_error(eco_format!("duplicate argument: {text}"));
            }
            positional = false;
        }
    }

    // Parses a normal positional argument.
    let arg = p.marker();
    let count = math_exprs(p, syntax_set!(End, Dollar, Comma, Semicolon, RightParen));
    if count == 0 {
        // Named argument requires a value.
        if !positional {
            p.expected("expression");
        }

        // Flush trivia so that the new empty Math node will be wrapped _inside_
        // any `SyntaxKind::Array` elements created in `math_args`.
        // (And if we don't follow by wrapping in an array, it has no effect.)
        // The difference in node layout without this would look like:
        // Expression: `$ mat( ;) $`
        // - Correct:   [ .., Space(" "), Array[Math[], ], Semicolon(";"), .. ]
        // - Incorrect: [ .., Math[], Array[], Space(" "), Semicolon(";"), .. ]
        p.flush_trivia();
    }

    // Wrap math function arguments to join adjacent math content or create an
    // empty 'Math' node for when we have 0 args. We don't wrap when
    // `count == 1`, since wrapping would change the type of the expression
    // from potentially non-content to content. Ex: `$ func(#12pt) $` would
    // change the type from size to content if wrapped.
    if count != 1 {
        p.wrap(arg, SyntaxKind::Math);
    }

    if !positional {
        p.wrap(m, SyntaxKind::Named);
    }
    positional
}

/// Parses the contents of a code block.
fn code(p: &mut Parser, stop_set: SyntaxSet) {
    let m = p.marker();
    code_exprs(p, stop_set);
    p.wrap(m, SyntaxKind::Code);
}

/// Parses a sequence of code expressions.
fn code_exprs(p: &mut Parser, stop_set: SyntaxSet) {
    debug_assert!(stop_set.contains(SyntaxKind::End));
    while !p.at_set(stop_set) {
        p.with_nl_mode(AtNewline::ContextualContinue, |p| {
            if !p.at_set(set::CODE_EXPR) {
                p.unexpected();
                return;
            }
            code_expr(p);
            if !p.at_set(stop_set) && !p.eat_if(SyntaxKind::Semicolon) {
                p.expected("semicolon or line break");
                if p.at(SyntaxKind::Label) {
                    p.hint("labels can only be applied in markup mode");
                    p.hint("try wrapping your code in a markup block (`[ ]`)");
                }
            }
        });
    }
}

/// Parses an atomic code expression embedded in markup or math.
fn embedded_code_expr(p: &mut Parser) {
    p.enter_modes(SyntaxMode::Code, AtNewline::Stop, |p| {
        p.assert(SyntaxKind::Hash);
        if p.had_trivia() || p.end() {
            p.expected("expression");
            return;
        }

        let stmt = p.at_set(set::STMT);
        let at = p.at_set(set::ATOMIC_CODE_EXPR);
        code_expr_prec(p, true, 0);

        // Consume error for things like `#12p` or `#"abc\"`.#
        if !at {
            p.unexpected();
        }

        let semi = (stmt || p.directly_at(SyntaxKind::Semicolon))
            && p.eat_if(SyntaxKind::Semicolon);

        if stmt && !semi && !p.end() && !p.at(SyntaxKind::RightBracket) {
            p.expected("semicolon or line break");
        }
    });
}

/// Parses a single code expression.
fn code_expr(p: &mut Parser) {
    code_expr_prec(p, false, 0)
}

/// Parses a code expression with at least the given precedence.
fn code_expr_prec(p: &mut Parser, atomic: bool, min_prec: u8) {
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

        SyntaxKind::Raw => p.eat(), // Raw is handled entirely in the Lexer.

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

/// Reparses a full content or code block.
pub(super) fn reparse_block(text: &str, range: Range<usize>) -> Option<SyntaxNode> {
    let mut p = Parser::new(text, range.start, SyntaxMode::Code);
    assert!(p.at(SyntaxKind::LeftBracket) || p.at(SyntaxKind::LeftBrace));
    block(&mut p);
    (p.balanced && p.prev_end() == range.end)
        .then(|| p.finish().into_iter().next().unwrap())
}

/// Parses a content or code block.
fn block(p: &mut Parser) {
    match p.current() {
        SyntaxKind::LeftBracket => content_block(p),
        SyntaxKind::LeftBrace => code_block(p),
        _ => p.expected("block"),
    }
}

/// Parses a code block: `{ let x = 1; x + 2 }`.
fn code_block(p: &mut Parser) {
    let m = p.marker();
    p.enter_modes(SyntaxMode::Code, AtNewline::Continue, |p| {
        p.assert(SyntaxKind::LeftBrace);
        code(p, syntax_set!(RightBrace, RightBracket, RightParen, End));
        p.expect_closing_delimiter(m, SyntaxKind::RightBrace);
    });
    p.wrap(m, SyntaxKind::CodeBlock);
}

/// Parses a content block: `[*Hi* there!]`.
fn content_block(p: &mut Parser) {
    let m = p.marker();
    p.enter_modes(SyntaxMode::Markup, AtNewline::Continue, |p| {
        p.assert(SyntaxKind::LeftBracket);
        markup(p, true, true, syntax_set!(RightBracket, End));
        p.expect_closing_delimiter(m, SyntaxKind::RightBracket);
    });
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
        pattern(p, false, &mut FxHashSet::default(), None);
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

    let mut seen = FxHashSet::default();
    pattern(p, false, &mut seen, None);

    if p.at(SyntaxKind::Comma) {
        let node = p.eat_and_get();
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
            p.with_nl_mode(AtNewline::Continue, |p| {
                let m2 = p.marker();
                p.assert(SyntaxKind::LeftParen);

                import_items(p);

                p.expect_closing_delimiter(m2, SyntaxKind::RightParen);
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
    if atomic {
        // Atomic expressions aren't modified by operators that follow them, so
        // our first guess of array/dict will be correct.
        parenthesized_or_array_or_dict(p);
        return;
    }

    // If we've seen this position before and have a memoized result, restore it
    // and return. Otherwise, get a key to this position and a checkpoint to
    // restart from in case we make a wrong prediction.
    let Some((memo_key, checkpoint)) = p.restore_memo_or_checkpoint() else { return };
    // The node length from when we restored.
    let prev_len = checkpoint.node_len;

    // When we reach a '(', we can't be sure what it is. First, we attempt to
    // parse as a simple parenthesized expression, array, or dictionary as
    // these are the most likely things. We can handle all of those in a single
    // pass.
    let kind = parenthesized_or_array_or_dict(p);

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
        let m = p.marker();
        params(p);
        if !p.expect(SyntaxKind::Arrow) {
            return;
        }
        code_expr(p);
        p.wrap(m, SyntaxKind::Closure);
    } else if p.at(SyntaxKind::Eq) && kind != SyntaxKind::Parenthesized {
        p.restore(checkpoint);
        let m = p.marker();
        destructuring_or_parenthesized(p, true, &mut FxHashSet::default());
        if !p.expect(SyntaxKind::Eq) {
            return;
        }
        code_expr(p);
        p.wrap(m, SyntaxKind::DestructAssignment);
    } else {
        return;
    }

    // Memoize result if we backtracked.
    p.memoize_parsed_nodes(memo_key, prev_len);
}

/// Parses either
/// - a parenthesized expression: `(1 + 2)`, or
/// - an array: `(1, "hi", 12cm)`, or
/// - a dictionary: `(thickness: 3pt, dash: "solid")`.
fn parenthesized_or_array_or_dict(p: &mut Parser) -> SyntaxKind {
    let mut state = GroupState {
        count: 0,
        maybe_just_parens: true,
        kind: None,
        seen: FxHashSet::default(),
    };

    // An edge case with parens is whether we can interpret a leading spread
    // expression as a dictionary, e.g. if we want `(..dict1, ..dict2)` to join
    // the two dicts.
    //
    // The issue is that we decide on the type of the parenthesized expression
    // here in the parser by the `SyntaxKind` we wrap with, instead of in eval
    // based on the type of the spread item.
    //
    // The current fix is that we allow a leading colon to force the
    // parenthesized value into a dict:
    // - `(..arr1, ..arr2)` is wrapped as an `Array`.
    // - `(: ..dict1, ..dict2)` is wrapped as a `Dict`.
    //
    // This does allow some unexpected expressions, such as `(: key: val)`, but
    // it's currently intentional.
    let m = p.marker();
    p.with_nl_mode(AtNewline::Continue, |p| {
        p.assert(SyntaxKind::LeftParen);
        if p.eat_if(SyntaxKind::Colon) {
            state.kind = Some(SyntaxKind::Dict);
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
    /// Whether this is just a single expression in parens: `(a)`. Single
    /// element arrays require an explicit comma: `(a,)`, unless we're
    /// spreading: `(..a)`.
    maybe_just_parens: bool,
    /// The `SyntaxKind` to wrap as (if we've figured it out yet).
    kind: Option<SyntaxKind>,
    /// Store named arguments so we can give an error if they're repeated.
    seen: FxHashSet<EcoString>,
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
        } && !state.seen.insert(key.clone())
        {
            node.convert_to_error(eco_format!("duplicate key: {key}"));
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
    if !p.directly_at(SyntaxKind::LeftParen) && !p.directly_at(SyntaxKind::LeftBracket) {
        p.expected("argument list");
        if p.at(SyntaxKind::LeftParen) || p.at(SyntaxKind::LeftBracket) {
            p.hint("there may not be any spaces before the argument list");
        }
    }

    let m = p.marker();
    if p.at(SyntaxKind::LeftParen) {
        let m2 = p.marker();
        p.with_nl_mode(AtNewline::Continue, |p| {
            p.assert(SyntaxKind::LeftParen);

            let mut seen = FxHashSet::default();
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
        });
    }

    while p.directly_at(SyntaxKind::LeftBracket) {
        content_block(p);
    }

    p.wrap(m, SyntaxKind::Args);
}

/// Parses a single argument in an argument list.
fn arg<'s>(p: &mut Parser<'s>, seen: &mut FxHashSet<&'s str>) {
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
    p.with_nl_mode(AtNewline::Continue, |p| {
        p.assert(SyntaxKind::LeftParen);

        let mut seen = FxHashSet::default();
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
    });
    p.wrap(m, SyntaxKind::Params);
}

/// Parses a single parameter in a parameter list.
fn param<'s>(p: &mut Parser<'s>, seen: &mut FxHashSet<&'s str>, sink: &mut bool) {
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
    seen: &mut FxHashSet<&'s str>,
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
    seen: &mut FxHashSet<&'s str>,
) {
    let mut sink = false;
    let mut count = 0;
    let mut maybe_just_parens = true;

    let m = p.marker();
    p.with_nl_mode(AtNewline::Continue, |p| {
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
    });

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
    seen: &mut FxHashSet<&'s str>,
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

    // We must use a full checkpoint here (can't just clone the lexer) because
    // there may be trivia between the identifier and the colon we need to skip.
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
    seen: &mut FxHashSet<&'s str>,
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

/// Manages parsing a stream of tokens into a tree of [`SyntaxNode`]s.
///
/// The implementation presents an interface that investigates a current `token`
/// with a [`SyntaxKind`] and can take one of the following actions:
///
/// 1. Eat a token: push `token` onto the `nodes` vector as a [leaf
///    node](`SyntaxNode::leaf`) and prepare a new `token` by calling into the
///    lexer.
/// 2. Wrap nodes from a marker to the end of `nodes` (excluding `token` and any
///    attached trivia) into an [inner node](`SyntaxNode::inner`) of a specific
///    `SyntaxKind`.
/// 3. Produce or convert nodes into an [error node](`SyntaxNode::error`) when
///    something expected is missing or something unexpected is found.
///
/// Overall the parser produces a nested tree of SyntaxNodes as a "_Concrete_
/// Syntax Tree." The raw Concrete Syntax Tree should contain the entire source
/// text, and is used as-is for e.g. syntax highlighting and IDE features. In
/// `ast.rs` the CST is interpreted as a lazy view over an "_Abstract_ Syntax
/// Tree." The AST module skips over irrelevant tokens -- whitespace, comments,
/// code parens, commas in function args, etc. -- as it iterates through the
/// tree.
///
/// ### Modes
///
/// The parser manages the transitions between the three modes of Typst through
/// [syntax modes](`SyntaxMode`) and [newline modes](`AtNewline`).
///
/// The syntax modes map to the three Typst modes and are stored in the lexer,
/// changing which `SyntaxKind`s it will generate.
///
/// The newline mode is used to determine whether a newline should end the
/// current expression. If so, the parser temporarily changes `token`'s kind to
/// a fake [`SyntaxKind::End`]. When the parser exits the mode the original
/// `SyntaxKind` is restored.
struct Parser<'s> {
    /// The source text shared with the lexer.
    text: &'s str,
    /// A lexer over the source text with multiple modes. Defines the boundaries
    /// of tokens and determines their [`SyntaxKind`]. Contains the [`SyntaxMode`]
    /// defining our current Typst mode.
    lexer: Lexer<'s>,
    /// The newline mode: whether to insert a temporary end at newlines.
    nl_mode: AtNewline,
    /// The current token under inspection, not yet present in `nodes`. This
    /// acts like a single item of lookahead for the parser.
    ///
    /// When wrapping, this is _not_ included in the wrapped nodes.
    token: Token,
    /// Whether the parser has the expected set of open/close delimiters. This
    /// only ever transitions from `true` to `false`.
    balanced: bool,
    /// Nodes representing the concrete syntax tree of previously parsed text.
    /// In Code and Math, includes previously parsed trivia, but not `token`.
    nodes: Vec<SyntaxNode>,
    /// Parser checkpoints for a given text index. Used for efficient parser
    /// backtracking similar to packrat parsing. See comments above in
    /// [`expr_with_paren`].
    memo: MemoArena,
}

/// A single token returned from the lexer with a cached [`SyntaxKind`] and a
/// record of preceding trivia.
#[derive(Debug, Clone)]
struct Token {
    /// The [`SyntaxKind`] of the current token.
    kind: SyntaxKind,
    /// The [`SyntaxNode`] of the current token, ready to be eaten and pushed
    /// onto the end of `nodes`.
    node: SyntaxNode,
    /// The number of preceding trivia before this token.
    n_trivia: usize,
    /// Whether this token's preceding trivia contained a newline.
    newline: Option<Newline>,
    /// The index into `text` of the start of our current token (the end is
    /// stored as the lexer's cursor).
    start: usize,
    /// The index into `text` of the end of the previous token.
    prev_end: usize,
}

/// Information about newlines in a group of trivia.
#[derive(Debug, Copy, Clone)]
struct Newline {
    /// The column of the start of the next token in its line.
    column: Option<usize>,
    /// Whether any of our newlines were paragraph breaks.
    parbreak: bool,
}

/// How to proceed with parsing when at a newline.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum AtNewline {
    /// Continue at newlines.
    Continue,
    /// Stop at any newline.
    Stop,
    /// Continue only if there is a continuation with `else` or `.` (Code only).
    ContextualContinue,
    /// Stop only at a parbreak, not normal newlines (Markup only).
    StopParBreak,
    /// Require that the token's column be greater or equal to a column (Markup
    /// only). If this is `0`, acts like `Continue`; if this is `usize::MAX`,
    /// acts like `Stop`.
    RequireColumn(usize),
}

impl AtNewline {
    /// Whether to stop at a newline or continue based on the current context.
    fn stop_at(self, Newline { column, parbreak }: Newline, kind: SyntaxKind) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            AtNewline::Continue => false,
            AtNewline::Stop => true,
            AtNewline::ContextualContinue => match kind {
                SyntaxKind::Else | SyntaxKind::Dot => false,
                _ => true,
            },
            AtNewline::StopParBreak => parbreak,
            AtNewline::RequireColumn(min_col) => {
                // When the column is `None`, the newline doesn't start a
                // column, and we continue parsing. This may happen on the
                // boundary of syntax modes, since we only report a column in
                // Markup.
                column.is_some_and(|column| column <= min_col)
            }
        }
    }
}

/// A marker representing a node's position in the parser. Mainly used for
/// wrapping, but can also index into the parser to access the node, like
/// `p[m]`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Marker(usize);

// Index into the parser with markers.
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

/// Creating/Consuming the parser and getting info about the current token.
impl<'s> Parser<'s> {
    /// Create a new parser starting from the given text offset and syntax mode.
    fn new(text: &'s str, offset: usize, mode: SyntaxMode) -> Self {
        let mut lexer = Lexer::new(text, mode);
        lexer.jump(offset);
        let nl_mode = AtNewline::Continue;
        let mut nodes = vec![];
        let token = Self::lex(&mut nodes, &mut lexer, nl_mode);
        Self {
            text,
            lexer,
            nl_mode,
            token,
            balanced: true,
            nodes,
            memo: Default::default(),
        }
    }

    /// Consume the parser, yielding the full vector of parsed SyntaxNodes.
    fn finish(self) -> Vec<SyntaxNode> {
        self.nodes
    }

    /// Consume the parser, generating a single top-level node.
    fn finish_into(self, kind: SyntaxKind) -> SyntaxNode {
        assert!(self.at(SyntaxKind::End));
        SyntaxNode::inner(kind, self.finish())
    }

    /// Similar to a `peek()` function: returns the `kind` of the next token to
    /// be eaten.
    fn current(&self) -> SyntaxKind {
        self.token.kind
    }

    /// Whether the current token is a given [`SyntaxKind`].
    fn at(&self, kind: SyntaxKind) -> bool {
        self.token.kind == kind
    }

    /// Whether the current token is contained in a [`SyntaxSet`].
    fn at_set(&self, set: SyntaxSet) -> bool {
        set.contains(self.token.kind)
    }

    /// Whether we're at the end of the token stream.
    ///
    /// Note: This might be a fake end due to the newline mode.
    fn end(&self) -> bool {
        self.at(SyntaxKind::End)
    }

    /// If we're at the given `kind` with no preceding trivia tokens.
    fn directly_at(&self, kind: SyntaxKind) -> bool {
        self.token.kind == kind && !self.had_trivia()
    }

    /// Whether `token` had any preceding trivia.
    fn had_trivia(&self) -> bool {
        self.token.n_trivia > 0
    }

    /// Whether `token` had a newline among any of its preceding trivia.
    fn had_newline(&self) -> bool {
        self.token.newline.is_some()
    }

    /// The number of characters until the most recent newline from the start of
    /// the current token. Uses a cached value from the newline mode if present.
    fn current_column(&self) -> usize {
        self.token
            .newline
            .and_then(|newline| newline.column)
            .unwrap_or_else(|| self.lexer.column(self.token.start))
    }

    /// The current token's text.
    fn current_text(&self) -> &'s str {
        &self.text[self.token.start..self.current_end()]
    }

    /// The offset into `text` of the current token's start.
    fn current_start(&self) -> usize {
        self.token.start
    }

    /// The offset into `text` of the current token's end.
    fn current_end(&self) -> usize {
        self.lexer.cursor()
    }

    /// The offset into `text` of the previous token's end.
    fn prev_end(&self) -> usize {
        self.token.prev_end
    }
}

// The main parsing interface for generating tokens and eating/modifying nodes.
impl<'s> Parser<'s> {
    /// A marker that will point to the current token in the parser once it's
    /// been eaten.
    fn marker(&self) -> Marker {
        Marker(self.nodes.len())
    }

    /// A marker that will point to first trivia before this token in the
    /// parser (or the token itself if no trivia precede it).
    fn before_trivia(&self) -> Marker {
        Marker(self.nodes.len() - self.token.n_trivia)
    }

    /// Eat the current node and return a reference for in-place mutation.
    #[track_caller]
    fn eat_and_get(&mut self) -> &mut SyntaxNode {
        let offset = self.nodes.len();
        self.eat();
        &mut self.nodes[offset]
    }

    /// Eat the token if at `kind`. Returns `true` if eaten.
    ///
    /// Note: In Math and Code, this will ignore trivia in front of the
    /// `kind`, To forbid skipping trivia, consider using `eat_if_direct`.
    fn eat_if(&mut self, kind: SyntaxKind) -> bool {
        let at = self.at(kind);
        if at {
            self.eat();
        }
        at
    }

    /// Assert that we are at the given [`SyntaxKind`] and eat it. This should
    /// be used when moving between functions that expect to start with a
    /// specific token.
    #[track_caller]
    fn assert(&mut self, kind: SyntaxKind) {
        assert_eq!(self.token.kind, kind);
        self.eat();
    }

    /// Convert the current token's [`SyntaxKind`] and eat it.
    fn convert_and_eat(&mut self, kind: SyntaxKind) {
        // Only need to replace the node here.
        self.token.node.convert_to_kind(kind);
        self.eat();
    }

    /// Eat the current token by saving it to the `nodes` vector, then move
    /// the lexer forward to prepare a new token.
    fn eat(&mut self) {
        self.nodes.push(std::mem::take(&mut self.token.node));
        self.token = Self::lex(&mut self.nodes, &mut self.lexer, self.nl_mode);
    }

    /// Detach the parsed trivia nodes from this token (but not newline info) so
    /// that subsequent wrapping will include the trivia.
    fn flush_trivia(&mut self) {
        self.token.n_trivia = 0;
        self.token.prev_end = self.token.start;
    }

    /// Wrap the nodes from a marker up to (but excluding) the current token in
    /// a new [inner node](`SyntaxNode::inner`) of the given kind. This is an
    /// easy interface for creating nested syntax nodes _after_ having parsed
    /// their children.
    fn wrap(&mut self, from: Marker, kind: SyntaxKind) {
        let to = self.before_trivia().0;
        let from = from.0.min(to);
        let children = self.nodes.drain(from..to).collect();
        self.nodes.insert(from, SyntaxNode::inner(kind, children));
    }

    /// Parse within the [`SyntaxMode`] for subsequent tokens (does not change the
    /// current token). This may re-lex the final token on exit.
    ///
    /// This function effectively repurposes the call stack as a stack of modes.
    fn enter_modes(
        &mut self,
        mode: SyntaxMode,
        stop: AtNewline,
        func: impl FnOnce(&mut Parser<'s>),
    ) {
        let previous = self.lexer.mode();
        self.lexer.set_mode(mode);
        self.with_nl_mode(stop, func);
        if mode != previous {
            self.lexer.set_mode(previous);
            self.lexer.jump(self.token.prev_end);
            self.nodes.truncate(self.nodes.len() - self.token.n_trivia);
            self.token = Self::lex(&mut self.nodes, &mut self.lexer, self.nl_mode);
        }
    }

    /// Parse within the [`AtNewline`] mode for subsequent tokens (does not
    /// change the current token). This may re-lex the final token on exit.
    ///
    /// This function effectively repurposes the call stack as a stack of modes.
    fn with_nl_mode(&mut self, mode: AtNewline, func: impl FnOnce(&mut Parser<'s>)) {
        let previous = self.nl_mode;
        self.nl_mode = mode;
        func(self);
        self.nl_mode = previous;
        if let Some(newline) = self.token.newline
            && mode != previous
        {
            // Restore our actual token's kind or insert a fake end.
            let actual_kind = self.token.node.kind();
            if self.nl_mode.stop_at(newline, actual_kind) {
                self.token.kind = SyntaxKind::End;
            } else {
                self.token.kind = actual_kind;
            }
        }
    }

    /// Move the lexer forward and prepare the current token. In Code, this
    /// might insert a temporary [`SyntaxKind::End`] based on our newline mode.
    ///
    /// This is not a method on `self` because we need a valid token before we
    /// can initialize the parser.
    fn lex(nodes: &mut Vec<SyntaxNode>, lexer: &mut Lexer, nl_mode: AtNewline) -> Token {
        let prev_end = lexer.cursor();
        let mut start = prev_end;
        let (mut kind, mut node) = lexer.next();
        let mut n_trivia = 0;
        let mut had_newline = false;
        let mut parbreak = false;

        while kind.is_trivia() {
            had_newline |= lexer.newline(); // Newlines are always trivia.
            parbreak |= kind == SyntaxKind::Parbreak;
            n_trivia += 1;
            nodes.push(node);
            start = lexer.cursor();
            (kind, node) = lexer.next();
        }

        let newline = if had_newline {
            let column =
                (lexer.mode() == SyntaxMode::Markup).then(|| lexer.column(start));
            let newline = Newline { column, parbreak };
            if nl_mode.stop_at(newline, kind) {
                // Insert a temporary `SyntaxKind::End` to halt the parser.
                // The actual kind will be restored from `node` later.
                kind = SyntaxKind::End;
            }
            Some(newline)
        } else {
            None
        };

        Token { kind, node, n_trivia, newline, start, prev_end }
    }
}

/// Extra parser state for efficiently recovering from mispredicted parses.
///
/// This is the same idea as packrat parsing, but we use it only in the limited
/// case of parenthesized structures. See [`expr_with_paren`] for more.
#[derive(Default)]
struct MemoArena {
    /// A single arena of previously parsed nodes (to reduce allocations).
    /// Memoized ranges refer to unique sections of the arena.
    arena: Vec<SyntaxNode>,
    /// A map from the parser's current position to a range of previously parsed
    /// nodes in the arena and a checkpoint of the parser's state. These allow
    /// us to reset the parser to avoid parsing the same location again.
    memo_map: FxHashMap<MemoKey, (Range<usize>, PartialState)>,
}

/// A type alias for the memo key so it doesn't get confused with other usizes.
///
/// The memo is keyed by the index into `text` of the current token's start.
type MemoKey = usize;

/// A checkpoint of the parser which can fully restore it to a previous state.
struct Checkpoint {
    node_len: usize,
    state: PartialState,
}

/// State needed to restore the parser's current token and the lexer (but not
/// the nodes vector).
#[derive(Clone)]
struct PartialState {
    cursor: usize,
    lex_mode: SyntaxMode,
    token: Token,
}

/// The Memoization interface.
impl Parser<'_> {
    /// Store the already parsed nodes and the parser state into the memo map by
    /// extending the arena and storing the extended range and a checkpoint.
    fn memoize_parsed_nodes(&mut self, key: MemoKey, prev_len: usize) {
        let Checkpoint { state, node_len } = self.checkpoint();
        let memo_start = self.memo.arena.len();
        self.memo.arena.extend_from_slice(&self.nodes[prev_len..node_len]);
        let arena_range = memo_start..self.memo.arena.len();
        self.memo.memo_map.insert(key, (arena_range, state));
    }

    /// Try to load a memoized result, return `None` if we did or `Some` (with a
    /// checkpoint and a key for the memo map) if we didn't.
    fn restore_memo_or_checkpoint(&mut self) -> Option<(MemoKey, Checkpoint)> {
        // We use the starting index of the current token as our key.
        let key: MemoKey = self.current_start();
        match self.memo.memo_map.get(&key).cloned() {
            Some((range, state)) => {
                self.nodes.extend_from_slice(&self.memo.arena[range]);
                // It's important that we don't truncate the nodes vector since
                // it may have grown or shrunk (due to other memoization or
                // error reporting) since we made this checkpoint.
                self.restore_partial(state);
                None
            }
            None => Some((key, self.checkpoint())),
        }
    }

    /// Restore the parser to the state at a checkpoint.
    fn restore(&mut self, checkpoint: Checkpoint) {
        self.nodes.truncate(checkpoint.node_len);
        self.restore_partial(checkpoint.state);
    }

    /// Restore parts of the checkpoint excluding the nodes vector.
    fn restore_partial(&mut self, state: PartialState) {
        self.lexer.jump(state.cursor);
        self.lexer.set_mode(state.lex_mode);
        self.token = state.token;
    }

    /// Save a checkpoint of the parser state.
    fn checkpoint(&self) -> Checkpoint {
        let node_len = self.nodes.len();
        let state = PartialState {
            cursor: self.lexer.cursor(),
            lex_mode: self.lexer.mode(),
            token: self.token.clone(),
        };
        Checkpoint { node_len, state }
    }
}

/// Functions for eating expected or unexpected tokens and generating errors if
/// we don't get what we expect.
impl Parser<'_> {
    /// Consume the given `kind` or produce an error.
    fn expect(&mut self, kind: SyntaxKind) -> bool {
        let at = self.at(kind);
        if at {
            self.eat();
        } else if kind == SyntaxKind::Ident && self.token.kind.is_keyword() {
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

    /// Whether the last non-trivia node is an error.
    fn after_error(&mut self) -> bool {
        let m = self.before_trivia();
        m.0 > 0 && self.nodes[m.0 - 1].kind().is_error()
    }

    /// Produce an error that the given `thing` was expected at the position
    /// of the marker `m`.
    fn expected_at(&mut self, m: Marker, thing: &str) {
        let error =
            SyntaxNode::error(SyntaxError::new(eco_format!("expected {thing}")), "");
        self.nodes.insert(m.0, error);
    }

    /// Add a hint to a trailing error.
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
        self.balanced &= !self.token.kind.is_grouping();
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
