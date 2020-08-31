//! Parsing of source code into syntax trees.

use std::str::FromStr;

use super::decoration::Decoration;
use super::span::{Pos, Span, Spanned};
use super::tokens::{is_newline_char, Token, TokenMode, Tokens};
use super::tree::{CallExpr, Code, Expr, SyntaxNode, SyntaxTree, TableExpr};
use super::Ident;
use crate::color::RgbaColor;
use crate::compute::table::SpannedEntry;
use crate::{Feedback, Pass};

/// Parse a string of source code.
pub fn parse(src: &str) -> Pass<SyntaxTree> {
    Parser::new(src).parse()
}

struct Parser<'s> {
    tokens: Tokens<'s>,
    peeked: Option<Option<Spanned<Token<'s>>>>,
    delimiters: Vec<(Pos, Token<'static>)>,
    feedback: Feedback,
}

impl<'s> Parser<'s> {
    fn new(src: &'s str) -> Self {
        Self {
            tokens: Tokens::new(src, TokenMode::Body),
            peeked: None,
            delimiters: vec![],
            feedback: Feedback::new(),
        }
    }

    fn parse(mut self) -> Pass<SyntaxTree> {
        let tree = self.parse_body_contents();
        Pass::new(tree, self.feedback)
    }
}

// Typesetting content.
impl Parser<'_> {
    fn parse_body_contents(&mut self) -> SyntaxTree {
        let mut tree = SyntaxTree::new();

        while let Some(token) = self.peek() {
            tree.push(match token.v {
                // Starting from two newlines counts as a paragraph break, a single
                // newline does not.
                Token::Space(newlines) => self.with_span(if newlines < 2 {
                    SyntaxNode::Spacing
                } else {
                    SyntaxNode::Parbreak
                }),

                Token::LineComment(_) | Token::BlockComment(_) => {
                    self.eat();
                    continue;
                }

                Token::LeftBracket => {
                    self.parse_bracket_call(false).map(SyntaxNode::Call)
                }

                Token::Star => self.with_span(SyntaxNode::ToggleBolder),
                Token::Underscore => self.with_span(SyntaxNode::ToggleItalic),
                Token::Backslash => self.with_span(SyntaxNode::Linebreak),

                Token::Raw { raw, terminated } => {
                    if !terminated {
                        error!(
                            @self.feedback, Span::at(token.span.end),
                            "expected backtick",
                        );
                    }
                    self.with_span(SyntaxNode::Raw(unescape_raw(raw)))
                }

                Token::Code { lang, raw, terminated } => {
                    if !terminated {
                        error!(
                            @self.feedback, Span::at(token.span.end),
                            "expected backticks",
                        );
                    }

                    let lang = lang.and_then(|lang| {
                        if let Some(ident) = Ident::new(lang.v) {
                            Some(Spanned::new(ident, lang.span))
                        } else {
                            error!(@self.feedback, lang.span, "invalid identifier");
                            None
                        }
                    });

                    let mut lines = unescape_code(raw);
                    let block = lines.len() > 1;

                    if lines.last().map(|s| s.is_empty()).unwrap_or(false) {
                        lines.pop();
                    }

                    self.with_span(SyntaxNode::Code(Code { lang, lines, block }))
                }

                Token::Text(text) => self.with_span(SyntaxNode::Text(text.to_string())),

                Token::UnicodeEscape { sequence, terminated } => {
                    if !terminated {
                        error!(
                            @self.feedback, Span::at(token.span.end),
                            "expected closing brace",
                        );
                    }

                    if let Some(c) = unescape_char(sequence) {
                        self.with_span(SyntaxNode::Text(c.to_string()))
                    } else {
                        self.eat();
                        error!(
                            @self.feedback, token.span,
                            "invalid unicode escape sequence",
                        );
                        continue;
                    }
                }

                unexpected => {
                    self.eat();
                    error!(
                        @self.feedback, token.span,
                        "unexpected {}", unexpected.name(),
                    );
                    continue;
                }
            });
        }

        tree
    }
}

// Function calls.
impl Parser<'_> {
    fn parse_bracket_call(&mut self, chained: bool) -> Spanned<CallExpr> {
        let before_bracket = self.pos();
        if !chained {
            self.start_group(Group::Bracket);
            self.tokens.push_mode(TokenMode::Header);
        }

        let before_name = self.pos();
        self.start_group(Group::Subheader);
        self.skip_white();
        let name = self.parse_ident().unwrap_or_else(|| {
            self.expected_found_or_at("function name", before_name);
            Spanned::new(Ident(String::new()), Span::at(before_name))
        });

        self.skip_white();

        let mut args = match self.eatv() {
            Some(Token::Colon) => self.parse_table_contents().0,
            Some(_) => {
                self.expected_at("colon", name.span.end);
                while self.eat().is_some() {}
                TableExpr::new()
            }
            None => TableExpr::new(),
        };

        self.end_group();
        self.skip_white();
        let (has_chained_child, end) = if self.peek().is_some() {
            let item = self.parse_bracket_call(true);
            let span = item.span;
            let t = vec![item.map(SyntaxNode::Call)];
            args.push(SpannedEntry::val(Spanned::new(Expr::Tree(t), span)));
            (true, span.end)
        } else {
            self.tokens.pop_mode();
            (false, self.end_group().end)
        };

        let start = if chained { before_name } else { before_bracket };
        let mut span = Span::new(start, end);

        if self.check(Token::LeftBracket) && !has_chained_child {
            self.start_group(Group::Bracket);
            self.tokens.push_mode(TokenMode::Body);

            let body = self.parse_body_contents();

            self.tokens.pop_mode();
            let body_span = self.end_group();

            let expr = Expr::Tree(body);
            args.push(SpannedEntry::val(Spanned::new(expr, body_span)));
            span.expand(body_span);
        }

        Spanned::new(CallExpr { name, args }, span)
    }

    fn parse_paren_call(&mut self, name: Spanned<Ident>) -> Spanned<CallExpr> {
        self.start_group(Group::Paren);
        let args = self.parse_table_contents().0;
        let args_span = self.end_group();
        let span = Span::merge(name.span, args_span);
        Spanned::new(CallExpr { name, args }, span)
    }
}

// Tables.
impl Parser<'_> {
    fn parse_table_contents(&mut self) -> (TableExpr, bool) {
        let mut table = TableExpr::new();
        let mut comma_and_keyless = true;

        while {
            self.skip_white();
            !self.eof()
        } {
            let (key, val) = if let Some(ident) = self.parse_ident() {
                self.skip_white();

                match self.peekv() {
                    Some(Token::Equals) => {
                        self.eat();
                        self.skip_white();
                        if let Some(value) = self.parse_expr() {
                            (Some(ident), value)
                        } else {
                            self.expected("value");
                            continue;
                        }
                    }

                    Some(Token::LeftParen) => {
                        let call = self.parse_paren_call(ident);
                        (None, call.map(Expr::Call))
                    }

                    _ => (None, ident.map(Expr::Ident)),
                }
            } else if let Some(value) = self.parse_expr() {
                (None, value)
            } else {
                self.expected("value");
                continue;
            };

            let behind = val.span.end;
            if let Some(key) = key {
                comma_and_keyless = false;
                table.insert(key.v.0, SpannedEntry::new(key.span, val));
                self.feedback
                    .decorations
                    .push(Spanned::new(Decoration::TableKey, key.span));
            } else {
                table.push(SpannedEntry::val(val));
            }

            if {
                self.skip_white();
                self.eof()
            } {
                break;
            }

            self.expect_at(Token::Comma, behind);
            comma_and_keyless = false;
        }

        let coercable = comma_and_keyless && !table.is_empty();
        (table, coercable)
    }
}

type Binop = fn(Box<Spanned<Expr>>, Box<Spanned<Expr>>) -> Expr;

// Expressions and values.
impl Parser<'_> {
    fn parse_expr(&mut self) -> Option<Spanned<Expr>> {
        self.parse_binops("summand", Self::parse_term, |token| match token {
            Token::Plus => Some(Expr::Add),
            Token::Hyphen => Some(Expr::Sub),
            _ => None,
        })
    }

    fn parse_term(&mut self) -> Option<Spanned<Expr>> {
        self.parse_binops("factor", Self::parse_factor, |token| match token {
            Token::Star => Some(Expr::Mul),
            Token::Slash => Some(Expr::Div),
            _ => None,
        })
    }

    /// Parse expression of the form `<operand> (<op> <operand>)*`.
    fn parse_binops(
        &mut self,
        operand_name: &str,
        mut parse_operand: impl FnMut(&mut Self) -> Option<Spanned<Expr>>,
        mut parse_op: impl FnMut(Token) -> Option<Binop>,
    ) -> Option<Spanned<Expr>> {
        let mut left = parse_operand(self)?;

        self.skip_white();
        while let Some(token) = self.peek() {
            if let Some(op) = parse_op(token.v) {
                self.eat();
                self.skip_white();

                if let Some(right) = parse_operand(self) {
                    let span = Span::merge(left.span, right.span);
                    let v = op(Box::new(left), Box::new(right));
                    left = Spanned::new(v, span);
                    self.skip_white();
                    continue;
                }

                error!(
                    @self.feedback, Span::merge(left.span, token.span),
                    "missing right {}", operand_name,
                );
            }
            break;
        }

        Some(left)
    }

    fn parse_factor(&mut self) -> Option<Spanned<Expr>> {
        if let Some(hyph) = self.check_eat(Token::Hyphen) {
            self.skip_white();
            if let Some(factor) = self.parse_factor() {
                let span = Span::merge(hyph.span, factor.span);
                Some(Spanned::new(Expr::Neg(Box::new(factor)), span))
            } else {
                error!(@self.feedback, hyph.span, "dangling minus");
                None
            }
        } else {
            self.parse_value()
        }
    }

    fn parse_value(&mut self) -> Option<Spanned<Expr>> {
        let Spanned { v: token, span } = self.peek()?;
        Some(match token {
            // This could be a function call or an identifier.
            Token::Ident(id) => {
                let name = Spanned::new(Ident(id.to_string()), span);
                self.eat();
                self.skip_white();
                if self.check(Token::LeftParen) {
                    self.parse_paren_call(name).map(Expr::Call)
                } else {
                    name.map(Expr::Ident)
                }
            }

            Token::Str { string, terminated } => {
                if !terminated {
                    self.expected_at("quote", span.end);
                }
                self.with_span(Expr::Str(unescape_string(string)))
            }

            Token::Bool(b) => self.with_span(Expr::Bool(b)),
            Token::Number(n) => self.with_span(Expr::Number(n)),
            Token::Length(s) => self.with_span(Expr::Length(s)),
            Token::Hex(s) => {
                if let Ok(color) = RgbaColor::from_str(s) {
                    self.with_span(Expr::Color(color))
                } else {
                    // Heal color by assuming black.
                    error!(@self.feedback, span, "invalid color");
                    let healed = RgbaColor::new_healed(0, 0, 0, 255);
                    self.with_span(Expr::Color(healed))
                }
            }

            // This could be a table or a parenthesized expression. We parse as
            // a table in any case and coerce the table into a value if it is
            // coercable (length 1 and no trailing comma).
            Token::LeftParen => {
                self.start_group(Group::Paren);
                let (table, coercable) = self.parse_table_contents();
                let span = self.end_group();

                let expr = if coercable {
                    table.into_values().next().expect("table is coercable").val.v
                } else {
                    Expr::Table(table)
                };

                Spanned::new(expr, span)
            }

            // This is a content expression.
            Token::LeftBrace => {
                self.start_group(Group::Brace);
                self.tokens.push_mode(TokenMode::Body);

                let tree = self.parse_body_contents();

                self.tokens.pop_mode();
                let span = self.end_group();
                Spanned::new(Expr::Tree(tree), span)
            }

            // This is a bracketed function call.
            Token::LeftBracket => {
                let call = self.parse_bracket_call(false);
                let tree = vec![call.map(SyntaxNode::Call)];
                Spanned::new(Expr::Tree(tree), span)
            }

            _ => return None,
        })
    }

    fn parse_ident(&mut self) -> Option<Spanned<Ident>> {
        self.peek().and_then(|token| match token.v {
            Token::Ident(id) => Some(self.with_span(Ident(id.to_string()))),
            _ => None,
        })
    }
}

// Error handling.
impl Parser<'_> {
    fn expect_at(&mut self, token: Token<'_>, pos: Pos) -> bool {
        if self.check(token) {
            self.eat();
            true
        } else {
            self.expected_at(token.name(), pos);
            false
        }
    }

    fn expected(&mut self, thing: &str) {
        if let Some(found) = self.eat() {
            error!(
                @self.feedback, found.span,
                "expected {}, found {}", thing, found.v.name(),
            );
        } else {
            error!(@self.feedback, Span::at(self.pos()), "expected {}", thing);
        }
    }

    fn expected_at(&mut self, thing: &str, pos: Pos) {
        error!(@self.feedback, Span::at(pos), "expected {}", thing);
    }

    fn expected_found_or_at(&mut self, thing: &str, pos: Pos) {
        if self.eof() {
            self.expected_at(thing, pos)
        } else {
            self.expected(thing);
        }
    }
}

// Parsing primitives.
impl<'s> Parser<'s> {
    fn start_group(&mut self, group: Group) {
        let start = self.pos();
        if let Some(start_token) = group.start() {
            self.assert(start_token);
        }
        self.delimiters.push((start, group.end()));
    }

    fn end_group(&mut self) -> Span {
        let peeked = self.peek();

        let (start, end_token) = self.delimiters.pop().expect("group was not started");

        if end_token != Token::Chain && peeked != None {
            self.delimiters.push((start, end_token));
            assert_eq!(peeked, None, "unfinished group");
        }

        match self.peeked.unwrap() {
            Some(token) if token.v == end_token => {
                self.peeked = None;
                Span::new(start, token.span.end)
            }
            _ => {
                let end = self.pos();
                if end_token != Token::Chain {
                    error!(
                        @self.feedback, Span::at(end),
                        "expected {}", end_token.name(),
                    );
                }
                Span::new(start, end)
            }
        }
    }

    fn skip_white(&mut self) {
        while matches!(
            self.peekv(),
            Some(Token::Space(_)) |
            Some(Token::LineComment(_)) |
            Some(Token::BlockComment(_))
        ) {
            self.eat();
        }
    }

    fn eatv(&mut self) -> Option<Token<'s>> {
        self.eat().map(Spanned::value)
    }

    fn peekv(&mut self) -> Option<Token<'s>> {
        self.peek().map(Spanned::value)
    }

    fn assert(&mut self, token: Token<'_>) {
        assert!(self.check_eat(token).is_some());
    }

    fn check_eat(&mut self, token: Token<'_>) -> Option<Spanned<Token<'s>>> {
        if self.check(token) { self.eat() } else { None }
    }

    /// Checks if the next token is of some kind
    fn check(&mut self, token: Token<'_>) -> bool {
        self.peekv() == Some(token)
    }

    fn with_span<T>(&mut self, v: T) -> Spanned<T> {
        let span = self.eat().expect("expected token").span;
        Spanned::new(v, span)
    }

    fn eof(&mut self) -> bool {
        self.peek().is_none()
    }

    fn eat(&mut self) -> Option<Spanned<Token<'s>>> {
        let token = self.peek()?;
        self.peeked = None;
        Some(token)
    }

    fn peek(&mut self) -> Option<Spanned<Token<'s>>> {
        let tokens = &mut self.tokens;
        let token = (*self.peeked.get_or_insert_with(|| tokens.next()))?;

        // Check for unclosed groups.
        if Group::is_delimiter(token.v) {
            if self.delimiters.iter().rev().any(|&(_, end)| token.v == end) {
                return None;
            }
        }

        Some(token)
    }

    fn pos(&self) -> Pos {
        self.peeked
            .flatten()
            .map(|s| s.span.start)
            .unwrap_or_else(|| self.tokens.pos())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Group {
    Paren,
    Bracket,
    Brace,
    Subheader,
}

impl Group {
    fn is_delimiter(token: Token<'_>) -> bool {
        matches!(
            token,
            Token::RightParen | Token::RightBracket | Token::RightBrace | Token::Chain
        )
    }

    fn start(self) -> Option<Token<'static>> {
        match self {
            Self::Paren => Some(Token::LeftParen),
            Self::Bracket => Some(Token::LeftBracket),
            Self::Brace => Some(Token::LeftBrace),
            Self::Subheader => None,
        }
    }

    fn end(self) -> Token<'static> {
        match self {
            Self::Paren => Token::RightParen,
            Self::Bracket => Token::RightBracket,
            Self::Brace => Token::RightBrace,
            Self::Subheader => Token::Chain,
        }
    }
}

fn unescape_string(string: &str) -> String {
    let mut iter = string.chars().peekable();
    let mut out = String::with_capacity(string.len());

    while let Some(c) = iter.next() {
        if c == '\\' {
            match iter.next() {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('u') if iter.peek() == Some(&'{') => {
                    iter.next();

                    let mut sequence = String::new();
                    let terminated = loop {
                        match iter.peek() {
                            // TODO: Feedback that closing brace is missing.
                            Some('}') => {
                                iter.next();
                                break true;
                            }
                            Some(&c) if c.is_ascii_hexdigit() => {
                                iter.next();
                                sequence.push(c);
                            }
                            _ => break false,
                        }
                    };

                    // TODO: Feedback that escape sequence is wrong.
                    if let Some(c) = unescape_char(&sequence) {
                        out.push(c);
                    } else {
                        out.push_str("\\u{");
                        out.push_str(&sequence);
                        if terminated {
                            out.push('}');
                        }
                    }
                }
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(c) => {
                    out.push('\\');
                    out.push(c);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }

    out
}

/// Unescape raw markup and split it into into lines.
fn unescape_raw(raw: &str) -> Vec<String> {
    let mut iter = raw.chars();
    let mut text = String::new();

    while let Some(c) = iter.next() {
        if c == '\\' {
            if let Some(c) = iter.next() {
                if c != '\\' && c != '`' {
                    text.push('\\');
                }

                text.push(c);
            } else {
                text.push('\\');
            }
        } else {
            text.push(c);
        }
    }

    split_lines(&text)
}

/// Unescape raw markup and split it into into lines.
fn unescape_code(raw: &str) -> Vec<String> {
    let mut iter = raw.chars().peekable();
    let mut text = String::new();
    let mut backticks = 0u32;
    let mut update_backtick_count;

    while let Some(c) = iter.next() {
        update_backtick_count = true;

        if c == '\\' && backticks > 0 {
            let mut tail = String::new();
            let mut escape_success = false;
            let mut backticks_after_slash = 0u32;

            while let Some(&s) = iter.peek() {
                match s {
                    '\\' => {
                        if backticks_after_slash == 0 {
                            tail.push('\\');
                        } else {
                            // Pattern like `\`\` should fail
                            // escape and just be printed verbantim.
                            break;
                        }
                    }
                    '`' => {
                        tail.push(s);
                        backticks_after_slash += 1;
                        if backticks_after_slash == 2 {
                            escape_success = true;
                            iter.next();
                            break;
                        }
                    }
                    _ => break,
                }

                iter.next();
            }

            if !escape_success {
                text.push(c);
                backticks = backticks_after_slash;
                update_backtick_count = false;
            } else {
                backticks = 0;
            }

            text.push_str(&tail);
        } else {
            text.push(c);
        }

        if update_backtick_count {
            if c == '`' {
                backticks += 1;
            } else {
                backticks = 0;
            }
        }
    }

    split_lines(&text)
}

/// Converts a hexademical sequence (without braces or "\u") into a character.
fn unescape_char(sequence: &str) -> Option<char> {
    u32::from_str_radix(sequence, 16).ok().and_then(std::char::from_u32)
}

fn split_lines(text: &str) -> Vec<String> {
    let mut iter = text.chars().peekable();
    let mut line = String::new();
    let mut lines = Vec::new();

    while let Some(c) = iter.next() {
        if is_newline_char(c) {
            if c == '\r' && iter.peek() == Some(&'\n') {
                iter.next();
            }

            lines.push(std::mem::take(&mut line));
        } else {
            line.push(c);
        }
    }

    lines.push(line);
    lines
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use crate::length::Length;
    use crate::syntax::tests::*;
    use Decoration::*;

    // ----------------------- Construct Syntax Nodes ----------------------- //

    use SyntaxNode::{
        Linebreak as L, Parbreak as P, Spacing as S, ToggleBolder as B, ToggleItalic as I,
    };

    fn T(text: &str) -> SyntaxNode {
        SyntaxNode::Text(text.to_string())
    }

    macro_rules! R {
        ($($line:expr),* $(,)?) => {
            SyntaxNode::Raw(vec![$($line.to_string()),*])
        };
    }

    macro_rules! C {
        (None, $($line:expr),* $(,)?) => {{
            let lines = vec![$($line.to_string()) ,*];
            SyntaxNode::Code(Code {
                lang: None,
                block: lines.len() > 1,
                lines,
            })
        }};
        (Some($lang:expr), $($line:expr),* $(,)?) => {{
            let lines = vec![$($line.to_string()) ,*];
            SyntaxNode::Code(Code {
                lang: Some(Into::<Spanned<&str>>::into($lang).map(|s| Ident(s.to_string()))),
                block: lines.len() > 1,
                lines,
            })
        }};
    }

    macro_rules! F {
        ($($tts:tt)*) => { SyntaxNode::Call(Call!(@$($tts)*)) }
    }

    // ------------------------ Construct Expressions ----------------------- //

    use Expr::{Bool, Color, Length as Len, Number as Num};

    fn Id(ident: &str) -> Expr {
        Expr::Ident(Ident(ident.to_string()))
    }
    fn Str(string: &str) -> Expr {
        Expr::Str(string.to_string())
    }

    macro_rules! Table {
        (@table=$table:expr,) => {};
        (@table=$table:expr, $key:expr => $value:expr $(, $($tts:tt)*)?) => {{
            let key = Into::<Spanned<&str>>::into($key);
            let val = Into::<Spanned<Expr>>::into($value);
            $table.insert(key.v, SpannedEntry::new(key.span, val));
            Table![@table=$table, $($($tts)*)?];
        }};
        (@table=$table:expr, $value:expr $(, $($tts:tt)*)?) => {
            let val = Into::<Spanned<Expr>>::into($value);
            $table.push(SpannedEntry::val(val));
            Table![@table=$table, $($($tts)*)?];
        };
        (@$($tts:tt)*) => {{
            #[allow(unused_mut)]
            let mut table = TableExpr::new();
            Table![@table=table, $($tts)*];
            table
        }};
        ($($tts:tt)*) => { Expr::Table(Table![@$($tts)*]) };
    }

    macro_rules! Tree {
        (@$($node:expr),* $(,)?) => {
            vec![$(Into::<Spanned<SyntaxNode>>::into($node)),*]
        };
        ($($tts:tt)*) => { Expr::Tree(Tree![@$($tts)*]) };
    }

    macro_rules! Call {
        (@$name:expr $(; $($tts:tt)*)?) => {{
            let name = Into::<Spanned<&str>>::into($name);
            CallExpr {
                name: name.map(|n| Ident(n.to_string())),
                args: Table![@$($($tts)*)?],
            }
        }};
        ($($tts:tt)*) => { Expr::Call(Call![@$($tts)*]) };
    }

    fn Neg<T: Into<Spanned<Expr>>>(e1: T) -> Expr {
        Expr::Neg(Box::new(e1.into()))
    }
    fn Add<T: Into<Spanned<Expr>>>(e1: T, e2: T) -> Expr {
        Expr::Add(Box::new(e1.into()), Box::new(e2.into()))
    }
    fn Sub<T: Into<Spanned<Expr>>>(e1: T, e2: T) -> Expr {
        Expr::Sub(Box::new(e1.into()), Box::new(e2.into()))
    }
    fn Mul<T: Into<Spanned<Expr>>>(e1: T, e2: T) -> Expr {
        Expr::Mul(Box::new(e1.into()), Box::new(e2.into()))
    }
    fn Div<T: Into<Spanned<Expr>>>(e1: T, e2: T) -> Expr {
        Expr::Div(Box::new(e1.into()), Box::new(e2.into()))
    }

    // ----------------------------- Test Macros ---------------------------- //

    // Test syntax trees with or without spans.
    macro_rules! t { ($($tts:tt)*) => {test!(@spans=false, $($tts)*)} }
    macro_rules! ts { ($($tts:tt)*) => {test!(@spans=true, $($tts)*)} }
    macro_rules! test {
        (@spans=$spans:expr, $src:expr => $($tts:tt)*) => {
            let exp = Tree![@$($tts)*];
            let pass = parse($src);
            check($src, exp, pass.output, $spans);
        };
    }

    // Test expressions.
    macro_rules! v {
        ($src:expr => $($tts:tt)*) => {
            t!(concat!("[val: ", $src, "]") => F!("val"; $($tts)*));
        }
    }

    // Test error messages.
    macro_rules! e {
        ($src:expr => $($tts:tt)*) => {
            let exp = vec![$($tts)*];
            let pass = parse($src);
            let found = pass.feedback.diagnostics.iter()
                .map(|s| s.as_ref().map(|e| e.message.as_str()))
                .collect::<Vec<_>>();
            check($src, exp, found, true);
        };
    }

    // Test decorations.
    macro_rules! d {
        ($src:expr => $($tts:tt)*) => {
            let exp = vec![$($tts)*];
            let pass = parse($src);
            check($src, exp, pass.feedback.decorations, true);
        };
    }

    // -------------------------------- Tests ------------------------------- //

    #[test]
    #[rustfmt::skip]
    fn test_unescape_strings() {
        fn test(string: &str, expected: &str) {
            assert_eq!(unescape_string(string), expected.to_string());
        }

        test(r#"hello world"#,  "hello world");
        test(r#"hello\nworld"#, "hello\nworld");
        test(r#"a\"bc"#,        "a\"bc");
        test(r#"a\u{2603}bc"#,  "a☃bc");
        test(r#"a\u{26c3bg"#,   "a𦰻g");
        test(r#"av\u{6797"#,    "av林");
        test(r#"a\\"#,          "a\\");
        test(r#"a\\\nbc"#,      "a\\\nbc");
        test(r#"a\tbc"#,        "a\tbc");
        test(r"🌎",             "🌎");
        test(r"🌎\",            r"🌎\");
        test(r"\🌎",            r"\🌎");
    }

    #[test]
    #[rustfmt::skip]
    fn test_unescape_raws() {
        fn test(raw: &str, expected: Vec<&str>) {
            assert_eq!(unescape_raw(raw), expected);
        }

        test("raw\\`",     vec!["raw`"]);
        test("raw\\\\`",   vec!["raw\\`"]);
        test("raw\ntext",  vec!["raw", "text"]);
        test("a\r\nb",     vec!["a", "b"]);
        test("a\n\nb",     vec!["a", "", "b"]);
        test("a\r\x0Bb",   vec!["a", "", "b"]);
        test("a\r\n\r\nb", vec!["a", "", "b"]);
        test("raw\\a",     vec!["raw\\a"]);
        test("raw\\",      vec!["raw\\"]);
    }

    #[test]
    #[rustfmt::skip]
    fn test_unescape_code() {
        fn test(raw: &str, expected: Vec<&str>) {
            assert_eq!(unescape_code(raw), expected);
        }

        test("code\\`",       vec!["code\\`"]);
        test("code`\\``",     vec!["code```"]);
        test("code`\\`a",     vec!["code`\\`a"]);
        test("code``hi`\\``", vec!["code``hi```"]);
        test("code`\\\\``",   vec!["code`\\``"]);
        test("code`\\`\\`go", vec!["code`\\`\\`go"]);
        test("code`\\`\\``",  vec!["code`\\```"]);
        test("code\ntext",    vec!["code", "text"]);
        test("a\r\nb",        vec!["a", "b"]);
        test("a\n\nb",        vec!["a", "", "b"]);
        test("a\r\x0Bb",      vec!["a", "", "b"]);
        test("a\r\n\r\nb",    vec!["a", "", "b"]);
        test("code\\a",       vec!["code\\a"]);
        test("code\\",        vec!["code\\"]);
    }

    #[test]
    fn test_parse_simple_nodes() {
        t!(""             => );
        t!("hi"           => T("hi"));
        t!("*hi"          => B, T("hi"));
        t!("hi_"          => T("hi"), I);
        t!("hi you"       => T("hi"), S, T("you"));
        t!("special~name" => T("special"), T("\u{00A0}"), T("name"));
        t!("special\\~name" => T("special"), T("~"), T("name"));
        t!("\\u{1f303}"   => T("🌃"));
        t!("\n\n\nhello"  => P, T("hello"));
        t!(r"a\ b"        => T("a"), L, S, T("b"));
        t!("`py`"         => R!["py"]);
        t!("`hi\nyou"     => R!["hi", "you"]);
        e!("`hi\nyou"     => s(1,3, 1,3, "expected backtick"));
        t!("`hi\\`du`"    => R!["hi`du"]);

        t!("```java System.out.print```" => C![Some("java"), "System.out.print"]);
        t!("``` console.log(\n\"alert\"\n)" => C![None, "console.log(", "\"alert\"", ")"]);
        t!("```typst \r\n Typst uses `\\`` to indicate code blocks" => C![
            Some("typst"), " Typst uses ``` to indicate code blocks"
        ]);

        e!("``` hi\nyou"      => s(1,3, 1,3,  "expected backticks"));
        e!("```🌍 hi\nyou```" => s(0,3, 0,4,  "invalid identifier"));
        e!("\\u{d421c809}"    => s(0,0, 0,12, "invalid unicode escape sequence"));
        e!("\\u{abc"          => s(0,6, 0,6, "expected closing brace"));
        t!("💜\n\n 🌍"       => T("💜"), P, T("🌍"));

        ts!("hi"   => s(0,0, 0,2, T("hi")));
        ts!("*Hi*" => s(0,0, 0,1, B), s(0,1, 0,3, T("Hi")), s(0,3, 0,4, B));
        ts!("💜\n\n 🌍" => s(0,0, 0,1, T("💜")), s(0,1, 2,1, P), s(2,1, 2,2, T("🌍")));
    }

    #[test]
    fn test_parse_comments() {
        // In body.
        t!("hi// you\nw"          => T("hi"), S, T("w"));
        t!("first//\n//\nsecond"  => T("first"), S, S, T("second"));
        t!("first//\n \nsecond"   => T("first"), P, T("second"));
        t!("first/*\n \n*/second" => T("first"), T("second"));
        e!("🌎\n*/n" => s(1,0, 1,2, "unexpected end of block comment"));

        // In header.
        t!("[val:/*12pt*/]" => F!("val"));
        t!("[val \n /* \n */:]" => F!("val"));
        e!("[val \n /* \n */:]" => );
        e!("[val : 12, /* \n */ 14]" => );
    }

    #[test]
    fn test_parse_groups() {
        e!("[)" => s(0,1, 0,2, "expected function name, found closing paren"),
                   s(0,2, 0,2, "expected closing bracket"));

        e!("[v:{]}" => s(0,4, 0,4, "expected closing brace"),
                       s(0,5, 0,6, "unexpected closing brace"));
    }

    #[test]
    fn test_parse_function_names() {
        // No closing bracket.
        t!("[" => F!(""));
        e!("[" => s(0,1, 0,1, "expected function name"),
                  s(0,1, 0,1, "expected closing bracket"));

        // No name.
        e!("[]"   => s(0,1, 0,1, "expected function name"));
        e!("[\"]" => s(0,1, 0,3, "expected function name, found string"),
                     s(0,3, 0,3, "expected closing bracket"));

        // A valid name.
        t!("[hi]"  => F!("hi"));
        t!("[  f]" => F!("f"));

        // An invalid name.
        e!("[12]"   => s(0,1, 0,3, "expected function name, found number"));
        e!("[  🌎]" => s(0,3, 0,4, "expected function name, found invalid token"));
    }

    #[test]
    fn test_parse_chaining() {
        // Things the parser has to make sense of
        t!("[hi: (5.0, 2.1 >> you]" => F!("hi"; Table![Num(5.0), Num(2.1)], Tree![F!("you")]));
        t!("[box >>][Hi]"           => F!("box"; Tree![T("Hi")]));
        t!("[box >> pad: 1pt][Hi]"  => F!("box"; Tree![
            F!("pad"; Len(Length::pt(1.0)), Tree!(T("Hi")))
        ]));
        t!("[bold: 400, >> emph >> sub: 1cm]" => F!("bold"; Num(400.0), Tree![
            F!("emph"; Tree!(F!("sub"; Len(Length::cm(1.0)))))
        ]));

        // Errors for unclosed / empty predecessor groups
        e!("[hi: (5.0, 2.1 >> you]" => s(0, 15, 0, 15, "expected closing paren"));
        e!("[>> abc]" => s(0, 1, 0, 1, "expected function name"));
    }

    #[test]
    fn test_parse_colon_starting_func_args() {
        // Just colon without args.
        e!("[val:]" => );

        // Wrong token.
        t!("[val=]"     => F!("val"));
        e!("[val=]"     => s(0,4, 0,4, "expected colon"));
        e!("[val/🌎:$]" => s(0,4, 0,4, "expected colon"));

        // String in invalid header without colon still parsed as string
        // Note: No "expected quote" error because not even the string was
        //       expected.
        e!("[val/\"]" => s(0,4, 0,4, "expected colon"),
                         s(0,7, 0,7, "expected closing bracket"));
    }

    #[test]
    fn test_parse_function_bodies() {
        t!("[val: 1][*Hi*]" => F!("val"; Num(1.0), Tree![B, T("Hi"), B]));
        e!(" [val][ */ ]"   => s(0,8, 0,10, "unexpected end of block comment"));

        // Raw in body.
        t!("[val][`Hi]`" => F!("val"; Tree![R!["Hi]"]]));
        e!("[val][`Hi]`" => s(0,11, 0,11, "expected closing bracket"));

        // Crazy.
        t!("[v][[v][v][v]]" => F!("v"; Tree![F!("v"; Tree![T("v")]), F!("v")]));

        // Spanned.
        ts!(" [box][Oh my]" =>
            s(0,0, 0,1, S),
            s(0,1, 0,13, F!(s(0,2, 0,5, "box");
                s(0,6, 0,13, Tree![
                    s(0,7, 0,9, T("Oh")), s(0,9, 0,10, S), s(0,10, 0,12, T("my"))
                ])
            ))
        );
    }

    #[test]
    fn test_parse_values() {
        // Simple.
        v!("_"         => Id("_"));
        v!("name"      => Id("name"));
        v!("α"         => Id("α"));
        v!("\"hi\""    => Str("hi"));
        v!("true"      => Bool(true));
        v!("false"     => Bool(false));
        v!("1.0e-4"    => Num(1e-4));
        v!("3.14"      => Num(3.14));
        v!("50%"       => Num(0.5));
        v!("4.5cm"     => Len(Length::cm(4.5)));
        v!("12e1pt"    => Len(Length::pt(12e1)));
        v!("#f7a20500" => Color(RgbaColor::new(0xf7, 0xa2, 0x05, 0x00)));
        v!("\"a\n[]\\\"string\"" => Str("a\n[]\"string"));

        // Content.
        v!("{_hi_}"        => Tree![I, T("hi"), I]);
        e!("[val: {_hi_}]" => );
        v!("[hi]"          => Tree![F!("hi")]);
        e!("[val: [hi]]"   => );

        // Healed colors.
        v!("#12345"            => Color(RgbaColor::new_healed(0, 0, 0, 0xff)));
        e!("[val: #12345]"     => s(0,6, 0,12, "invalid color"));
        e!("[val: #a5]"        => s(0,6, 0,9,  "invalid color"));
        e!("[val: #14b2ah]"    => s(0,6, 0,13, "invalid color"));
        e!("[val: #f075ff011]" => s(0,6, 0,16, "invalid color"));

        // Unclosed string.
        v!("\"hello"        => Str("hello]"));
        e!("[val: \"hello]" => s(0,13, 0,13, "expected quote"),
                               s(0,13, 0,13, "expected closing bracket"));

        // Spanned.
        ts!("[val: 1.4]" => s(0,0, 0,10, F!(s(0,1, 0,4, "val"); s(0,6, 0,9, Num(1.4)))));
    }

    #[test]
    fn test_parse_expressions() {
        // Coerced table.
        v!("(hi)" => Id("hi"));

        // Operations.
        v!("-1"          => Neg(Num(1.0)));
        v!("-- 1"        => Neg(Neg(Num(1.0))));
        v!("3.2in + 6pt" => Add(Len(Length::inches(3.2)), Len(Length::pt(6.0))));
        v!("5 - 0.01"    => Sub(Num(5.0), Num(0.01)));
        v!("(3mm * 2)"   => Mul(Len(Length::mm(3.0)), Num(2.0)));
        v!("12e-3cm/1pt" => Div(Len(Length::cm(12e-3)), Len(Length::pt(1.0))));

        // More complex.
        v!("(3.2in + 6pt)*(5/2-1)" => Mul(
            Add(Len(Length::inches(3.2)), Len(Length::pt(6.0))),
            Sub(Div(Num(5.0), Num(2.0)), Num(1.0))
        ));
        v!("(6.3E+2+4* - 3.2pt)/2" => Div(
            Add(Num(6.3e2), Mul(Num(4.0), Neg(Len(Length::pt(3.2))))),
            Num(2.0)
        ));

        // Associativity of multiplication and division.
        v!("3/4*5" => Mul(Div(Num(3.0), Num(4.0)), Num(5.0)));

        // Spanned.
        ts!("[val: 1 + 3]" => s(0,0, 0,12, F!(
            s(0,1, 0,4, "val"); s(0,6, 0,11, Add(
                s(0,6, 0,7, Num(1.0)),
                s(0,10, 0,11, Num(3.0)),
            ))
        )));

        // Span of parenthesized expression contains parens.
        ts!("[val: (1)]" => s(0,0, 0,10, F!(s(0,1, 0,4, "val"); s(0,6, 0,9, Num(1.0)))));

        // Invalid expressions.
        v!("4pt--"        => Len(Length::pt(4.0)));
        e!("[val: 4pt--]" => s(0,10, 0,11, "dangling minus"),
                             s(0,6, 0,10, "missing right summand"));

        v!("3mm+4pt*"        => Add(Len(Length::mm(3.0)), Len(Length::pt(4.0))));
        e!("[val: 3mm+4pt*]" => s(0,10, 0,14, "missing right factor"));
    }

    #[test]
    fn test_parse_tables() {
        // Okay.
        v!("()"                 => Table![]);
        v!("(false)"            => Bool(false));
        v!("(true,)"            => Table![Bool(true)]);
        v!("(key=val)"          => Table!["key" => Id("val")]);
        v!("(1, 2)"             => Table![Num(1.0), Num(2.0)]);
        v!("(1, key=\"value\")" => Table![Num(1.0), "key" => Str("value")]);

        // Decorations.
        d!("[val: key=hi]"    => s(0,6, 0,9, TableKey));
        d!("[val: (key=hi)]"  => s(0,7, 0,10, TableKey));
        d!("[val: f(key=hi)]" => s(0,8, 0,11, TableKey));

        // Spanned with spacing around keyword arguments.
        ts!("[val: \n hi \n = /* //\n */ \"s\n\"]" => s(0,0, 4,2, F!(
            s(0,1, 0,4, "val"); s(1,1, 1,3, "hi") => s(3,4, 4,1, Str("s\n"))
        )));
        e!("[val: \n hi \n = /* //\n */ \"s\n\"]" => );
    }

    #[test]
    fn test_parse_tables_compute_func_calls() {
        v!("empty()"                  => Call!("empty"));
        v!("add ( 1 , 2 )"            => Call!("add"; Num(1.0), Num(2.0)));
        v!("items(\"fire\", #f93a6d)" => Call!("items";
            Str("fire"), Color(RgbaColor::new(0xf9, 0x3a, 0x6d, 0xff))
        ));

        // More complex.
        v!("css(1pt, rgb(90, 102, 254), \"solid\")" => Call!(
            "css";
            Len(Length::pt(1.0)),
            Call!("rgb"; Num(90.0), Num(102.0), Num(254.0)),
            Str("solid"),
        ));

        // Unclosed.
        v!("lang(中文]"       => Call!("lang"; Id("中文")));
        e!("[val: lang(中文]" => s(0,13, 0,13, "expected closing paren"));

        // Invalid name.
        v!("👠(\"abc\", 13e-5)"        => Table!(Str("abc"), Num(13.0e-5)));
        e!("[val: 👠(\"abc\", 13e-5)]" => s(0,6, 0,7, "expected value, found invalid token"));
    }

    #[test]
    fn test_parse_tables_nested() {
        v!("(1, ( ab=(), d = (3, 14pt) )), false" =>
            Table![
                Num(1.0),
                Table!(
                    "ab" => Table![],
                    "d"  => Table!(Num(3.0), Len(Length::pt(14.0))),
                ),
            ],
            Bool(false),
        );
    }

    #[test]
    fn test_parse_tables_errors() {
        // Expected value.
        e!("[val: (=)]"         => s(0,7, 0,8, "expected value, found equals sign"));
        e!("[val: (,)]"         => s(0,7, 0,8, "expected value, found comma"));
        v!("(\x07 abc,)"        => Table![Id("abc")]);
        e!("[val: (\x07 abc,)]" => s(0,7, 0,8, "expected value, found invalid token"));
        e!("[val: (key=,)]"     => s(0,11, 0,12, "expected value, found comma"));
        e!("[val: hi,)]"        => s(0,9, 0,10, "expected value, found closing paren"));

        // Expected comma.
        v!("(true false)"        => Table![Bool(true), Bool(false)]);
        e!("[val: (true false)]" => s(0,11, 0,11, "expected comma"));

        // Expected closing paren.
        e!("[val: (#000]" => s(0,11, 0,11, "expected closing paren"));
        e!("[val: (key]"  => s(0,10, 0,10, "expected closing paren"));
        e!("[val: (key=]" => s(0,11, 0,11, "expected value"),
                             s(0,11, 0,11, "expected closing paren"));

        // Bad key.
        v!("true=you"        => Bool(true), Id("you"));
        e!("[val: true=you]" =>
            s(0,10, 0,10, "expected comma"),
            s(0,10, 0,11, "expected value, found equals sign"));

        // Unexpected equals sign.
        v!("z=y=4"        => Num(4.0), "z" => Id("y"));
        e!("[val: z=y=4]" =>
            s(0,9, 0,9, "expected comma"),
            s(0,9, 0,10, "expected value, found equals sign"));
    }
}
