//! Parsing of source code into syntax models.

use std::iter::FromIterator;
use std::str::FromStr;

use crate::{Pass, Feedback};
use super::func::{FuncHeader, FuncArgs, FuncArg};
use super::expr::*;
use super::scope::Scope;
use super::span::{Position, Span, Spanned};
use super::tokens::{Token, Tokens, TokenizationMode};
use super::*;


/// The context for parsing.
#[derive(Debug, Copy, Clone)]
pub struct ParseContext<'a> {
    /// The scope containing function definitions.
    pub scope: &'a Scope,
}

/// Parse source code into a syntax model.
///
/// All errors and decorations are offset by the `start` position.
pub fn parse(start: Position, src: &str, ctx: ParseContext) -> Pass<SyntaxModel> {
    let mut model = SyntaxModel::new();
    let mut feedback = Feedback::new();

    // We always start in body mode. The header tokenization mode is only used
    // in the `FuncParser`.
    let mut tokens = Tokens::new(start, src, TokenizationMode::Body);

    while let Some(token) = tokens.next() {
        let span = token.span;

        let node = match token.v {
            Token::LineComment(_) | Token::BlockComment(_) => continue,

            // Only at least two newlines mean a _real_ newline indicating a
            // paragraph break.
            Token::Space(newlines) => if newlines >= 2 {
                Node::Parbreak
            } else {
                Node::Space
            },

            Token::Function { header, body, terminated } => {
                let parsed = FuncParser::new(header, body, ctx).parse();
                feedback.extend_offset(span.start, parsed.feedback);

                if !terminated {
                    feedback.errors.push(err!(Span::at(span.end);
                        "expected closing bracket"));
                }

                parsed.output
            }

            Token::Star       => Node::ToggleBolder,
            Token::Underscore => Node::ToggleItalic,
            Token::Backslash  => Node::Linebreak,

            Token::Raw { raw, terminated } => {
                if !terminated {
                    feedback.errors.push(err!(Span::at(span.end);
                        "expected backtick"));
                }

                Node::Raw(unescape_raw(raw))
            }

            Token::Text(text) => Node::Text(text.to_string()),

            other => {
                feedback.errors.push(err!(span; "unexpected {}", other.name()));
                continue;
            }
        };

        model.add(Spanned { v: node, span: token.span });
    }

    Pass::new(model, feedback)
}

/// Performs the function parsing.
struct FuncParser<'s> {
    ctx: ParseContext<'s>,
    feedback: Feedback,

    /// ```typst
    /// [tokens][body]
    ///  ^^^^^^
    /// ```
    tokens: Tokens<'s>,
    peeked: Option<Option<Spanned<Token<'s>>>>,

    /// The spanned body string if there is a body.
    /// ```typst
    /// [tokens][body]
    ///          ^^^^
    /// ```
    body: Option<Spanned<&'s str>>,
}

impl<'s> FuncParser<'s> {
    /// Create a new function parser.
    fn new(
        header: &'s str,
        body: Option<Spanned<&'s str>>,
        ctx: ParseContext<'s>
    ) -> FuncParser<'s> {
        FuncParser {
            ctx,
            feedback: Feedback::new(),
            tokens: Tokens::new(Position::new(0, 1), header, TokenizationMode::Header),
            peeked: None,
            body,
        }
    }

    /// Do the parsing.
    fn parse(mut self) -> Pass<Node> {
        let parsed = if let Some(header) = self.parse_func_header() {
            let name = header.name.v.as_str();
            let (parser, deco) = match self.ctx.scope.get_parser(name) {
                // A valid function.
                Ok(parser) => (parser, Decoration::ValidFuncName),

                // The fallback parser was returned. Invalid function.
                Err(parser) => {
                    self.feedback.errors.push(err!(header.name.span; "unknown function"));
                    (parser, Decoration::InvalidFuncName)
                }
            };

            self.feedback.decos.push(Spanned::new(deco, header.name.span));

            parser(header, self.body, self.ctx)
        } else {
            let default = FuncHeader {
                name: Spanned::new(Ident("".to_string()), Span::ZERO),
                args: FuncArgs::new(),
            };

            // Use the fallback function such that the body is still rendered
            // even if the header is completely unparsable.
            self.ctx.scope.get_fallback_parser()(default, self.body, self.ctx)
        };

        self.feedback.extend(parsed.feedback);

        Pass::new(Node::Model(parsed.output), self.feedback)
    }

    /// Parse the header tokens.
    fn parse_func_header(&mut self) -> Option<FuncHeader> {
        let start = self.pos();
        self.skip_whitespace();

        let name = match self.parse_ident() {
            Some(ident) => ident,
            None => {
                let other = self.eat();
                self.expected_found_or_at("identifier", other, start);
                return None;
            }
        };

        self.skip_whitespace();
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

    /// Parse the argument list between colons and end of the header.
    fn parse_func_args(&mut self) -> FuncArgs {
        // Parse a collection until the token is `None`, that is, the end of the
        // header.
        self.parse_collection(None, |p| {
            // If we have an identifier we might have a keyword argument,
            // otherwise its for sure a postional argument.
            if let Some(ident) = p.parse_ident() {
                // This could still be a named tuple
                if let Some(Token::LeftParen) = p.peekv() {
                    return Ok(FuncArg::Pos(
                        p.parse_named_tuple(ident)
                            .map(|t| Expr::NamedTuple(t))
                    ));
                }

                p.skip_whitespace();

                if let Some(Token::Equals) = p.peekv() {
                    p.eat();
                    p.skip_whitespace();

                    // Semantic highlighting for argument keys.
                    p.feedback.decos.push(
                        Spanned::new(Decoration::ArgumentKey, ident.span));

                    let value = p.parse_expr().ok_or(("value", None))?;

                    // Add a keyword argument.
                    Ok(FuncArg::Key(Pair { key: ident, value }))
                } else {
                    // Add a positional argument because there was no equals
                    // sign after the identifier that could have been a key.
                    Ok(FuncArg::Pos(ident.map(|id| Expr::Ident(id))))
                }
            } else {
                // Add a positional argument because we haven't got an
                // identifier that could be an argument key.
                p.parse_expr().map(|expr| FuncArg::Pos(expr))
                    .ok_or(("argument", None))
            }
        }).v
    }

    /// Parse an atomic or compound (tuple / object) expression.
    fn parse_expr(&mut self) -> Option<Spanned<Expr>> {
        let first = self.peek()?;
        macro_rules! take {
            ($v:expr) => ({ self.eat(); Spanned { v: $v, span: first.span } });
        }

        Some(match first.v {
            Token::ExprIdent(i) => {
                let name = take!(Ident(i.to_string()));

                // This could be a named tuple or an identifier
                if let Some(Token::LeftParen) = self.peekv() {
                    self.parse_named_tuple(name).map(|t| Expr::NamedTuple(t))
                } else {
                    name.map(|i| Expr::Ident(i))
                }
            },
            Token::ExprStr { string, terminated } => {
                if !terminated {
                    self.expected_at("quote", first.span.end);
                }

                take!(Expr::Str(unescape_string(string)))
            }

            Token::ExprNumber(n) => take!(Expr::Number(n)),
            Token::ExprSize(s) => take!(Expr::Size(s)),
            Token::ExprBool(b) => take!(Expr::Bool(b)),
            Token::ExprHex(s) => {
                if let Ok(color) = RgbaColor::from_str(s) {
                    take!(Expr::Color(color))
                } else {
                    // Heal color by assuming black
                    self.feedback.errors.push(err!(first.span; "invalid color"));
                    take!(Expr::Color(RgbaColor::new_healed(0, 0, 0, 255)))
                }
            },

            Token::LeftParen => self.parse_tuple().map(|t| Expr::Tuple(t)),
            Token::LeftBrace => self.parse_object().map(|o| Expr::Object(o)),

            _ => return None,
        })
    }

    /// Parse a tuple expression: `(<expr>, ...)`.
    fn parse_tuple(&mut self) -> Spanned<Tuple> {
        let token = self.eat();
        debug_assert_eq!(token.map(Spanned::value), Some(Token::LeftParen));

        // Parse a collection until a right paren appears and complain about
        // missing a `value` when an invalid token is encoutered.
        self.parse_collection(Some(Token::RightParen),
            |p| p.parse_expr().ok_or(("value", None)))
    }

    /// Parse a tuple expression: `name(<expr>, ...)` with a given identifier.
    fn parse_named_tuple(&mut self, name: Spanned<Ident>) -> Spanned<NamedTuple> {
        let tuple = self.parse_tuple();
        let span = Span::merge(name.span, tuple.span);
        Spanned::new(NamedTuple::new(name, tuple), span)
    }

    /// Parse an object expression: `{ <key>: <value>, ... }`.
    fn parse_object(&mut self) -> Spanned<Object> {
        let token = self.eat();
        debug_assert_eq!(token.map(Spanned::value), Some(Token::LeftBrace));

        // Parse a collection until a right brace appears.
        self.parse_collection(Some(Token::RightBrace), |p| {
            // Expect an identifier as the key.
            let key = p.parse_ident().ok_or(("key", None))?;

            // Expect a colon behind the key (only separated by whitespace).
            let behind_key = p.pos();
            p.skip_whitespace();
            if p.peekv() != Some(Token::Colon) {
                return Err(("colon", Some(behind_key)));
            }

            p.eat();
            p.skip_whitespace();

            // Semantic highlighting for object keys.
            p.feedback.decos.push(
                Spanned::new(Decoration::ObjectKey, key.span));

            let value = p.parse_expr().ok_or(("value", None))?;

            Ok(Pair { key, value })
        })
    }

    /// Parse a comma-separated collection where each item is parsed through
    /// `parse_item` until the `end` token is met.
    fn parse_collection<C, I, F>(
        &mut self,
        end: Option<Token>,
        mut parse_item: F
    ) -> Spanned<C>
    where
        C: FromIterator<I>,
        F: FnMut(&mut Self) -> Result<I, (&'static str, Option<Position>)>,
    {
        let start = self.pos();

        // Parse the comma separated items.
        let collection = std::iter::from_fn(|| {
            self.skip_whitespace();
            let peeked = self.peekv();

            // We finished as expected.
            if peeked == end {
                self.eat();
                return None;
            }

            // We finished without the expected end token (which has to be a
            // `Some` value at this point since otherwise we would have already
            // returned in the previous case).
            if peeked == None {
                self.eat();
                self.expected_at(end.unwrap().name(), self.pos());
                return None;
            }

            // Try to parse a collection item.
            match parse_item(self) {
                Ok(item) => {
                    // Expect a comma behind the item (only separated by
                    // whitespace).
                    let behind_item = self.pos();
                    self.skip_whitespace();
                    match self.peekv() {
                        Some(Token::Comma) => { self.eat(); }
                        t @ Some(_) if t != end => self.expected_at("comma", behind_item),
                        _ => {}
                    }

                    return Some(Some(item));
                }

                // The item parser expected something different at either some
                // given position or instead of the currently peekable token.
                Err((expected, Some(pos))) => self.expected_at(expected, pos),
                Err((expected, None)) => {
                    let token = self.peek();
                    if token.map(Spanned::value) != end {
                        self.eat();
                    }
                    self.expected_found_or_at(expected, token, self.pos());
                }
            }

            Some(None)
        }).filter_map(|x| x).collect();

        let end = self.pos();
        Spanned::new(collection, Span { start, end })
    }

    /// Try to parse an identifier and do nothing if the peekable token is no
    /// identifier.
    fn parse_ident(&mut self) -> Option<Spanned<Ident>> {
        match self.peek() {
            Some(Spanned { v: Token::ExprIdent(s), span }) => {
                self.eat();
                Some(Spanned { v: Ident(s.to_string()), span })
            }
            _ => None
        }
    }

    /// Skip all whitespace/comment tokens.
    fn skip_whitespace(&mut self) {
        self.eat_until(|t| match t {
            Token::Space(_) | Token::LineComment(_) |
            Token::BlockComment(_) => false,
            _ => true,
        }, false)
    }

    /// Add an error about an expected `thing` which was not found, showing
    /// what was found instead.
    fn expected_found(&mut self, thing: &str, found: Spanned<Token>) {
        self.feedback.errors.push(err!(found.span;
            "expected {}, found {}", thing, found.v.name()));
    }

    /// Add an error about an `thing` which was expected but not found at the
    /// given position.
    fn expected_at(&mut self, thing: &str, pos: Position) {
        self.feedback.errors.push(err!(Span::at(pos); "expected {}", thing));
    }

    /// Add a expected-found-error if `found` is `Some` and an expected-error
    /// otherwise.
    fn expected_found_or_at(
        &mut self,
        thing: &str,
        found: Option<Spanned<Token>>,
        pos: Position
    ) {
        match found {
            Some(found) => self.expected_found(thing, found),
            None => self.expected_at(thing, pos),
        }
    }

    /// Consume tokens until the function returns true and only consume the last
    /// token if instructed to so by `eat_match`.
    fn eat_until<F>(&mut self, mut f: F, eat_match: bool)
    where F: FnMut(Token<'s>) -> bool {
        while let Some(token) = self.peek() {
            if f(token.v) {
                if eat_match {
                    self.eat();
                }
                break;
            }

            self.eat();
        }
    }

    /// Consume and return the next token.
    fn eat(&mut self) -> Option<Spanned<Token<'s>>> {
        self.peeked.take()
            .unwrap_or_else(|| self.tokens.next())
    }

    /// Peek at the next token without consuming it.
    fn peek(&mut self) -> Option<Spanned<Token<'s>>> {
        let iter = &mut self.tokens;
        *self.peeked.get_or_insert_with(|| iter.next())
    }

    /// Peek at the unspanned value of the next token.
    fn peekv(&mut self) -> Option<Token<'s>> {
        self.peek().map(Spanned::value)
    }

    /// The position at the end of the last eaten token / start of the peekable
    /// token.
    fn pos(&self) -> Position {
        self.peeked.flatten()
            .map(|s| s.span.start)
            .unwrap_or_else(|| self.tokens.pos())
    }
}

/// Unescape a string: `the string is \"this\"` => `the string is "this"`.
fn unescape_string(string: &str) -> String {
    let mut s = String::with_capacity(string.len());
    let mut iter = string.chars();

    while let Some(c) = iter.next() {
        if c == '\\' {
            match iter.next() {
                Some('\\') => s.push('\\'),
                Some('"') => s.push('"'),
                Some('n') => s.push('\n'),
                Some('t') => s.push('\t'),
                Some(c) => { s.push('\\'); s.push(c); }
                None => s.push('\\'),
            }
        } else {
            s.push(c);
        }
    }

    s
}

/// Unescape raw markup into lines.
fn unescape_raw(raw: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut s = String::new();
    let mut iter = raw.chars().peekable();

    while let Some(c) = iter.next() {
        if c == '\\' {
            match iter.next() {
                Some('`') => s.push('`'),
                Some(c) => { s.push('\\'); s.push(c); }
                None => s.push('\\'),
            }
        } else if is_newline_char(c) {
            if c == '\r' && iter.peek() == Some(&'\n') {
                iter.next();
            }

            lines.push(std::mem::replace(&mut s, String::new()));
        } else {
            s.push(c);
        }
    }

    lines.push(s);
    lines
}


#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::size::Size;
    use crate::syntax::test::{DebugFn, check, zspan};
    use crate::syntax::func::Value;
    use super::*;

    use Decoration::*;
    use Node::{
        Space as S, ToggleItalic as Italic, ToggleBolder as Bold,
        Parbreak, Linebreak,
    };

    use Expr::{Number as Num, Size as Sz, Bool};
    fn Id(text: &str) -> Expr { Expr::Ident(Ident(text.to_string())) }
    fn Str(text: &str) -> Expr { Expr::Str(text.to_string()) }
    fn Pt(points: f32) -> Expr { Expr::Size(Size::pt(points)) }

    fn Clr(r: u8, g: u8, b: u8, a: u8) -> Expr {
        Expr::Color(RgbaColor::new(r, g, b, a))
    }
    fn ClrStr(color: &str) -> Expr {
        Expr::Color(RgbaColor::from_str(color).expect("invalid test color"))
    }
    fn ClrStrHealed() -> Expr {
        let mut c = RgbaColor::from_str("000f").expect("invalid test color");
        c.healed = true;
        Expr::Color(c)
    }

    fn T(text: &str) -> Node { Node::Text(text.to_string()) }

    /// Create a raw text node.
    macro_rules! raw {
        ($($line:expr),* $(,)?) => {
            Node::Raw(vec![$($line.to_string()),*])
        };
    }

    /// Create a tuple expression.
    macro_rules! tuple {
        ($($items:expr),* $(,)?) => {
            Expr::Tuple(Tuple { items: spanned![vec $($items),*].0 })
        };
    }

    /// Create a named tuple expression.
    macro_rules! named_tuple {
        ($name:expr $(, $items:expr)* $(,)?) => {
            Expr::NamedTuple(NamedTuple::new(
                zspan(Ident($name.to_string())),
                zspan(Tuple { items: spanned![vec $($items),*].0 })
            ))
        };
    }

    /// Create an object expression.
    macro_rules! object {
        ($($key:expr => $value:expr),* $(,)?) => {
            Expr::Object(Object {
                pairs: vec![$(Pair {
                    key: zspan(Ident($key.to_string())),
                    value: zspan($value),
                }),*]
            })
        };
    }

    /// Test whether the given string parses into
    /// - the given node list (required).
    /// - the given error list (optional, if omitted checks against empty list).
    /// - the given decoration list (optional, if omitted it is not tested).
    macro_rules! p {
        ($source:expr => [$($model:tt)*]) => {
            p!($source => [$($model)*], []);
        };

        ($source:expr => [$($model:tt)*], [$($errors:tt)*] $(, [$($decos:tt)*])? $(,)?) => {
            let mut scope = Scope::new::<DebugFn>();
            scope.add::<DebugFn>("f");
            scope.add::<DebugFn>("n");
            scope.add::<DebugFn>("box");
            scope.add::<DebugFn>("val");

            let ctx = ParseContext { scope: &scope };
            let pass = parse(Position::ZERO, $source, ctx);

            // Test model
            let (exp, cmp) = spanned![vec $($model)*];
            check($source, exp, pass.output.nodes, cmp);

            // Test errors
            let (exp, cmp) = spanned![vec $($errors)*];
            let exp = exp.into_iter()
                .map(|s: Spanned<&str>| s.map(|e| e.to_string()))
                .collect::<Vec<_>>();
            let found = pass.feedback.errors.into_iter()
                .map(|s| s.map(|e| e.message))
                .collect::<Vec<_>>();
            check($source, exp, found, cmp);

            // Test decos
            $(let (exp, cmp) = spanned![vec $($decos)*];
            check($source, exp, pass.feedback.decos, cmp);)?
        };
    }

    /// Write down a `DebugFn` function model compactly.
    macro_rules! func {
        ($name:tt $(: ($($pos:tt)*), { $($key:tt)* } )? $(; $($body:tt)*)?) => ({
            #[allow(unused_mut)]
            let mut args = FuncArgs::new();
            $(args.pos = Tuple::parse(zspan(tuple!($($pos)*))).unwrap();)?
            $(args.key = Object::parse(zspan(object! { $($key)* })).unwrap();)?

            Node::Model(Box::new(DebugFn {
                header: FuncHeader {
                    name: spanned!(item $name).map(|s| Ident(s.to_string())),
                    args,
                },
                body: func!(@body $($($body)*)?),
            }))
        });

        (@body [$($body:tt)*]) => ({
            Some(SyntaxModel { nodes: spanned![vec $($body)*].0 })
        });
        (@body) => (None);
    }

    #[test]
    fn parse_color_strings() {
        assert_eq!(Clr(0xf6, 0x12, 0x43, 0xff), ClrStr("f61243ff"));
        assert_eq!(Clr(0xb3, 0xd8, 0xb3, 0xff), ClrStr("b3d8b3"));
        assert_eq!(Clr(0xfc, 0xd2, 0xa9, 0xad), ClrStr("fCd2a9AD"));
        assert_eq!(Clr(0x22, 0x33, 0x33, 0xff), ClrStr("233"));
        assert_eq!(Clr(0x11, 0x11, 0x11, 0xbb), ClrStr("111b"));
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
        fn test(raw: &str, expected: Node) {
            let vec = if let Node::Raw(v) = expected { v } else { panic!() };
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
        // Basic nodes
        p!(""                     => []);
        p!("hi"                   => [T("hi")]);
        p!("*hi"                  => [Bold, T("hi")]);
        p!("hi_"                  => [T("hi"), Italic]);
        p!("hi you"               => [T("hi"), S, T("you")]);
        p!("hi// you\nw"          => [T("hi"), S, T("w")]);
        p!("\n\n\nhello"          => [Parbreak, T("hello")]);
        p!("first//\n//\nsecond"  => [T("first"), S, S, T("second")]);
        p!("first//\n \nsecond"   => [T("first"), Parbreak, T("second")]);
        p!("first/*\n \n*/second" => [T("first"), T("second")]);
        p!(r"a\ b"                => [T("a"), Linebreak, S, T("b")]);
        p!("💜\n\n 🌍"            => [T("💜"), Parbreak, T("🌍")]);

        // Raw markup
        p!("`py`"         => [raw!["py"]]);
        p!("[val][`hi]`]" => [func!("val"; [raw!["hi]"]])]);
        p!("`hi\nyou"     => [raw!["hi", "you"]], [(1:3, 1:3, "expected backtick")]);
        p!("`hi\\`du`"    => [raw!["hi`du"]]);

        // Spanned nodes
        p!("Hi"      => [(0:0, 0:2, T("Hi"))]);
        p!("*Hi*"    => [(0:0, 0:1, Bold), (0:1, 0:3, T("Hi")), (0:3, 0:4, Bold)]);
        p!("🌎\n*/[n]" =>
            [(0:0, 0:1, T("🌎")), (0:1, 1:0, S), (1:2, 1:5, func!((0:1, 0:2, "n")))],
            [(1:0, 1:2, "unexpected end of block comment")],
            [(1:3, 1:4, ValidFuncName)],
        );
    }

    #[test]
    fn parse_function_names() {
        // No closing bracket
        p!("[" => [func!("")], [
            (0:1, 0:1, "expected identifier"),
            (0:1, 0:1, "expected closing bracket")
        ]);

        // No name
        p!("[]" => [func!("")], [(0:1, 0:1, "expected identifier")]);
        p!("[\"]" => [func!("")], [
            (0:1, 0:3, "expected identifier, found string"),
            (0:3, 0:3, "expected closing bracket"),
        ]);

        // An unknown name
        p!("[hi]" =>
            [func!("hi")],
            [(0:1, 0:3, "unknown function")],
            [(0:1, 0:3, InvalidFuncName)],
        );

        // A valid name
        p!("[f]"   => [func!("f")], [], [(0:1, 0:2, ValidFuncName)]);
        p!("[  f]" => [func!("f")], [], [(0:3, 0:4, ValidFuncName)]);

        // An invalid token for a name
        p!("[12]"   => [func!("")], [(0:1, 0:3, "expected identifier, found number")], []);
        p!("[🌎]"   => [func!("")], [(0:1, 0:2, "expected identifier, found invalid token")], []);
        p!("[  🌎]" => [func!("")], [(0:3, 0:4, "expected identifier, found invalid token")], []);
    }

    #[test]
    fn parse_colon_starting_function_arguments() {
        // No colon before arg
        p!("[val\"s\"]" => [func!("val")], [(0:4, 0:4, "expected colon")]);

        // No colon before valid, but wrong token
        p!("[val=]" => [func!("val")], [(0:4, 0:4, "expected colon")]);

        // No colon before invalid tokens, which are ignored
        p!("[val/🌎:$]" =>
            [func!("val")],
            [(0:4, 0:4, "expected colon")],
            [(0:1, 0:4, ValidFuncName)],
        );

        // String in invalid header without colon still parsed as string
        // Note: No "expected quote" error because not even the string was
        //       expected.
        p!("[val/\"]" => [func!("val")], [
            (0:4, 0:4, "expected colon"),
            (0:7, 0:7, "expected closing bracket"),
        ]);

        // Just colon without args
        p!("[val:]"         => [func!("val")]);
        p!("[val:/*12pt*/]" => [func!("val")]);

        // Whitespace / comments around colon
        p!("[val\n:\ntrue]"      => [func!("val": (Bool(true)), {})]);
        p!("[val/*:*/://\ntrue]" => [func!("val": (Bool(true)), {})]);
    }

    #[test]
    fn parse_one_positional_argument() {
        // Different expressions
        p!("[val: true]"   =>
            [func!("val": (Bool(true)), {})], [],
            [(0:1, 0:4, ValidFuncName)],
        );
        p!("[val: _]"      => [func!("val": (Id("_")), {})]);
        p!("[val: name]"   => [func!("val": (Id("name")), {})]);
        p!("[val: \"hi\"]" => [func!("val": (Str("hi")), {})]);
        p!("[val: \"a\n[]\\\"string\"]" => [func!("val": (Str("a\n[]\"string")), {})]);
        p!("[val: 3.14]"   => [func!("val": (Num(3.14)), {})]);
        p!("[val: 4.5cm]"  => [func!("val": (Sz(Size::cm(4.5))), {})]);
        p!("[val: 12e1pt]" => [func!("val": (Pt(12e1)), {})]);
        p!("[val: #f7a20500]" => [func!("val": (ClrStr("f7a20500")), {})]);

        // Unclosed string.
        p!("[val: \"hello]" => [func!("val": (Str("hello]")), {})], [
            (0:13, 0:13, "expected quote"),
            (0:13, 0:13, "expected closing bracket"),
        ]);

        //Invalid colors
        p!("[val: #12345]" => [func!("val": (ClrStrHealed()), {})], [
            (0:6, 0:12, "invalid color"),
        ]);
        p!("[val: #a5]" => [func!("val": (ClrStrHealed()), {})], [
            (0:6, 0:9, "invalid color"),
        ]);
        p!("[val: #14b2ah]" => [func!("val": (ClrStrHealed()), {})], [
            (0:6, 0:13, "invalid color"),
        ]);
        p!("[val: #f075ff011]" => [func!("val": (ClrStrHealed()), {})], [
            (0:6, 0:16, "invalid color"),
        ]);
    }

    #[test]
    fn parse_tuples() {
        // Empty tuple
        p!("[val: ()]" => [func!("val": (tuple!()), {})]);
        p!("[val: empty()]" => [func!("val": (named_tuple!("empty")), {})]);

        // Invalid value
        p!("[val: (🌎)]" =>
            [func!("val": (tuple!()), {})],
            [(0:7, 0:8, "expected value, found invalid token")],
        );
        p!("[val: sound(\x07)]" =>
            [func!("val": (named_tuple!("sound")), {})],
            [(0:12, 0:13, "expected value, found invalid token")],
        );

        // Invalid tuple name
        p!("[val: 👠(\"abc\", 13e-5)]" =>
            [func!("val": (tuple!(Str("abc"), Num(13.0e-5))), {})],
            [(0:6, 0:7, "expected argument, found invalid token")],
        );

        // Unclosed tuple
        p!("[val: (hello]" =>
            [func!("val": (tuple!(Id("hello"))), {})],
            [(0:12, 0:12, "expected closing paren")],
        );
        p!("[val: lang(中文]" =>
            [func!("val": (named_tuple!("lang", Id("中文"))), {})],
            [(0:13, 0:13, "expected closing paren")],
        );

        // Valid values
        p!("[val: (1, 2)]" => [func!("val": (tuple!(Num(1.0), Num(2.0))), {})]);
        p!("[val: (\"s\",)]" => [func!("val": (tuple!(Str("s"))), {})]);
        p!("[val: cmyk(1, 46, 0, 0)]" =>
            [func!("val": (named_tuple!(
                "cmyk", Num(1.0), Num(46.0), Num(0.0), Num(0.0)
            )), {})]
        );
        p!("[val: items(\"fire\", #f93a6d)]" =>
            [func!("val": (named_tuple!(
                "items", Str("fire"), ClrStr("f93a6d")
            )), {})]
        );

        // Nested tuples
        p!("[val: (1, (2))]" =>
            [func!("val": (tuple!(Num(1.0), tuple!(Num(2.0)))), {})]
        );
        p!("[val: css(1pt, rgb(90, 102, 254), \"solid\")]" =>
            [func!("val": (named_tuple!(
                "css", Pt(1.0), named_tuple!(
                    "rgb", Num(90.0), Num(102.0), Num(254.0)
                ), Str("solid")
            )), {})]
        );

        // Invalid commas
        p!("[val: (,)]" =>
            [func!("val": (tuple!()), {})],
            [(0:7, 0:8, "expected value, found comma")],
        );
        p!("[val: nose(,)]" =>
            [func!("val": (named_tuple!("nose")), {})],
            [(0:11, 0:12, "expected value, found comma")],
        );
        p!("[val: (true false)]" =>
            [func!("val": (tuple!(Bool(true), Bool(false))), {})],
            [(0:11, 0:11, "expected comma")],
        );
    }

    #[test]
    fn parse_objects() {
        let f = || func!("val": (object! {}), {});

        // Okay objects
        p!("[val: {}]" => [f()]);
        p!("[val: { key: value }]" =>
            [func!("val": (object! { "key" => Id("value") }), {})]);

        // Unclosed object
        p!("[val: {hello: world]" =>
            [func!("val": (object! { "hello" => Id("world") }), {})],
            [(0:19, 0:19, "expected closing brace")],
        );
        p!("[val: { a]" =>
            [func!("val": (object! {}), {})],
            [(0:9, 0:9, "expected colon"), (0:9, 0:9, "expected closing brace")],
        );

        // Missing key
        p!("[val: {,}]" => [f()], [(0:7, 0:8, "expected key, found comma")]);
        p!("[val: { 12pt }]" => [f()], [(0:8, 0:12, "expected key, found size")]);
        p!("[val: { : }]" => [f()], [(0:8, 0:9, "expected key, found colon")]);

        // Missing colon
        p!("[val: { key }]" => [f()], [(0:11, 0:11, "expected colon")]);
        p!("[val: { key false }]" => [f()], [
            (0:11, 0:11, "expected colon"),
            (0:12, 0:17, "expected key, found bool"),
        ]);
        p!("[val: { a b:c }]" =>
            [func!("val": (object! { "b" => Id("c") }), {})],
            [(0:9, 0:9, "expected colon")],
        );

        // Missing value
        p!("[val: { key: : }]" => [f()], [(0:13, 0:14, "expected value, found colon")]);
        p!("[val: { key: , k: \"s\" }]" =>
            [func!("val": (object! { "k" => Str("s") }), {})],
            [(0:13, 0:14, "expected value, found comma")],
        );

        // Missing comma, invalid token
        p!("[val: left={ a: 2, b: false 🌎 }]" =>
            [func!("val": (), {
                "left" => object! {
                    "a" => Num(2.0),
                    "b" => Bool(false),
                }
            })],
            [(0:27, 0:27, "expected comma"),
             (0:28, 0:29, "expected key, found invalid token")],
        );
    }

    #[test]
    fn parse_nested_tuples_and_objects() {
        p!("[val: (1, { ab: (), d: (3, 14pt) }), false]" => [func!("val": (
            tuple!(
                Num(1.0),
                object!(
                    "ab" => tuple!(),
                    "d" => tuple!(Num(3.0), Pt(14.0)),
                ),
            ),
            Bool(false),
        ), {})]);
    }

    #[test]
    fn parse_one_keyword_argument() {
        // Correct
        p!("[val: x=true]" =>
            [func!("val": (), { "x" => Bool(true) })], [],
            [(0:6, 0:7, ArgumentKey), (0:1, 0:4, ValidFuncName)],
        );

        // Spacing around keyword arguments
        p!("\n [val: \n hi \n = /* //\n */ \"s\n\"]" =>
            [S, func!("val": (), { "hi" => Str("s\n") })], [],
            [(2:1, 2:3, ArgumentKey), (1:2, 1:5, ValidFuncName)],
        );

        // Missing value
        p!("[val: x=]" =>
            [func!("val")],
            [(0:8, 0:8, "expected value")],
            [(0:6, 0:7, ArgumentKey), (0:1, 0:4, ValidFuncName)],
        );
    }

    #[test]
    fn parse_multiple_mixed_arguments() {
        p!("[val: a,]" => [func!("val": (Id("a")), {})]);
        p!("[val: 12pt, key=value]" =>
            [func!("val": (Pt(12.0)), { "key" => Id("value") })], [],
            [(0:12, 0:15, ArgumentKey), (0:1, 0:4, ValidFuncName)],
        );
        p!("[val: a , \"b\" , c]" => [func!("val": (Id("a"), Str("b"), Id("c")), {})]);
    }

    #[test]
    fn parse_invalid_values() {
        p!("[val: )]"     => [func!("val")], [(0:6, 0:7, "expected argument, found closing paren")]);
        p!("[val: }]"     => [func!("val")], [(0:6, 0:7, "expected argument, found closing brace")]);
        p!("[val: :]"     => [func!("val")], [(0:6, 0:7, "expected argument, found colon")]);
        p!("[val: ,]"     => [func!("val")], [(0:6, 0:7, "expected argument, found comma")]);
        p!("[val: =]"     => [func!("val")], [(0:6, 0:7, "expected argument, found equals sign")]);
        p!("[val: 🌎]"    => [func!("val")], [(0:6, 0:7, "expected argument, found invalid token")]);
        p!("[val: 12ept]" => [func!("val")], [(0:6, 0:11, "expected argument, found invalid token")]);
        p!("[val: [hi]]"  =>
            [func!("val")],
            [(0:6, 0:10, "expected argument, found function")],
            [(0:1, 0:4, ValidFuncName)],
        );
    }

    #[test]
    fn parse_invalid_key_value_pairs() {
        // Invalid keys
        p!("[val: true=you]" =>
            [func!("val": (Bool(true), Id("you")), {})],
            [(0:10, 0:10, "expected comma"),
             (0:10, 0:11, "expected argument, found equals sign")],
            [(0:1, 0:4, ValidFuncName)],
        );

        p!("[box: z=y=4]" =>
            [func!("box": (Num(4.0)), { "z" => Id("y") })],
            [(0:9, 0:9, "expected comma"),
             (0:9, 0:10, "expected argument, found equals sign")],
        );

        // Invalid colon after keyable positional argument
        p!("[val: key:12]" =>
            [func!("val": (Id("key"), Num(12.0)), {})],
            [(0:9, 0:9, "expected comma"),
             (0:9, 0:10, "expected argument, found colon")],
            [(0:1, 0:4, ValidFuncName)],
        );

        // Invalid colon after non-keyable positional argument
        p!("[val: true:12]" => [func!("val": (Bool(true), Num(12.0)), {})],
            [(0:10, 0:10, "expected comma"),
             (0:10, 0:11, "expected argument, found colon")],
            [(0:1, 0:4, ValidFuncName)],
        );
    }

    #[test]
    fn parse_invalid_commas() {
        // Missing commas
        p!("[val: 1pt 1]" =>
            [func!("val": (Pt(1.0), Num(1.0)), {})],
            [(0:9, 0:9, "expected comma")],
        );
        p!(r#"[val: _"s"]"# =>
            [func!("val": (Id("_"), Str("s")), {})],
            [(0:7, 0:7, "expected comma")],
        );

        // Unexpected commas
        p!("[val:,]" => [func!("val")], [(0:5, 0:6, "expected argument, found comma")]);
        p!("[val: key=,]" => [func!("val")], [(0:10, 0:11, "expected value, found comma")]);
        p!("[val:, true]" =>
            [func!("val": (Bool(true)), {})],
            [(0:5, 0:6, "expected argument, found comma")],
        );
    }

    #[test]
    fn parse_bodies() {
        p!("[val][Hi]" => [func!("val"; [T("Hi")])]);

        // Body nodes in bodies.
        p!("[val:*][*Hi*]" =>
            [func!("val"; [Bold, T("Hi"), Bold])],
            [(0:5, 0:6, "expected argument, found invalid token")],
        );

        // Errors in bodies.
        p!(" [val][ */ ]" =>
            [S, func!("val"; [S, S])],
            [(0:8, 0:10, "unexpected end of block comment")],
        );
    }

    #[test]
    fn parse_spanned_functions() {
        // Space before function
        p!(" [val]" =>
            [(0:0, 0:1, S), (0:1, 0:6, func!((0:1, 0:4, "val")))], [],
            [(0:2, 0:5, ValidFuncName)],
        );

        // Newline before function
        p!(" \n\r\n[val]" =>
            [(0:0, 2:0, Parbreak), (2:0, 2:5, func!((0:1, 0:4, "val")))], [],
            [(2:1, 2:4, ValidFuncName)],
        );

        // Content before function
        p!("hello [val][world] 🌎" =>
            [
                (0:0, 0:5, T("hello")),
                (0:5, 0:6, S),
                (0:6, 0:18, func!((0:1, 0:4, "val"); [(0:6, 0:11, T("world"))])),
                (0:18, 0:19, S),
                (0:19, 0:20, T("🌎"))
            ], [],
            [(0:7, 0:10, ValidFuncName)],
        );

        // Nested function
        p!(" [val][\nbody[ box]\n ]" =>
            [
                (0:0, 0:1, S),
                (0:1, 2:2, func!((0:1, 0:4, "val"); [
                    (0:6, 1:0, S),
                    (1:0, 1:4, T("body")),
                    (1:4, 1:10, func!((0:2, 0:5, "box"))),
                    (1:10, 2:1, S),
                ]))
            ], [],
            [(0:2, 0:5, ValidFuncName), (1:6, 1:9, ValidFuncName)],
        );
    }
}
