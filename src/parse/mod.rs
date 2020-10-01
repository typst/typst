//! Parsing and tokenization.

mod lines;
mod resolve;
mod scanner;
mod tokens;

pub use lines::*;
pub use resolve::*;
pub use scanner::*;
pub use tokens::*;

use std::str::FromStr;

use super::*;
use crate::color::RgbaColor;
use crate::compute::dict::SpannedEntry;
use crate::syntax::*;
use crate::{Feedback, Pass};

/// Parse a string of source code.
pub fn parse(src: &str) -> Pass<SyntaxTree> {
    Parser::new(src).parse()
}

struct Parser<'s> {
    tokens: Tokens<'s>,
    peeked: Option<Option<Spanned<Token<'s>>>>,
    delimiters: Vec<(Pos, Token<'static>)>,
    at_block_or_line_start: bool,
    feedback: Feedback,
}

impl<'s> Parser<'s> {
    fn new(src: &'s str) -> Self {
        Self {
            tokens: Tokens::new(src, TokenMode::Body),
            peeked: None,
            delimiters: vec![],
            at_block_or_line_start: true,
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

        self.at_block_or_line_start = true;
        while !self.eof() {
            if let Some(node) = self.parse_node() {
                tree.push(node);
            }
        }

        tree
    }

    fn parse_node(&mut self) -> Option<Spanned<SyntaxNode>> {
        let token = self.peek()?;
        let end = Span::at(token.span.end);

        // Set block or line start to false because most nodes have that effect, but
        // remember the old value to actually check it for hashtags and because comments
        // and spaces want to retain it.
        let was_at_block_or_line_start = self.at_block_or_line_start;
        self.at_block_or_line_start = false;

        Some(match token.v {
            // Starting from two newlines counts as a paragraph break, a single
            // newline does not.
            Token::Space(n) => {
                if n == 0 {
                    self.at_block_or_line_start = was_at_block_or_line_start;
                } else if n >= 1 {
                    self.at_block_or_line_start = true;
                }

                self.with_span(if n >= 2 {
                    SyntaxNode::Parbreak
                } else {
                    SyntaxNode::Spacing
                })
            }

            Token::LineComment(_) | Token::BlockComment(_) => {
                self.at_block_or_line_start = was_at_block_or_line_start;
                self.eat();
                return None;
            }

            Token::LeftBracket => {
                let call = self.parse_bracket_call(false);
                self.at_block_or_line_start = false;
                call.map(SyntaxNode::Call)
            }

            Token::Star => self.with_span(SyntaxNode::ToggleBolder),
            Token::Underscore => self.with_span(SyntaxNode::ToggleItalic),
            Token::Backslash => self.with_span(SyntaxNode::Linebreak),

            Token::Hashtag if was_at_block_or_line_start => {
                self.parse_heading().map(SyntaxNode::Heading)
            }

            Token::Raw { raw, backticks, terminated } => {
                if !terminated {
                    error!(@self.feedback, end, "expected backtick(s)");
                }

                let raw = resolve::resolve_raw(raw, backticks);
                self.with_span(SyntaxNode::Raw(raw))
            }

            Token::Text(text) => self.with_span(SyntaxNode::Text(text.to_string())),
            Token::Hashtag => self.with_span(SyntaxNode::Text("#".to_string())),

            Token::UnicodeEscape { sequence, terminated } => {
                if !terminated {
                    error!(@self.feedback, end, "expected closing brace");
                }

                if let Some(c) = resolve::resolve_hex(sequence) {
                    self.with_span(SyntaxNode::Text(c.to_string()))
                } else {
                    error!(@self.feedback, token.span, "invalid unicode escape sequence");
                    // TODO: Decide whether to render the escape sequence.
                    self.eat();
                    return None;
                }
            }

            unexpected => {
                error!(@self.feedback, token.span, "unexpected {}", unexpected.name());
                self.eat();
                return None;
            }
        })
    }

    fn parse_heading(&mut self) -> Spanned<Heading> {
        let start = self.pos();
        self.assert(Token::Hashtag);

        let mut level = 0;
        while self.peekv() == Some(Token::Hashtag) {
            level += 1;
            self.eat();
        }

        let span = Span::new(start, self.pos());
        let level = level.span_with(span);

        if level.v > 5 {
            warning!(
                @self.feedback, level.span,
                "section depth larger than 6 has no effect",
            );
        }

        self.skip_ws();

        let mut tree = SyntaxTree::new();
        while !self.eof() && !matches!(self.peekv(), Some(Token::Space(n)) if n >= 1) {
            if let Some(node) = self.parse_node() {
                tree.push(node);
            }
        }

        let span = Span::new(start, self.pos());
        Heading { level, tree }.span_with(span)
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
        self.skip_ws();
        let name = self.parse_ident().unwrap_or_else(|| {
            self.expected_found_or_at("function name", before_name);
            Ident(String::new()).span_with(Span::at(before_name))
        });

        self.skip_ws();

        let mut args = match self.eatv() {
            Some(Token::Colon) => self.parse_dict_contents().0,
            Some(_) => {
                self.expected_at("colon", name.span.end);
                while self.eat().is_some() {}
                DictExpr::new()
            }
            None => DictExpr::new(),
        };

        self.end_group();
        self.skip_ws();
        let (has_chained_child, end) = if self.peek().is_some() {
            let item = self.parse_bracket_call(true);
            let span = item.span;
            let t = vec![item.map(SyntaxNode::Call)];
            args.push(SpannedEntry::val(Expr::Tree(t).span_with(span)));
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
            args.push(SpannedEntry::val(expr.span_with(body_span)));
            span.expand(body_span);
        }

        CallExpr { name, args }.span_with(span)
    }

    fn parse_paren_call(&mut self, name: Spanned<Ident>) -> Spanned<CallExpr> {
        self.start_group(Group::Paren);
        let args = self.parse_dict_contents().0;
        let args_span = self.end_group();
        let span = Span::merge(name.span, args_span);
        CallExpr { name, args }.span_with(span)
    }
}

// Dicts.
impl Parser<'_> {
    fn parse_dict_contents(&mut self) -> (DictExpr, bool) {
        let mut dict = DictExpr::new();
        let mut comma_and_keyless = true;

        while {
            self.skip_ws();
            !self.eof()
        } {
            let (key, val) = if let Some(ident) = self.parse_ident() {
                self.skip_ws();

                match self.peekv() {
                    Some(Token::Equals) => {
                        self.eat();
                        self.skip_ws();
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
                dict.insert(key.v.0, SpannedEntry::new(key.span, val));
                self.feedback
                    .decorations
                    .push(Decoration::DictKey.span_with(key.span));
            } else {
                dict.push(SpannedEntry::val(val));
            }

            if {
                self.skip_ws();
                self.eof()
            } {
                break;
            }

            self.expect_at(Token::Comma, behind);
            comma_and_keyless = false;
        }

        let coercable = comma_and_keyless && !dict.is_empty();
        (dict, coercable)
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

        self.skip_ws();
        while let Some(token) = self.peek() {
            if let Some(op) = parse_op(token.v) {
                self.eat();
                self.skip_ws();

                if let Some(right) = parse_operand(self) {
                    let span = Span::merge(left.span, right.span);
                    let v = op(Box::new(left), Box::new(right));
                    left = v.span_with(span);
                    self.skip_ws();
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
            self.skip_ws();
            if let Some(factor) = self.parse_factor() {
                let span = Span::merge(hyph.span, factor.span);
                Some(Expr::Neg(Box::new(factor)).span_with(span))
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
                let name = Ident(id.to_string()).span_with(span);
                self.eat();
                self.skip_ws();
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
                self.with_span(Expr::Str(resolve::resolve_string(string)))
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

            // This could be a dictionary or a parenthesized expression. We
            // parse as a dictionary in any case and coerce  into a value if
            // that's coercable (length 1 and no trailing comma).
            Token::LeftParen => {
                self.start_group(Group::Paren);
                let (dict, coercable) = self.parse_dict_contents();
                let span = self.end_group();

                let expr = if coercable {
                    dict.into_values().next().expect("dict is coercable").val.v
                } else {
                    Expr::Dict(dict)
                };

                expr.span_with(span)
            }

            // This is a content expression.
            Token::LeftBrace => {
                self.start_group(Group::Brace);
                self.tokens.push_mode(TokenMode::Body);

                let tree = self.parse_body_contents();

                self.tokens.pop_mode();
                let span = self.end_group();
                Expr::Tree(tree).span_with(span)
            }

            // This is a bracketed function call.
            Token::LeftBracket => {
                let call = self.parse_bracket_call(false);
                let tree = vec![call.map(SyntaxNode::Call)];
                Expr::Tree(tree).span_with(span)
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

    fn skip_ws(&mut self) {
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
        v.span_with(span)
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

#[cfg(test)]
mod tests;
