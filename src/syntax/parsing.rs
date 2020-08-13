//! Parsing of source code into syntax trees.

use std::str::FromStr;

use crate::{Feedback, Pass};
use super::decoration::Decoration;
use super::expr::*;
use super::scope::Scope;
use super::span::{Pos, Span, Spanned};
use super::tokens::{is_newline_char, Token, TokenMode, Tokens};
use super::tree::{SyntaxNode, SyntaxTree};

/// A function which parses a function call into a dynamic node.
pub type CallParser = dyn Fn(FuncCall, &ParseState) -> Pass<SyntaxNode>;

/// An invocation of a function.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncCall<'s> {
    pub header: FuncHeader,
    pub body: FuncBody<'s>,
}

/// The parsed header of a function (everything in the first set of brackets).
#[derive(Debug, Clone, PartialEq)]
pub struct FuncHeader {
    pub name: Spanned<Ident>,
    pub args: FuncArgs,
}

/// The body of a function as a raw spanned string containing what's inside of
/// the brackets.
pub type FuncBody<'s> = Option<Spanned<&'s str>>;

/// The positional and keyword arguments passed to a function.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct FuncArgs {
    pub pos: Tuple,
    pub key: Object,
}

impl FuncArgs {
    /// Create new empty function arguments.
    pub fn new() -> Self {
        Self {
            pos: Tuple::new(),
            key: Object::new(),
        }
    }

    /// Add an argument.
    pub fn push(&mut self, arg: Spanned<FuncArg>) {
        match arg.v {
            FuncArg::Pos(item) => self.pos.push(Spanned::new(item, arg.span)),
            FuncArg::Key(pair) => self.key.push(Spanned::new(pair, arg.span)),
        }
    }
}

/// Either a positional or keyword argument.
#[derive(Debug, Clone, PartialEq)]
pub enum FuncArg {
    Pos(Expr),
    Key(Pair),
}

/// The state which can influence how a string of source code is parsed.
///
/// Parsing is pure - when passed in the same state and source code, the output
/// must be the same.
pub struct ParseState {
    /// The scope containing all function definitions.
    pub scope: Scope,
}

/// Parse a string of source code.
///
/// All spans in the resulting tree and feedback are offset by the given
/// `offset` position. This is used to make spans of a function body relative to
/// the start of the function as a whole as opposed to the start of the
/// function's body.
pub fn parse(src: &str, offset: Pos, state: &ParseState) -> Pass<SyntaxTree> {
    let mut tree = SyntaxTree::new();
    let mut par = SyntaxTree::new();
    let mut feedback = Feedback::new();

    for token in Tokens::new(src, offset, TokenMode::Body) {
        let span = token.span;
        let node = match token.v {
            // Starting from two newlines counts as a paragraph break, a single
            // newline does not.
            Token::Space(newlines) => if newlines < 2 {
                SyntaxNode::Spacing
            } else {
                // End the current paragraph if it is not empty.
                if let (Some(first), Some(last)) = (par.first(), par.last()) {
                    let span = Span::merge(first.span, last.span);
                    let node = SyntaxNode::Par(std::mem::take(&mut par));
                    tree.push(Spanned::new(node, span));
                }
                continue;
            }

            Token::Function { header, body, terminated } => {
                let parsed = FuncParser::new(header, body, state).parse();
                feedback.extend_offset(parsed.feedback, span.start);
                if !terminated {
                    error!(@feedback, Span::at(span.end), "expected closing bracket");
                }
                parsed.output
            }

            Token::Star => SyntaxNode::ToggleBolder,
            Token::Underscore => SyntaxNode::ToggleItalic,
            Token::Backslash => SyntaxNode::Linebreak,
            Token::Raw { raw, terminated } => {
                if !terminated {
                    error!(@feedback, Span::at(span.end), "expected backtick");
                }
                SyntaxNode::Raw(unescape_raw(raw))
            }
            Token::Text(text) => SyntaxNode::Text(text.to_string()),

            Token::LineComment(_) | Token::BlockComment(_) => continue,
            unexpected => {
                error!(@feedback, span, "unexpected {}", unexpected.name());
                continue;
            }
        };

        par.push(Spanned::new(node, span));
    }

    if let (Some(first), Some(last)) = (par.first(), par.last()) {
        let span = Span::merge(first.span, last.span);
        let node = SyntaxNode::Par(par);
        tree.push(Spanned::new(node, span));
    }

    Pass::new(tree, feedback)
}

struct FuncParser<'s> {
    state: &'s ParseState,
    /// The tokens inside the header.
    tokens: Tokens<'s>,
    peeked: Option<Option<Spanned<Token<'s>>>>,
    body: Option<Spanned<&'s str>>,
    feedback: Feedback,
}

impl<'s> FuncParser<'s> {
    fn new(
        header: &'s str,
        body: Option<Spanned<&'s str>>,
        state: &'s ParseState,
    ) -> Self {
        Self {
            state,
            // Start at column 1 because the opening bracket is also part of
            // the function, but not part of the `header` string.
            tokens: Tokens::new(header, Pos::new(0, 1), TokenMode::Header),
            peeked: None,
            body,
            feedback: Feedback::new(),
        }
    }

    fn parse(mut self) -> Pass<SyntaxNode> {
        let (parser, header) = if let Some(header) = self.parse_func_header() {
            let name = header.name.v.as_str();
            let (parser, deco) = match self.state.scope.get_parser(name) {
                // The function exists in the scope.
                Some(parser) => (parser, Decoration::ResolvedFunc),

                // The function does not exist in the scope. The parser that is
                // returned here is a fallback parser which exists to make sure
                // the content of the function is not totally dropped (on a best
                // effort basis).
                None => {
                    error!(@self.feedback, header.name.span, "unknown function");
                    let parser = self.state.scope.get_fallback_parser();
                    (parser, Decoration::UnresolvedFunc)
                }
            };

            self.feedback.decorations.push(Spanned::new(deco, header.name.span));
            (parser, header)
        } else {
            // Parse the body with the fallback parser even when the header is
            // completely unparsable.
            let parser = self.state.scope.get_fallback_parser();
            let header = FuncHeader {
                name: Spanned::new(Ident(String::new()), Span::ZERO),
                args: FuncArgs::new(),
            };
            (parser, header)
        };

        let call = FuncCall { header, body: self.body };
        let parsed = parser(call, self.state);
        self.feedback.extend(parsed.feedback);
        Pass::new(parsed.output, self.feedback)
    }

    fn parse_func_header(&mut self) -> Option<FuncHeader> {
        let after_bracket = self.pos();

        self.skip_white();
        let name = try_opt_or!(self.parse_ident(), {
            self.expected_found_or_at("function name", after_bracket);
            return None;
        });

        self.skip_white();
        let args = match self.eat().map(Spanned::value) {
            Some(Token::Colon) => self.parse_func_args(),
            Some(_) => {
                self.expected_at("colon", name.span.end);
                FuncArgs::new()
            }
            None => FuncArgs::new(),
        };

        Some(FuncHeader { name, args })
    }

    fn parse_func_args(&mut self) -> FuncArgs {
        let mut args = FuncArgs::new();
        loop {
            self.skip_white();
            if self.eof() {
                break;
            }

            let arg = if let Some(ident) = self.parse_ident() {
                self.skip_white();

                // This could be a keyword argument, or a positional argument of
                // type named tuple or identifier.
                if self.check_eat(Token::Equals).is_some() {
                    self.skip_white();

                    let key = ident;
                    self.feedback.decorations
                        .push(Spanned::new(Decoration::ArgumentKey, key.span));

                    let value = try_opt_or!(self.parse_expr(), {
                        self.expected("value");
                        continue;
                    });

                    let span = Span::merge(key.span, value.span);
                    let arg = FuncArg::Key(Pair { key, value });
                    Spanned::new(arg, span)
                } else if self.check(Token::LeftParen) {
                    let tuple = self.parse_named_tuple(ident);
                    tuple.map(|tup| FuncArg::Pos(Expr::NamedTuple(tup)))
                } else {
                    ident.map(|id| FuncArg::Pos(Expr::Ident(id)))
                }
            } else {
                // It's a positional argument.
                try_opt_or!(self.parse_expr(), {
                    self.expected("argument");
                    continue;
                }).map(|expr| FuncArg::Pos(expr))
            };

            let behind_arg = arg.span.end;
            args.push(arg);

            self.skip_white();
            if self.eof() {
                break;
            }

            self.expect_at(Token::Comma, behind_arg);
        }
        args
    }
}

// Parsing expressions and values
impl FuncParser<'_> {
    fn parse_ident(&mut self) -> Option<Spanned<Ident>> {
        self.peek().and_then(|token| match token.v {
            Token::Ident(id) => self.eat_span(Ident(id.to_string())),
            _ => None,
        })
    }

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
        mut parse_op: impl FnMut(Token) -> Option<
            fn(Box<Spanned<Expr>>, Box<Spanned<Expr>>) -> Expr
        >,
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
            if let Some(value) = self.parse_value() {
                let span = Span::merge(hyph.span, value.span);
                Some(Spanned::new(Expr::Neg(Box::new(value)), span))
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
        match token {
            // This could be a named tuple or an identifier.
            Token::Ident(id) => {
                let name = Spanned::new(Ident(id.to_string()), span);
                self.eat();
                self.skip_white();
                Some(if self.check(Token::LeftParen) {
                    self.parse_named_tuple(name).map(|tup| Expr::NamedTuple(tup))
                } else {
                    name.map(|id| Expr::Ident(id))
                })
            }

            Token::Str { string, terminated } => {
                if !terminated {
                    self.expected_at("quote", span.end);
                }
                self.eat_span(Expr::Str(unescape_string(string)))
            }

            Token::Bool(b) => self.eat_span(Expr::Bool(b)),
            Token::Number(n) => self.eat_span(Expr::Number(n)),
            Token::Length(s) => self.eat_span(Expr::Length(s)),
            Token::Hex(s) => {
                if let Ok(color) = RgbaColor::from_str(s) {
                    self.eat_span(Expr::Color(color))
                } else {
                    // Heal color by assuming black.
                    error!(@self.feedback, span, "invalid color");
                    let healed = RgbaColor::new_healed(0, 0, 0, 255);
                    self.eat_span(Expr::Color(healed))
                }
            }

            // This could be a tuple or a parenthesized expression. We parse as
            // a tuple in any case and coerce the tuple into a value if it is
            // coercable (length 1 and no trailing comma).
            Token::LeftParen => {
                let (tuple, coercable) = self.parse_tuple();
                Some(if coercable {
                    tuple.map(|v| {
                        v.0.into_iter().next().expect("tuple is coercable").v
                    })
                } else {
                    tuple.map(|tup| Expr::Tuple(tup))
                })
            }
            Token::LeftBrace => {
                Some(self.parse_object().map(|obj| Expr::Object(obj)))
            }

            _ => None,
        }
    }

    fn parse_named_tuple(&mut self, name: Spanned<Ident>) -> Spanned<NamedTuple> {
        let tuple = self.parse_tuple().0;
        let span = Span::merge(name.span, tuple.span);
        Spanned::new(NamedTuple::new(name, tuple), span)
    }

    /// The boolean tells you whether the tuple can be coerced into a value
    /// (this is the case when it's length 1 and has no trailing comma).
    fn parse_tuple(&mut self) -> (Spanned<Tuple>, bool) {
        let start = self.pos();
        self.assert(Token::LeftParen);

        let mut tuple = Tuple::new();
        let mut commaless = true;
        loop {
            self.skip_white();
            if self.eof() || self.check(Token::RightParen) {
                break;
            }

            let expr = try_opt_or!(self.parse_expr(), {
                self.expected("value");
                continue;
            });

            let behind_expr = expr.span.end;
            tuple.push(expr);

            self.skip_white();
            if self.eof() || self.check(Token::RightParen) {
                break;
            }

            self.expect_at(Token::Comma, behind_expr);
            commaless = false;
        }

        self.expect(Token::RightParen);
        let end = self.pos();
        let coercable = commaless && !tuple.0.is_empty();

        (Spanned::new(tuple, Span::new(start, end)), coercable)
    }

    fn parse_object(&mut self) -> Spanned<Object> {
        let start = self.pos();
        self.assert(Token::LeftBrace);

        let mut object = Object::new();
        loop {
            self.skip_white();
            if self.eof() || self.check(Token::RightBrace) {
                break;
            }

            let key = try_opt_or!(self.parse_ident(), {
                self.expected("key");
                continue;
            });

            let after_key = self.pos();
            self.skip_white();
            if !self.expect_at(Token::Equals, after_key) {
                continue;
            }

            self.feedback.decorations
                .push(Spanned::new(Decoration::ObjectKey, key.span));

            self.skip_white();
            let value = try_opt_or!(self.parse_expr(), {
                self.expected("value");
                continue;
            });

            let behind_value = value.span.end;
            let span = Span::merge(key.span, value.span);
            object.push(Spanned::new(Pair { key, value }, span));

            self.skip_white();
            if self.eof() || self.check(Token::RightBrace) {
                break;
            }

            self.expect_at(Token::Comma, behind_value);
        }

        self.expect(Token::RightBrace);
        let end = self.pos();

        Spanned::new(object, Span::new(start, end))
    }
}

// Error handling
impl FuncParser<'_> {
    fn expect(&mut self, token: Token<'_>) -> bool {
        if self.check(token) {
            self.eat();
            true
        } else {
            self.expected(token.name());
            false
        }
    }

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

// Parsing primitives
impl<'s> FuncParser<'s> {
    fn skip_white(&mut self) {
        loop {
            match self.peek().map(Spanned::value) {
                Some(Token::Space(_))
                | Some(Token::LineComment(_))
                | Some(Token::BlockComment(_)) => { self.eat(); }
                _ => break,
            }
        }
    }

    fn eat(&mut self) -> Option<Spanned<Token<'s>>> {
        self.peeked.take().unwrap_or_else(|| self.tokens.next())
    }

    fn eat_span<T>(&mut self, v: T) -> Option<Spanned<T>> {
        self.eat().map(|spanned| spanned.map(|_| v))
    }

    fn peek(&mut self) -> Option<Spanned<Token<'s>>> {
        let tokens = &mut self.tokens;
        *self.peeked.get_or_insert_with(|| tokens.next())
    }

    fn assert(&mut self, token: Token<'_>) {
        assert!(self.check_eat(token).is_some());
    }

    fn check(&mut self, token: Token<'_>) -> bool {
        self.peek().map(Spanned::value) == Some(token)
    }

    fn check_eat(&mut self, token: Token<'_>) -> Option<Spanned<Token<'s>>> {
        if self.check(token) {
            self.eat()
        } else {
            None
        }
    }

    fn eof(&mut self) -> bool {
        self.peek().is_none()
    }

    fn pos(&self) -> Pos {
        self.peeked
            .flatten()
            .map(|s| s.span.start)
            .unwrap_or_else(|| self.tokens.pos())
    }
}

fn unescape_string(string: &str) -> String {
    let mut iter = string.chars();
    let mut out = String::with_capacity(string.len());

    while let Some(c) = iter.next() {
        if c == '\\' {
            match iter.next() {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(c) => { out.push('\\'); out.push(c); }
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
    let mut iter = raw.chars().peekable();
    let mut line = String::new();
    let mut lines = Vec::new();

    while let Some(c) = iter.next() {
        if c == '\\' {
            match iter.next() {
                Some('`') => line.push('`'),
                Some(c) => { line.push('\\'); line.push(c); }
                None => line.push('\\'),
            }
        } else if is_newline_char(c) {
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
    use crate::length::Length;
    use crate::syntax::span::SpanVec;
    use crate::syntax::test::{check, debug_func, DebugNode};
    use super::*;

    use Decoration::*;
    use Expr::{Bool, Length as Len, Number as Num};
    use SyntaxNode::{
        Spacing as S, Linebreak, ToggleItalic as Italic, ToggleBolder as Bold,
    };

    /// Test whether the given string parses into
    /// - the given SyntaxNode list (required).
    /// - the given error list (optional, if omitted checks against empty list).
    /// - the given decoration list (optional, if omitted it is not tested).
    macro_rules! p {
        ($source:expr => [$($tree:tt)*]) => {
            p!($source => [$($tree)*], []);
        };

        ($source:expr => [$($tree:tt)*], [$($diagnostics:tt)*] $(, [$($decos:tt)*])? $(,)?) => {
            let mut scope = Scope::new(Box::new(debug_func));
            scope.insert("f", Box::new(debug_func));
            scope.insert("n", Box::new(debug_func));
            scope.insert("box", Box::new(debug_func));
            scope.insert("val", Box::new(debug_func));

            let state = ParseState { scope };
            let pass = parse($source, Pos::ZERO, &state);

            // Test tree.
            let (exp, cmp) = span_vec![$($tree)*];
            check($source, exp, pass.output, cmp);

            // Test diagnostics.
            let (exp, cmp) = span_vec![$($diagnostics)*];
            let exp = exp.into_iter()
                .map(|s: Spanned<&str>| s.map(|e| e.to_string()))
                .collect::<Vec<_>>();
            let found = pass.feedback.diagnostics.into_iter()
                .map(|s| s.map(|e| e.message))
                .collect::<Vec<_>>();
            check($source, exp, found, cmp);

            // Test decos.
            $(let (exp, cmp) = span_vec![$($decos)*];
            check($source, exp, pass.feedback.decorations, cmp);)?
        };
    }

    /// Shorthand for `p!("[val: ...]" => par![func!("val", ...)])`.
    macro_rules! pval {
        ($header:expr => $($tts:tt)*) => {
            p!(concat!("[val: ", $header, "]") => [par![func!("val": $($tts)*)]]);
        }
    }

    fn Id(text: &str) -> Expr { Expr::Ident(Ident(text.to_string())) }
    fn Str(text: &str) -> Expr { Expr::Str(text.to_string()) }
    fn Color(r: u8, g: u8, b: u8, a: u8) -> Expr { Expr::Color(RgbaColor::new(r, g, b, a)) }
    fn ColorStr(color: &str) -> Expr { Expr::Color(RgbaColor::from_str(color).expect("invalid test color")) }
    fn Neg(e1: Expr) -> Expr { Expr::Neg(Box::new(Z(e1))) }
    fn Add(e1: Expr, e2: Expr) -> Expr { Expr::Add(Box::new(Z(e1)), Box::new(Z(e2))) }
    fn Sub(e1: Expr, e2: Expr) -> Expr { Expr::Sub(Box::new(Z(e1)), Box::new(Z(e2))) }
    fn Mul(e1: Expr, e2: Expr) -> Expr { Expr::Mul(Box::new(Z(e1)), Box::new(Z(e2))) }
    fn Div(e1: Expr, e2: Expr) -> Expr { Expr::Div(Box::new(Z(e1)), Box::new(Z(e2)))  }
    fn T(text: &str) -> SyntaxNode { SyntaxNode::Text(text.to_string()) }
    fn Z<T>(v: T) -> Spanned<T> { Spanned::zero(v) }

    macro_rules! tuple {
        ($($tts:tt)*) => {
            Expr::Tuple(Tuple(span_vec![$($tts)*].0))
        };
    }

    macro_rules! named_tuple {
        ($name:tt $(, $($tts:tt)*)?) => {
            Expr::NamedTuple(NamedTuple::new(
                span_item!($name).map(|n| Ident(n.to_string())),
                Z(Tuple(span_vec![$($($tts)*)?].0))
            ))
        };
    }

    macro_rules! object {
        ($($key:tt => $value:expr),* $(,)?) => {
            Expr::Object(Object(vec![$(Z(Pair {
                key: span_item!($key).map(|k| Ident(k.to_string())),
                value: Z($value),
            })),*]))
        };
    }

    macro_rules! raw {
        ($($line:expr),* $(,)?) => {
            SyntaxNode::Raw(vec![$($line.to_string()),*])
        };
    }

    macro_rules! par {
        ($($tts:tt)*) => {
            SyntaxNode::Par(span_vec![$($tts)*].0)
        };
    }

    macro_rules! func {
        ($name:tt
            $(: ($($pos:tt)*) $(, { $($key:tt => $value:expr),* })? )?
            $(; $($body:tt)*)?
        ) => {{
            #[allow(unused_mut)]
            let mut args = FuncArgs::new();
            $(
                let items: SpanVec<Expr> = span_vec![$($pos)*].0;
                for item in items {
                    args.push(item.map(|v| FuncArg::Pos(v)));
                }
                $($(args.push(Z(FuncArg::Key(Pair {
                    key: span_item!($key).map(|k| Ident(k.to_string())),
                    value: Z($value),
                })));)*)?
            )?
            SyntaxNode::boxed(DebugNode {
                header: FuncHeader {
                    name: span_item!($name).map(|s| Ident(s.to_string())),
                    args,
                },
                body: func!(@body $($($body)*)?),
            })
        }};
        (@body [$($body:tt)*]) => { Some(span_vec![$($body)*].0) };
        (@body) => { None };
    }

    #[test]
    fn parse_color_strings() {
        assert_eq!(Color(0xf6, 0x12, 0x43, 0xff), ColorStr("f61243ff"));
        assert_eq!(Color(0xb3, 0xd8, 0xb3, 0xff), ColorStr("b3d8b3"));
        assert_eq!(Color(0xfc, 0xd2, 0xa9, 0xad), ColorStr("fCd2a9AD"));
        assert_eq!(Color(0x22, 0x33, 0x33, 0xff), ColorStr("233"));
        assert_eq!(Color(0x11, 0x11, 0x11, 0xbb), ColorStr("111b"));
    }

    #[test]
    fn unescape_strings() {
        fn test(string: &str, expected: &str) {
            assert_eq!(unescape_string(string), expected.to_string());
        }

        test(r#"hello world"#,  "hello world");
        test(r#"hello\nworld"#, "hello\nworld");
        test(r#"a\"bc"#,        "a\"bc");
        test(r#"a\\"#,          "a\\");
        test(r#"a\\\nbc"#,      "a\\\nbc");
        test(r#"a\tbc"#,        "a\tbc");
        test(r"🌎",             "🌎");
        test(r"🌎\",            r"🌎\");
        test(r"\🌎",            r"\🌎");
    }

    #[test]
    fn unescape_raws() {
        fn test(raw: &str, expected: SyntaxNode) {
            let vec = if let SyntaxNode::Raw(v) = expected { v } else { panic!() };
            assert_eq!(unescape_raw(raw), vec);
        }

        test("raw\\`",     raw!["raw`"]);
        test("raw\ntext",  raw!["raw", "text"]);
        test("a\r\nb",     raw!["a", "b"]);
        test("a\n\nb",     raw!["a", "", "b"]);
        test("a\r\x0Bb",   raw!["a", "", "b"]);
        test("a\r\n\r\nb", raw!["a", "", "b"]);
        test("raw\\a",     raw!["raw\\a"]);
        test("raw\\",      raw!["raw\\"]);
    }

    #[test]
    fn parse_basic_nodes() {
        // Basic nodes.
        p!(""                     => []);
        p!("hi"                   => [par![T("hi")]]);
        p!("*hi"                  => [par![Bold, T("hi")]]);
        p!("hi_"                  => [par![T("hi"), Italic]]);
        p!("hi you"               => [par![T("hi"), S, T("you")]]);
        p!("hi// you\nw"          => [par![T("hi"), S, T("w")]]);
        p!("\n\n\nhello"          => [par![T("hello")]]);
        p!("first//\n//\nsecond"  => [par![T("first"), S, S, T("second")]]);
        p!("first//\n \nsecond"   => [par![T("first")], par![T("second")]]);
        p!("first/*\n \n*/second" => [par![T("first"), T("second")]]);
        p!(r"a\ b"                => [par![T("a"), Linebreak, S, T("b")]]);
        p!("💜\n\n 🌍"           => [par![T("💜")], par![T("🌍")]]);

        // Raw markup.
        p!("`py`"         => [par![raw!["py"]]]);
        p!("[val][`hi]`]" => [par![func!("val"; [par![raw!["hi]"]]])]]);
        p!("`hi\nyou"     => [par![raw!["hi", "you"]]], [(1:3, 1:3, "expected backtick")]);
        p!("`hi\\`du`"    => [par![raw!["hi`du"]]]);

        // Spanned SyntaxNodes.
        p!("Hi"      => [(0:0, 0:2, par![(0:0, 0:2, T("Hi"))])]);
        p!("*Hi*"    => [(0:0, 0:4, par![(0:0, 0:1, Bold), (0:1, 0:3, T("Hi")), (0:3, 0:4, Bold)])]);
        p!("🌎\n*/[n]" =>
            [(0:0, 1:5, par![(0:0, 0:1, T("🌎")), (0:1, 1:0, S), (1:2, 1:5, func!((0:1, 0:2, "n")))])],
            [(1:0, 1:2, "unexpected end of block comment")],
            [(1:3, 1:4, ResolvedFunc)],
        );
    }

    #[test]
    fn parse_function_names() {
        // No closing bracket.
        p!("[" => [par![func!("")]], [
            (0:1, 0:1, "expected function name"),
            (0:1, 0:1, "expected closing bracket")
        ]);

        // No name.
        p!("[]" => [par![func!("")]], [(0:1, 0:1, "expected function name")]);
        p!("[\"]" => [par![func!("")]], [
            (0:1, 0:3, "expected function name, found string"),
            (0:3, 0:3, "expected closing bracket"),
        ]);

        // An unknown name.
        p!("[hi]" =>
            [par![func!("hi")]],
            [(0:1, 0:3, "unknown function")],
            [(0:1, 0:3, UnresolvedFunc)],
        );

        // A valid name.
        p!("[f]"   => [par![func!("f")]], [], [(0:1, 0:2, ResolvedFunc)]);
        p!("[  f]" => [par![func!("f")]], [], [(0:3, 0:4, ResolvedFunc)]);

        // An invalid token for a name.
        p!("[12]"   => [par![func!("")]], [(0:1, 0:3, "expected function name, found number")], []);
        p!("[🌎]"   => [par![func!("")]], [(0:1, 0:2, "expected function name, found invalid token")], []);
        p!("[  🌎]" => [par![func!("")]], [(0:3, 0:4, "expected function name, found invalid token")], []);
    }

    #[test]
    fn parse_colon_starting_function_arguments() {
        // Valid.
        p!("[val: true]" =>
            [par![func!["val": (Bool(true))]]], [],
            [(0:1, 0:4, ResolvedFunc)],
        );

        // No colon before arg.
        p!("[val\"s\"]" => [par![func!("val")]], [(0:4, 0:4, "expected colon")]);

        // No colon before valid, but wrong token.
        p!("[val=]" => [par![func!("val")]], [(0:4, 0:4, "expected colon")]);

        // No colon before invalid tokens, which are ignored.
        p!("[val/🌎:$]" =>
            [par![func!("val")]],
            [(0:4, 0:4, "expected colon")],
            [(0:1, 0:4, ResolvedFunc)],
        );

        // String in invalid header without colon still parsed as string
        // Note: No "expected quote" error because not even the string was
        //       expected.
        p!("[val/\"]" => [par![func!("val")]], [
            (0:4, 0:4, "expected colon"),
            (0:7, 0:7, "expected closing bracket"),
        ]);

        // Just colon without args.
        p!("[val:]"         => [par![func!("val")]]);
        p!("[val:/*12pt*/]" => [par![func!("val")]]);

        // Whitespace / comments around colon.
        p!("[val\n:\ntrue]"      => [par![func!("val": (Bool(true)))]]);
        p!("[val/*:*/://\ntrue]" => [par![func!("val": (Bool(true)))]]);
    }

    #[test]
    fn parse_one_positional_argument() {
        // Different expressions.
        pval!("_"      => (Id("_")));
        pval!("name"   => (Id("name")));
        pval!("\"hi\"" => (Str("hi")));
        pval!("3.14"   => (Num(3.14)));
        pval!("4.5cm"  => (Len(Length::cm(4.5))));
        pval!("12e1pt" => (Len(Length::pt(12e1))));
        pval!("#f7a20500" => (ColorStr("f7a20500")));
        pval!("\"a\n[]\\\"string\"" => (Str("a\n[]\"string")));

        // Trailing comma.
        pval!("a," => (Id("a")));

        // Simple coerced tuple.
        pval!("(hi)" => (Id("hi")));

        // Math.
        pval!("3.2in + 6pt" => (Add(Len(Length::inches(3.2)), Len(Length::pt(6.0)))));
        pval!("5 - 0.01"    => (Sub(Num(5.0), Num(0.01))));
        pval!("(3mm * 2)"   => (Mul(Len(Length::mm(3.0)), Num(2.0))));
        pval!("12e-3cm/1pt" => (Div(Len(Length::cm(12e-3)), Len(Length::pt(1.0)))));

        // Span of expression.
        p!("[val: 1 + 3]" => [(0:0, 0:12, par![(0:0, 0:12, func!(
            (0:1, 0:4, "val"): ((0:6, 0:11, Expr::Add(
                Box::new(span_item!((0:6, 0:7, Num(1.0)))),
                Box::new(span_item!((0:10, 0:11, Num(3.0)))),
            )))
        ))])]);

        // Unclosed string.
        p!("[val: \"hello]" => [par![func!("val": (Str("hello]")), {})]], [
            (0:13, 0:13, "expected quote"),
            (0:13, 0:13, "expected closing bracket"),
        ]);

        // Invalid, healed colors.
        let healed = Expr::Color(RgbaColor::new_healed(0, 0, 0, 255));
        p!("[val: #12345]"     => [par![func!("val": (healed.clone()))]], [(0:6, 0:12, "invalid color")]);
        p!("[val: #a5]"        => [par![func!("val": (healed.clone()))]], [(0:6, 0:9,  "invalid color")]);
        p!("[val: #14b2ah]"    => [par![func!("val": (healed.clone()))]], [(0:6, 0:13, "invalid color")]);
        p!("[val: #f075ff011]" => [par![func!("val": (healed.clone()))]], [(0:6, 0:16, "invalid color")]);
    }

    #[test]
    fn parse_complex_mathematical_expressions() {
        // Valid expressions.
        pval!("(3.2in + 6pt)*(5/2-1)" => (Mul(
            Add(Len(Length::inches(3.2)), Len(Length::pt(6.0))),
            Sub(Div(Num(5.0), Num(2.0)), Num(1.0))
        )));
        pval!("(6.3E+2+4* - 3.2pt)/2" => (Div(
            Add(Num(6.3e2), Mul(Num(4.0), Neg(Len(Length::pt(3.2))))),
            Num(2.0)
        )));

        // Associativity of multiplication and division.
        pval!("3/4*5" => (Mul(Div(Num(3.0), Num(4.0)), Num(5.0))));

        // Span of parenthesized expression contains parens.
        p!("[val: (1)]" => [(0:0, 0:10, par![
            (0:0, 0:10, func!((0:1, 0:4, "val"): ((0:6, 0:9, Num(1.0)))))
        ])]);

        // Invalid expressions.
        p!("[val: 4pt--]" => [par![func!("val": (Len(Length::pt(4.0))))]], [
            (0:10, 0:11, "dangling minus"),
            (0:6, 0:10, "missing right summand")
        ]);
        p!("[val: 3mm+4pt*]" =>
            [par![func!("val": (Add(Len(Length::mm(3.0)), Len(Length::pt(4.0)))))]],
            [(0:10, 0:14, "missing right factor")],
        );
    }

    #[test]
    fn parse_tuples() {
        // Empty tuple.
        pval!("()" => (tuple!()));
        pval!("empty()" => (named_tuple!("empty")));

        // Space between name and tuple.
        pval!("add ( 1 , 2 )" => (named_tuple!("add", Num(1.0), Num(2.0))));
        pval!("num = add ( 1 , 2 )" => (), {
            "num" => named_tuple!("add", Num(1.0), Num(2.0))
        });

        // Invalid value.
        p!("[val: sound(\x07)]" =>
            [par![func!("val": (named_tuple!("sound")), {})]],
            [(0:12, 0:13, "expected value, found invalid token")],
        );

        // Invalid tuple name.
        p!("[val: 👠(\"abc\", 13e-5)]" =>
            [par![func!("val": (tuple!(Str("abc"), Num(13.0e-5))), {})]],
            [(0:6, 0:7, "expected argument, found invalid token")],
        );

        // Unclosed tuple.
        p!("[val: lang(中文]" =>
            [par![func!("val": (named_tuple!("lang", Id("中文"))), {})]],
            [(0:13, 0:13, "expected closing paren")],
        );

        // Valid values.
        pval!("(1, 2)" => (tuple!(Num(1.0), Num(2.0))));
        pval!("(\"s\",)" => (tuple!(Str("s"))));
        pval!("items(\"fire\", #f93a6d)" => (
            named_tuple!("items", Str("fire"), ColorStr("f93a6d")
        )));

        // Nested tuples.
        pval!("css(1pt, rgb(90, 102, 254), \"solid\")" => (named_tuple!(
            "css",
            Len(Length::pt(1.0)),
            named_tuple!("rgb", Num(90.0), Num(102.0), Num(254.0)),
            Str("solid"),
        )));

        // Invalid commas.
        p!("[val: (,)]" =>
            [par![func!("val": (tuple!()), {})]],
            [(0:7, 0:8, "expected value, found comma")],
        );
        p!("[val: (true false)]" =>
            [par![func!("val": (tuple!(Bool(true), Bool(false))), {})]],
            [(0:11, 0:11, "expected comma")],
        );
    }

    #[test]
    fn parse_objects() {
        let val = || par![func!("val": (object! {}), {})];

        // Okay objects.
        pval!("{}" => (object! {}));
        pval!("{ key = value }" => (object! { "key" => Id("value") }));

        // Unclosed object.
        p!("[val: {hello = world]" =>
            [par![func!("val": (object! { "hello" => Id("world") }), {})]],
            [(0:20, 0:20, "expected closing brace")],
        );
        p!("[val: { a]" =>
            [par![func!("val": (object! {}), {})]],
            [(0:9, 0:9, "expected equals sign"), (0:9, 0:9, "expected closing brace")],
        );

        // Missing key.
        p!("[val: {,}]" => [val()], [(0:7, 0:8, "expected key, found comma")]);
        p!("[val: { 12pt }]" => [val()], [(0:8, 0:12, "expected key, found length")]);
        p!("[val: { : }]" => [val()], [(0:8, 0:9, "expected key, found colon")]);

        // Missing colon.
        p!("[val: { key }]" => [val()], [(0:11, 0:11, "expected equals sign")]);
        p!("[val: { key false }]" => [val()], [
            (0:11, 0:11, "expected equals sign"),
            (0:12, 0:17, "expected key, found bool"),
        ]);
        p!("[val: { a b=c }]" =>
            [par![func!("val": (object! { "b" => Id("c") }), {})]],
            [(0:9, 0:9, "expected equals sign")],
        );

        // Missing value.
        p!("[val: { key= : }]" => [val()], [(0:13, 0:14, "expected value, found colon")]);
        p!("[val: { key= , k= \"s\" }]" =>
            [par![func!("val": (object! { "k" => Str("s") }), {})]],
            [(0:13, 0:14, "expected value, found comma")],
        );

        // Missing comma, invalid token.
        p!("[val: left={ a=2, b=false 🌎 }]" =>
            [par![func!("val": (), {
                "left" => object! {
                    "a" => Num(2.0),
                    "b" => Bool(false),
                }
            })]],
            [(0:25, 0:25, "expected comma"),
             (0:26, 0:27, "expected key, found invalid token")],
        );
    }

    #[test]
    fn parse_nested_tuples_and_objects() {
        pval!("(1, { ab=(), d = (3, 14pt) }), false" => (
            tuple!(
                Num(1.0),
                object!(
                    "ab" => tuple!(),
                    "d" => tuple!(Num(3.0), Len(Length::pt(14.0))),
                ),
            ),
            Bool(false),
        ));
    }

    #[test]
    fn parse_one_keyword_argument() {
        // Correct
        p!("[val: x=true]" =>
            [par![func!("val": (), { "x" => Bool(true) })]], [],
            [(0:6, 0:7, ArgumentKey), (0:1, 0:4, ResolvedFunc)],
        );

        // Spacing around keyword arguments
        p!("\n [val: \n hi \n = /* //\n */ \"s\n\"]" =>
            [par![S, func!("val": (), { "hi" => Str("s\n") })]], [],
            [(2:1, 2:3, ArgumentKey), (1:2, 1:5, ResolvedFunc)],
        );

        // Missing value
        p!("[val: x=]" =>
            [par![func!("val")]],
            [(0:8, 0:8, "expected value")],
            [(0:6, 0:7, ArgumentKey), (0:1, 0:4, ResolvedFunc)],
        );
    }

    #[test]
    fn parse_multiple_mixed_arguments() {
        p!("[val: 12pt, key=value]" =>
            [par![func!("val": (Len(Length::pt(12.0))), { "key" => Id("value") })]],
            [],
            [(0:12, 0:15, ArgumentKey), (0:1, 0:4, ResolvedFunc)],
        );
        pval!("a , x=\"b\" , c" => (Id("a"), Id("c")), { "x" => Str("b") });
    }

    #[test]
    fn parse_invalid_values() {
        p!("[val: )]"     => [par![func!("val")]], [(0:6, 0:7, "expected argument, found closing paren")]);
        p!("[val: }]"     => [par![func!("val")]], [(0:6, 0:7, "expected argument, found closing brace")]);
        p!("[val: :]"     => [par![func!("val")]], [(0:6, 0:7, "expected argument, found colon")]);
        p!("[val: ,]"     => [par![func!("val")]], [(0:6, 0:7, "expected argument, found comma")]);
        p!("[val: =]"     => [par![func!("val")]], [(0:6, 0:7, "expected argument, found equals sign")]);
        p!("[val: 🌎]"    => [par![func!("val")]], [(0:6, 0:7, "expected argument, found invalid token")]);
        p!("[val: 12ept]" => [par![func!("val")]], [(0:6, 0:11, "expected argument, found invalid token")]);
        p!("[val: [hi]]"  =>
            [par![func!("val")]],
            [(0:6, 0:10, "expected argument, found function")],
            [(0:1, 0:4, ResolvedFunc)],
        );
    }

    #[test]
    fn parse_invalid_key_value_pairs() {
        // Invalid keys.
        p!("[val: true=you]" =>
            [par![func!("val": (Bool(true), Id("you")), {})]],
            [(0:10, 0:10, "expected comma"),
             (0:10, 0:11, "expected argument, found equals sign")],
            [(0:1, 0:4, ResolvedFunc)],
        );

        // Unexpected equals.
        p!("[box: z=y=4]" =>
            [par![func!("box": (Num(4.0)), { "z" => Id("y") })]],
            [(0:9, 0:9, "expected comma"),
             (0:9, 0:10, "expected argument, found equals sign")],
        );

        // Invalid colon after keyable positional argument.
        p!("[val: key:12]" =>
            [par![func!("val": (Id("key"), Num(12.0)), {})]],
            [(0:9, 0:9, "expected comma"),
             (0:9, 0:10, "expected argument, found colon")],
            [(0:1, 0:4, ResolvedFunc)],
        );

        // Invalid colon after unkeyable positional argument.
        p!("[val: true:12]" =>
            [par![func!("val": (Bool(true), Num(12.0)), {})]],
            [(0:10, 0:10, "expected comma"),
             (0:10, 0:11, "expected argument, found colon")],
            [(0:1, 0:4, ResolvedFunc)],
        );
    }

    #[test]
    fn parse_invalid_commas() {
        // Missing commas.
        p!("[val: 1pt 1]" =>
            [par![func!("val": (Len(Length::pt(1.0)), Num(1.0)), {})]],
            [(0:9, 0:9, "expected comma")],
        );
        p!(r#"[val: _"s"]"# =>
            [par![func!("val": (Id("_"), Str("s")), {})]],
            [(0:7, 0:7, "expected comma")],
        );

        // Unexpected commas.
        p!("[val:,]" => [par![func!("val")]], [(0:5, 0:6, "expected argument, found comma")]);
        p!("[val: key=,]" => [par![func!("val")]], [(0:10, 0:11, "expected value, found comma")]);
        p!("[val:, true]" =>
            [par![func!("val": (Bool(true)), {})]],
            [(0:5, 0:6, "expected argument, found comma")],
        );
    }

    #[test]
    fn parse_bodies() {
        p!("[val][Hi]" => [par![func!("val"; [par![T("Hi")]])]]);
        p!("[val:*][*Hi*]" =>
            [par![func!("val"; [par![Bold, T("Hi"), Bold]])]],
            [(0:5, 0:6, "expected argument, found star")],
        );
        // Errors in bodies.
        p!(" [val][ */ ]" =>
            [par![S, func!("val"; [par![S, S]])]],
            [(0:8, 0:10, "unexpected end of block comment")],
        );
    }

    #[test]
    fn parse_spanned_functions() {
        // Space before function
        p!(" [val]" =>
            [(0:0, 0:6, par![(0:0, 0:1, S), (0:1, 0:6, func!((0:1, 0:4, "val")))])],
            [],
            [(0:2, 0:5, ResolvedFunc)],
        );

        // Newline before function
        p!("a \n\r\n[val]" =>
            [
                (0:0, 0:1, par![(0:0, 0:1, T("a"))]),
                (2:0, 2:5, par![(2:0, 2:5, func!((0:1, 0:4, "val")))]),
            ],
            [],
            [(2:1, 2:4, ResolvedFunc)],
        );

        // Content before function
        p!("hello [val][world] 🌎" =>
            [(0:0, 0:20, par![
                (0:0, 0:5, T("hello")),
                (0:5, 0:6, S),
                (0:6, 0:18, func!((0:1, 0:4, "val"); [
                    (0:6, 0:11, par![(0:6, 0:11, T("world"))])
                ])),
                (0:18, 0:19, S),
                (0:19, 0:20, T("🌎"))
            ])],
            [],
            [(0:7, 0:10, ResolvedFunc)],
        );

        // Nested function
        p!(" [val][\nbody[ box]\n ]" =>
            [(0:0, 2:2, par![
                (0:0, 0:1, S),
                (0:1, 2:2, func!((0:1, 0:4, "val"); [(0:6, 2:1, par![
                    (0:6, 1:0, S),
                    (1:0, 1:4, T("body")),
                    (1:4, 1:10, func!((0:2, 0:5, "box"))),
                    (1:10, 2:1, S),
                ])]))
            ])],
            [],
            [(0:2, 0:5, ResolvedFunc), (1:6, 1:9, ResolvedFunc)],
        );
    }
}
