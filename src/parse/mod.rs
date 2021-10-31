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

use crate::source::SourceFile;
use crate::syntax::*;
use crate::util::EcoString;

/// Parse a source file.
pub fn parse(source: &SourceFile) -> Rc<GreenNode> {
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
        if let Some(node) = p.last_child() {
            at_start &= matches!(node.kind(),
                &NodeKind::Space(_) | &NodeKind::Parbreak |
                &NodeKind::LineComment | &NodeKind::BlockComment
            );
        }
    }

    p.end(NodeKind::Markup);
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
        | NodeKind::UnicodeEscape(_) => p.eat(),

        NodeKind::Eq if *at_start => heading(p),
        NodeKind::ListBullet if *at_start => list_node(p),
        NodeKind::EnumNumbering(_) if *at_start => enum_node(p),

        // Line-based markup that is not currently at the start of the line.
        NodeKind::Eq | NodeKind::ListBullet | NodeKind::EnumNumbering(_) => {
            p.convert(NodeKind::Text(p.peek_src().into()))
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
            expr_with(p, true, 0);
            if stmt && p.success() && !p.eof() {
                p.expected_at("semicolon or line break");
            }
            p.end_group();
        }

        // Block and template.
        NodeKind::LeftBrace => block(p),
        NodeKind::LeftBracket => template(p),

        // Comments.
        NodeKind::LineComment | NodeKind::BlockComment | NodeKind::Error(_, _) => p.eat(),

        _ => {
            *at_start = false;
            p.unexpected();
        }
    };
}

/// Parse a heading.
fn heading(p: &mut Parser) {
    p.start();
    p.start();
    p.eat_assert(&NodeKind::Eq);

    // Count depth.
    let mut level: usize = 1;
    while p.eat_if(&NodeKind::Eq) {
        level += 1;
    }

    if level > 6 {
        p.lift();
        p.end(NodeKind::Text(EcoString::from('=').repeat(level)));
    } else {
        p.end(NodeKind::HeadingLevel(level as u8));
        let column = p.column(p.prev_end());
        markup_indented(p, column);
        p.end(NodeKind::Heading);
    }
}

/// Parse a single list item.
fn list_node(p: &mut Parser) {
    p.start();
    p.eat_assert(&NodeKind::ListBullet);
    let column = p.column(p.prev_end());
    markup_indented(p, column);
    p.end(NodeKind::List);
}

/// Parse a single enum item.
fn enum_node(p: &mut Parser) {
    p.start();
    p.eat();
    let column = p.column(p.prev_end());
    markup_indented(p, column);
    p.end(NodeKind::Enum);
}

/// Parse an expression.
fn expr(p: &mut Parser) {
    expr_with(p, false, 0)
}

/// Parse an expression with operators having at least the minimum precedence.
///
/// If `atomic` is true, this does not parse binary operations and arrow
/// functions, which is exactly what we want in a shorthand expression directly
/// in markup.
///
/// Stops parsing at operations with lower precedence than `min_prec`,
fn expr_with(p: &mut Parser, atomic: bool, min_prec: usize) {
    p.start();
    let mut offset = p.child_count();
    // Start the unary expression.
    match p.eat_map(|x| UnOp::from_token(&x)) {
        Some(op) => {
            let prec = op.precedence();
            expr_with(p, atomic, prec);

            if p.may_lift_abort() {
                return;
            }

            p.end_and_start_with(NodeKind::Unary);
        }
        None => {
            primary(p, atomic);
            if p.may_lift_abort() {
                return;
            }
        }
    };

    loop {
        // Exclamation mark, parenthesis or bracket means this is a function
        // call.
        if matches!(
            p.peek_direct(),
            Some(NodeKind::LeftParen | NodeKind::LeftBracket)
        ) {
            call(p, p.child_count() - offset);
            continue;
        }

        if p.peek() == Some(&NodeKind::With) {
            with_expr(p, p.child_count() - offset);

            if p.may_lift_abort() {
                return;
            }
        }

        if atomic {
            p.lift();
            break;
        }

        let op = match p.peek().and_then(BinOp::from_token) {
            Some(binop) => binop,
            None => {
                p.lift();
                break;
            }
        };

        let mut prec = op.precedence();
        if prec < min_prec {
            p.lift();
            break;
        }

        p.eat();

        match op.associativity() {
            Associativity::Left => prec += 1,
            Associativity::Right => {}
        }

        expr_with(p, atomic, prec);

        if !p.success() {
            p.lift();
            break;
        }

        offset = p.end_and_start_with(NodeKind::Binary).0;
    }
}

/// Parse a primary expression.
fn primary(p: &mut Parser, atomic: bool) {
    if literal(p) {
        return;
    }

    match p.peek() {
        // Things that start with an identifier.
        Some(NodeKind::Ident(_)) => {
            // Start closure params.
            p.start();
            p.eat();

            // Arrow means this is a closure's lone parameter.
            if !atomic && p.peek() == Some(&NodeKind::Arrow) {
                p.end_and_start_with(NodeKind::ClosureParams);
                p.eat();

                expr(p);

                p.end_or_abort(NodeKind::Closure);
            } else {
                p.lift();
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
        }

        // Nothing.
        _ => {
            p.expected("expression");
            p.unsuccessful();
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
fn parenthesized(p: &mut Parser) {
    let offset = p.child_count();
    p.start();
    p.start_group(Group::Paren, TokenMode::Code);
    let colon = p.eat_if(&NodeKind::Colon);
    let kind = collection(p).0;
    p.end_group();
    let token_count = p.child_count() - offset;

    // Leading colon makes this a (empty) dictionary.
    if colon {
        p.lift();
        dict(p, token_count);
        return;
    }

    // Arrow means this is a closure's parameter list.
    if p.peek() == Some(&NodeKind::Arrow) {
        p.start_with(token_count);
        params(p, 0, true);
        p.end(NodeKind::ClosureParams);

        p.eat_assert(&NodeKind::Arrow);

        expr(p);

        p.end_or_abort(NodeKind::Closure);
        return;
    }

    // Find out which kind of collection this is.
    match kind {
        CollectionKind::Group => p.end(NodeKind::Group),
        CollectionKind::Positional => {
            p.lift();
            array(p, token_count);
        }
        CollectionKind::Named => {
            p.lift();
            dict(p, token_count);
        }
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
    let mut missing_coma = None;

    while !p.eof() {
        let item_kind = item(p);
        if p.success() {
            if items == 0 && item_kind == NodeKind::Named {
                kind = CollectionKind::Named;
            }

            if item_kind == NodeKind::ParameterSink {
                has_comma = true;
            }

            items += 1;

            if let Some(pos) = missing_coma.take() {
                p.expected_at_child(pos, "comma");
            }

            if p.eof() {
                break;
            }

            if p.eat_if(&NodeKind::Comma) {
                has_comma = true;
            } else {
                missing_coma = Some(p.child_count());
            }
        }
    }

    if !has_comma && items == 1 && kind == CollectionKind::Positional {
        kind = CollectionKind::Group;
    }

    (kind, items)
}

/// Parse an expression or a named pair. Returns if this is a named pair.
fn item(p: &mut Parser) -> NodeKind {
    p.start();
    if p.eat_if(&NodeKind::Dots) {
        expr(p);

        p.end_or_abort(NodeKind::ParameterSink);
        return NodeKind::ParameterSink;
    }

    expr(p);

    if p.may_lift_abort() {
        return NodeKind::None;
    }

    if p.eat_if(&NodeKind::Colon) {
        let child = p.child(1).unwrap();
        if matches!(child.kind(), &NodeKind::Ident(_)) {
            expr(p);
            p.end_or_abort(NodeKind::Named);
        } else {
            p.wrap(
                1,
                NodeKind::Error(ErrorPosition::Full, "expected identifier".into()),
            );

            expr(p);
            p.end(NodeKind::Named);
            p.unsuccessful();
        }

        NodeKind::Named
    } else {
        p.lift();
        p.last_child().unwrap().kind().clone()
    }
}

/// Convert a collection into an array, producing errors for anything other than
/// expressions.
fn array(p: &mut Parser, items: usize) {
    p.filter_children(
        p.child_count() - items,
        |x| match x.kind() {
            NodeKind::Named | NodeKind::ParameterSink => false,
            _ => true,
        },
        |kind| match kind {
            NodeKind::Named => (
                ErrorPosition::Full,
                "expected expression, found named pair".into(),
            ),
            NodeKind::ParameterSink => {
                (ErrorPosition::Full, "spreading is not allowed here".into())
            }
            _ => unreachable!(),
        },
    );

    p.convert_with(items, NodeKind::Array);
}

/// Convert a collection into a dictionary, producing errors for anything other
/// than named pairs.
fn dict(p: &mut Parser, items: usize) {
    p.filter_children(
        p.child_count() - items,
        |x| {
            x.kind() == &NodeKind::Named
                || x.kind().is_parenthesis()
                || x.kind() == &NodeKind::Comma
                || x.kind() == &NodeKind::Colon
        },
        |kind| match kind {
            NodeKind::ParameterSink => {
                (ErrorPosition::Full, "spreading is not allowed here".into())
            }
            _ => (
                ErrorPosition::Full,
                "expected named pair, found expression".into(),
            ),
        },
    );
    p.convert_with(items, NodeKind::Dict);
}

/// Convert a collection into a list of parameters, producing errors for
/// anything other than identifiers, spread operations and named pairs.
fn params(p: &mut Parser, count: usize, allow_parens: bool) {
    p.filter_children(
        count,
        |x| match x.kind() {
                NodeKind::Named | NodeKind::Comma | NodeKind::Ident(_) => true,
                NodeKind::ParameterSink => matches!(
                    x.children().last().map(|x| x.kind()),
                    Some(&NodeKind::Ident(_))
                ),
                _ => false,
            }
            || (allow_parens && x.kind().is_parenthesis()),
        |_| (ErrorPosition::Full, "expected identifier".into()),
    );
}

// Parse a template block: `[...]`.
fn template(p: &mut Parser) {
    p.start();
    p.start_group(Group::Bracket, TokenMode::Markup);
    markup(p);
    p.end_group();
    p.end(NodeKind::Template);
}

/// Parse a code block: `{...}`.
fn block(p: &mut Parser) {
    p.start();
    p.start_group(Group::Brace, TokenMode::Code);
    while !p.eof() {
        p.start_group(Group::Stmt, TokenMode::Code);
        expr(p);
        if p.success() {
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
}

/// Parse a function call.
fn call(p: &mut Parser, callee: usize) {
    p.start_with(callee);
    match p.peek_direct() {
        Some(NodeKind::LeftParen) | Some(NodeKind::LeftBracket) => args(p, true),
        _ => {
            p.expected_at("argument list");
            p.may_end_abort(NodeKind::Call);
            return;
        }
    };

    p.end(NodeKind::Call);
}

/// Parse the arguments to a function call.
fn args(p: &mut Parser, allow_template: bool) {
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
}

/// Parse a with expression.
fn with_expr(p: &mut Parser, preserve: usize) {
    p.start_with(preserve);
    p.eat_assert(&NodeKind::With);

    if p.peek() == Some(&NodeKind::LeftParen) {
        args(p, false);
        p.end(NodeKind::WithExpr);
    } else {
        p.expected("argument list");
        p.may_end_abort(NodeKind::WithExpr);
    }
}

/// Parse a let expression.
fn let_expr(p: &mut Parser) {
    p.start();
    p.eat_assert(&NodeKind::Let);

    let offset = p.child_count();
    ident(p);
    if p.may_end_abort(NodeKind::LetExpr) {
        return;
    }

    if p.peek() == Some(&NodeKind::With) {
        with_expr(p, p.child_count() - offset);
    } else {
        // If a parenthesis follows, this is a function definition.
        let has_params = if p.peek_direct() == Some(&NodeKind::LeftParen) {
            p.start();
            p.start_group(Group::Paren, TokenMode::Code);
            let offset = p.child_count();
            collection(p);
            params(p, offset, true);
            p.end_group();
            p.end(NodeKind::ClosureParams);
            true
        } else {
            false
        };

        if p.eat_if(&NodeKind::Eq) {
            expr(p);
        } else if has_params {
            // Function definitions must have a body.
            p.expected_at("body");
        }

        // Rewrite into a closure expression if it's a function definition.
        if has_params {
            if p.may_end_abort(NodeKind::LetExpr) {
                return;
            }

            p.convert_with(p.child_count() - offset, NodeKind::Closure);
        }
    }

    p.end(NodeKind::LetExpr);
}

/// Parse an if expresion.
fn if_expr(p: &mut Parser) {
    p.start();
    p.eat_assert(&NodeKind::If);

    expr(p);
    if p.may_end_abort(NodeKind::IfExpr) {
        return;
    }

    body(p);
    if p.may_end_abort(NodeKind::IfExpr) {
        // Expected function body.
        return;
    }

    if p.eat_if(&NodeKind::Else) {
        if p.peek() == Some(&NodeKind::If) {
            if_expr(p);
        } else {
            body(p);
        }
    }

    p.end(NodeKind::IfExpr);
}

/// Parse a while expresion.
fn while_expr(p: &mut Parser) {
    p.start();
    p.eat_assert(&NodeKind::While);

    expr(p);

    if p.may_end_abort(NodeKind::WhileExpr) {
        return;
    }

    body(p);
    if !p.may_end_abort(NodeKind::WhileExpr) {
        p.end(NodeKind::WhileExpr);
    }
}

/// Parse a for expression.
fn for_expr(p: &mut Parser) {
    p.start();
    p.eat_assert(&NodeKind::For);

    for_pattern(p);

    if p.may_end_abort(NodeKind::ForExpr) {
        return;
    }

    if p.eat_expect(&NodeKind::In) {
        expr(p);

        if p.may_end_abort(NodeKind::ForExpr) {
            return;
        }

        body(p);

        if !p.may_end_abort(NodeKind::ForExpr) {
            p.end(NodeKind::ForExpr);
        }
    } else {
        p.unsuccessful();
        p.may_end_abort(NodeKind::ForExpr);
    }
}

/// Parse a for loop pattern.
fn for_pattern(p: &mut Parser) {
    p.start();
    ident(p);

    if p.may_end_abort(NodeKind::ForPattern) {
        return;
    }

    if p.peek() == Some(&NodeKind::Comma) {
        p.eat();

        ident(p);

        if p.may_end_abort(NodeKind::ForPattern) {
            return;
        }
    }

    p.end(NodeKind::ForPattern);
}

/// Parse an import expression.
fn import_expr(p: &mut Parser) {
    p.start();
    p.eat_assert(&NodeKind::Import);

    if !p.eat_if(&NodeKind::Star) {
        // This is the list of identifiers scenario.
        p.start();
        p.start_group(Group::Imports, TokenMode::Code);
        let offset = p.child_count();
        let items = collection(p).1;
        if items == 0 {
            p.expected_at("import items");
        }
        p.end_group();

        p.filter_children(
            offset,
            |n| matches!(n.kind(), NodeKind::Ident(_) | NodeKind::Comma),
            |_| (ErrorPosition::Full, "expected identifier".into()),
        );
        p.end(NodeKind::ImportItems);
    };

    if p.eat_expect(&NodeKind::From) {
        expr(p);
    }

    p.end(NodeKind::ImportExpr);
}

/// Parse an include expression.
fn include_expr(p: &mut Parser) {
    p.start();
    p.eat_assert(&NodeKind::Include);

    expr(p);
    p.end(NodeKind::IncludeExpr);
}

/// Parse an identifier.
fn ident(p: &mut Parser) {
    match p.peek() {
        Some(NodeKind::Ident(_)) => p.eat(),
        _ => {
            p.expected("identifier");
            p.unsuccessful();
        }
    }
}

/// Parse a control flow body.
fn body(p: &mut Parser) {
    match p.peek() {
        Some(NodeKind::LeftBracket) => template(p),
        Some(NodeKind::LeftBrace) => block(p),
        _ => {
            p.expected_at("body");
            p.unsuccessful();
        }
    }
}
