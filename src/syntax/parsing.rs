//! Parsing of source code into syntax models.

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
            // Only at least two newlines mean a _real_ newline indicating a
            // paragraph break.
            Token::Space(newlines) => if newlines >= 2 {
                Node::Newline
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
            Token::Backtick   => Node::ToggleMonospace,
            Token::Text(text) => Node::Text(text.to_string()),

            Token::LineComment(_) | Token::BlockComment(_) => continue,

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

        let name = match self.eat() {
            Some(Spanned { v: Token::ExprIdent(ident), span }) => {
                Spanned { v: Ident(ident.to_string()), span }
            }
            other => {
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

    /// Parse the function arguments after a colon.
    fn parse_func_args(&mut self) -> FuncArgs {
        let mut args = FuncArgs::new();

        self.skip_whitespace();
        while self.peek().is_some() {
            match self.parse_arg() {
                Some(arg) => args.add(arg),
                None => {}
            }

            self.skip_whitespace();
        }

        args
    }

    /// Parse a positional or keyword argument.
    fn parse_arg(&mut self) -> Option<FuncArg> {
        let first = self.peek()?;
        let span = first.span;

        let arg = if let Token::ExprIdent(ident) = first.v {
            self.eat();
            self.skip_whitespace();

            let ident = Ident(ident.to_string());
            if let Some(Token::Equals) = self.peekv() {
                self.eat();
                self.skip_whitespace();

                self.feedback.decos.push(Spanned::new(Decoration::ArgumentKey, span));

                self.parse_expr().map(|value| {
                    FuncArg::Key(Pair {
                        key: Spanned { v: ident, span },
                        value,
                    })
                })
            } else {
                Some(FuncArg::Pos(Spanned::new(Expr::Ident(ident), span)))
            }
        } else {
            self.parse_expr().map(|expr| FuncArg::Pos(expr))
        };

        if let Some(arg) = &arg {
            self.skip_whitespace();
            match self.peekv() {
                Some(Token::Comma) => { self.eat(); }
                Some(_) => self.expected_at("comma", arg.span().end),
                _ => {}
            }
        } else {
            let found = self.eat();
            self.expected_found_or_at("value", found, self.pos());
        }

        arg
    }

    /// Parse an atomic or compound (tuple / object) expression.
    fn parse_expr(&mut self) -> Option<Spanned<Expr>> {
        let first = self.peek()?;
        let spanned = |v| Spanned { v, span: first.span };

        Some(match first.v {
            Token::ExprIdent(i) => {
                self.eat();
                spanned(Expr::Ident(Ident(i.to_string())))
            }
            Token::ExprStr { string, terminated } => {
                if !terminated {
                    self.expected_at("quote", first.span.end);
                }

                self.eat();
                spanned(Expr::Str(unescape(string)))
            }
            Token::ExprNumber(n) => { self.eat(); spanned(Expr::Number(n)) }
            Token::ExprSize(s) => { self.eat(); spanned(Expr::Size(s)) }
            Token::ExprBool(b) => { self.eat(); spanned(Expr::Bool(b)) }

            Token::LeftParen => self.parse_tuple(),
            Token::LeftBrace => self.parse_object(),

            _ => return None,
        })
    }

    /// Parse a tuple expression.
    fn parse_tuple(&mut self) -> Spanned<Expr> {
        let start = self.pos();

        // TODO: Do the thing.
        self.eat_until(|t| t == Token::RightParen, true);

        let end = self.pos();
        let span = Span { start, end };

        Spanned { v: Expr::Tuple(Tuple::new()), span }
    }

    /// Parse an object expression.
    fn parse_object(&mut self) -> Spanned<Expr> {
        let start = self.pos();

        // TODO: Do the thing.
        self.eat_until(|t| t == Token::RightBrace, true);

        let end = self.pos();
        let span = Span { start, end };

        Spanned { v: Expr::Object(Object::new()), span }
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

/// Unescape a string.
fn unescape(string: &str) -> String {
    let mut s = String::with_capacity(string.len());
    let mut escaped = false;

    for c in string.chars() {
        if c == '\\' {
            if escaped {
                s.push('\\');
            }
            escaped = !escaped;
        } else {
            if escaped {
                match c {
                    '"' => s.push('"'),
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    c => { s.push('\\'); s.push(c); }
                }
            } else {
                s.push(c);
            }

            escaped = false;
        }
    }

    s
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
        Space as S, Newline as N,
        ToggleItalic as Italic, ToggleBolder as Bold, ToggleMonospace as Mono,
    };

    use Expr::{Number as Num, Size as Sz, Bool};
    fn Id(text: &str) -> Expr { Expr::Ident(Ident(text.to_string())) }
    fn Str(text: &str) -> Expr { Expr::Str(text.to_string()) }
    fn Pt(points: f32) -> Expr { Expr::Size(Size::pt(points)) }
    fn T(text: &str) -> Node { Node::Text(text.to_string()) }

    /// Create a tuple expression.
    macro_rules! tuple {
        ($($items:expr),* $(,)?) => {
            Expr::Tuple(Tuple { items: spanned![vec $($items),*].0 })
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

    /// Test whether the given string parses into the given transform pass.
    macro_rules! test {
        ($source:expr => [$($model:tt)*], $transform:expr) => {
            let (exp, cmp) = spanned![vec $($model)*];

            let mut scope = Scope::new::<DebugFn>();
            scope.add::<DebugFn>("f");
            scope.add::<DebugFn>("n");
            scope.add::<DebugFn>("box");
            scope.add::<DebugFn>("val");
            let ctx = ParseContext { scope: &scope };

            let found = parse(Position::ZERO, $source, ctx);
            let (exp, found) = $transform(exp, found);

            check($source, exp, found, cmp);
        };
    }

    /// Test whether the given string parses into the given node list.
    macro_rules! p {
        ($($tts:tt)*) => {
            test!($($tts)*, |exp, found: Pass<SyntaxModel>| (exp, found.output.nodes));
        };
    }

    /// Test whether the given string yields the given parse errors.
    macro_rules! e {
        ($($tts:tt)*) => {
            test!($($tts)*, |exp: Vec<Spanned<&str>>, found: Pass<SyntaxModel>| (
                exp.into_iter().map(|s| s.map(|e| e.to_string())).collect::<Vec<_>>(),
                found.feedback.errors.into_iter().map(|s| s.map(|e| e.message))
                    .collect::<Vec<_>>()
            ));
        };
    }

    /// Test whether the given string yields the given decorations.
    macro_rules! d {
        ($($tts:tt)*) => {
            test!($($tts)*, |exp, found: Pass<SyntaxModel>| (exp, found.feedback.decos));
        };
    }

    /// Write down a `DebugFn` function model compactly.
    macro_rules! func {
        ($name:tt $(, ($($pos:tt)*), { $($key:tt)* } )? $(; $($body:tt)*)?) => ({
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
    fn unescape_strings() {
        fn test(string: &str, expected: &str) {
            assert_eq!(unescape(string), expected.to_string());
        }

        test(r#"hello world"#,  "hello world");
        test(r#"hello\nworld"#, "hello\nworld");
        test(r#"a\"bc"#,        "a\"bc");
        test(r#"a\\"#,          "a\\");
        test(r#"a\\\nbc"#,      "a\\\nbc");
        test(r#"a\tbc"#,        "a\tbc");
        test("🌎",              "🌎");
    }

    #[test]
    fn parse_flat_nodes() {
        p!(""                     => []);
        p!("hi"                   => [T("hi")]);
        p!("*hi"                  => [Bold, T("hi")]);
        p!("hi_"                  => [T("hi"), Italic]);
        p!("`py`"                 => [Mono, T("py"), Mono]);
        p!("hi you"               => [T("hi"), S, T("you")]);
        p!("hi// you\nw"          => [T("hi"), S, T("w")]);
        p!("\n\n\nhello"          => [N, T("hello")]);
        p!("first//\n//\nsecond"  => [T("first"), S, S, T("second")]);
        p!("first//\n \nsecond"   => [T("first"), N, T("second")]);
        p!("first/*\n \n*/second" => [T("first"), T("second")]);
        p!("💜\n\n 🌍"            => [T("💜"), N, T("🌍")]);

        p!("Hi" => [(0:0, 0:2, T("Hi"))]);
        p!("*Hi*" => [(0:0, 0:1, Bold), (0:1, 0:3, T("Hi")), (0:3, 0:4, Bold)]);
        p!("🌎*/[n]" => [(0:0, 0:1, T("🌎")), (0:3, 0:6, func!((0:1, 0:2, "n")))]);

        e!("hi\n */" => [(1:1, 1:3, "unexpected end of block comment")]);
    }

    #[test]
    fn parse_function_names() {
        // No closing bracket
        p!("[" => [func!("")]);
        e!("[" => [
            (0:1, 0:1, "expected identifier"),
            (0:1, 0:1, "expected closing bracket")
        ]);

        // No name
        p!("[]" => [func!("")]);
        e!("[]" => [(0:1, 0:1, "expected identifier")]);

        p!("[\"]" => [func!("")]);
        e!("[\"]" => [
            (0:1, 0:3, "expected identifier, found string"),
            (0:3, 0:3, "expected closing bracket"),
        ]);

        // A valid name
        p!("[f]" => [func!("f")]);
        e!("[f]" => []);
        d!("[f]" => [(0:1, 0:2, ValidFuncName)]);
        p!("[  f]" => [func!("f")]);
        e!("[  f]" => []);
        d!("[  f]" => [(0:3, 0:4, ValidFuncName)]);

        // An unknown name
        p!("[hi]" => [func!("hi")]);
        e!("[hi]" => [(0:1, 0:3, "unknown function")]);
        d!("[hi]" => [(0:1, 0:3, InvalidFuncName)]);

        // An invalid token
        p!("[🌎]" => [func!("")]);
        e!("[🌎]" => [(0:1, 0:2, "expected identifier, found invalid token")]);
        d!("[🌎]" => []);
        p!("[  🌎]" => [func!("")]);
        e!("[  🌎]" => [(0:3, 0:4, "expected identifier, found invalid token")]);
        d!("[  🌎]" => []);
    }

    #[test]
    fn parse_colon_starting_function_arguments() {
        // No colon before arg
        p!("[val\"s\"]" => [func!("val")]);
        e!("[val\"s\"]" => [(0:4, 0:4, "expected colon")]);

        // No colon before valid, but wrong token
        p!("[val=]" => [func!("val")]);
        e!("[val=]" => [(0:4, 0:4, "expected colon")]);

        // No colon before invalid tokens, which are ignored
        p!("[val/🌎:$]" => [func!("val")]);
        e!("[val/🌎:$]" => [(0:4, 0:4, "expected colon")]);
        d!("[val/🌎:$]" => [(0:1, 0:4, ValidFuncName)]);

        // String in invalid header without colon still parsed as string
        // Note: No "expected quote" error because not even the string was
        //       expected.
        e!("[val/\"]" => [
            (0:4, 0:4, "expected colon"),
            (0:7, 0:7, "expected closing bracket"),
        ]);

        // Just colon without args
        p!("[val:]" => [func!("val")]);
        e!("[val:]" => []);
        p!("[val:/*12pt*/]" => [func!("val")]);

        // Whitespace / comments around colon
        p!("[val\n:\ntrue]"      => [func!("val", (Bool(true)), {})]);
        p!("[val/*:*/://\ntrue]" => [func!("val", (Bool(true)), {})]);
        e!("[val/*:*/://\ntrue]" => []);
    }

    #[test]
    fn parse_one_positional_argument() {
        // Different expressions
        d!("[val: true]"   => [(0:1, 0:4, ValidFuncName)]);
        p!("[val: true]"   => [func!("val", (Bool(true)), {})]);
        p!("[val: _]"      => [func!("val", (Id("_")), {})]);
        p!("[val: name]"   => [func!("val", (Id("name")), {})]);
        p!("[val: \"hi\"]" => [func!("val", (Str("hi")), {})]);
        p!("[val: \"a\n[]\\\"string\"]" => [func!("val", (Str("a\n[]\"string")), {})]);
        p!("[val: 3.14]"   => [func!("val", (Num(3.14)), {})]);
        p!("[val: 4.5cm]"  => [func!("val", (Sz(Size::cm(4.5))), {})]);
        p!("[val: 12e1pt]" => [func!("val", (Pt(12e1)), {})]);

        // Unclosed string.
        p!("[val: \"hello]" => [func!("val", (Str("hello]")), {})]);
        e!("[val: \"hello]" => [
            (0:13, 0:13, "expected quote"),
            (0:13, 0:13, "expected closing bracket"),
        ]);

        // Tuple: unimplemented
        p!("[val: ()]" => [func!("val", (tuple!()), {})]);

        // Object: unimplemented
        p!("[val: {}]" => [func!("val", (object! {}), {})]);
    }

    #[test]
    fn parse_one_keyword_argument() {
        // Correct
        p!("[val: x=true]" => [func!("val", (), { "x" => Bool(true) })]);
        d!("[val: x=true]" => [(0:6, 0:7, ArgumentKey), (0:1, 0:4, ValidFuncName)]);

        // Spacing around keyword arguments
        p!("\n [val: \n hi \n = /* //\n */ \"s\n\"]" => [S, func!("val", (), { "hi" => Str("s\n") })]);
        d!("\n [val: \n hi \n = /* //\n */ \"s\n\"]" => [(2:1, 2:3, ArgumentKey), (1:2, 1:5, ValidFuncName)]);
        e!("\n [val: \n hi \n = /* //\n */ \"s\n\"]" => []);

        // Missing value
        p!("[val: x=]" => [func!("val")]);
        e!("[val: x=]" => [(0:8, 0:8, "expected value")]);
        d!("[val: x=]" => [(0:6, 0:7, ArgumentKey), (0:1, 0:4, ValidFuncName)]);
    }

    #[test]
    fn parse_multiple_mixed_arguments() {
        p!("[val: a,]" => [func!("val", (Id("a")), {})]);
        e!("[val: a,]" => []);
        p!("[val: 12pt, key=value]" => [func!("val", (Pt(12.0)), { "key" => Id("value") })]);
        d!("[val: 12pt, key=value]" => [(0:12, 0:15, ArgumentKey), (0:1, 0:4, ValidFuncName)]);
        e!("[val: 12pt, key=value]" => []);
        p!("[val: a , \"b\" , c]" => [func!("val", (Id("a"), Str("b"), Id("c")), {})]);
        e!("[val: a , \"b\" , c]" => []);
    }

    #[test]
    fn parse_invalid_values() {
        e!("[val: )]"     => [(0:6, 0:7, "expected value, found closing paren")]);
        e!("[val: }]"     => [(0:6, 0:7, "expected value, found closing brace")]);
        e!("[val: :]"     => [(0:6, 0:7, "expected value, found colon")]);
        e!("[val: ,]"     => [(0:6, 0:7, "expected value, found comma")]);
        e!("[val: =]"     => [(0:6, 0:7, "expected value, found equals sign")]);
        e!("[val: 🌎]"    => [(0:6, 0:7, "expected value, found invalid token")]);
        e!("[val: 12ept]" => [(0:6, 0:11, "expected value, found invalid token")]);
        e!("[val: [hi]]"  => [(0:6, 0:10, "expected value, found function")]);
        d!("[val: [hi]]"  => [(0:1, 0:4, ValidFuncName)]);
    }

    #[test]
    fn parse_invalid_key_value_pairs() {
        // Invalid keys
        p!("[val: true=you]" => [func!("val", (Bool(true), Id("you")), {})]);
        e!("[val: true=you]" => [
            (0:10, 0:10, "expected comma"),
            (0:10, 0:11, "expected value, found equals sign"),
        ]);
        d!("[val: true=you]" => [(0:1, 0:4, ValidFuncName)]);

        p!("[box: z=y=4]" => [func!("box", (Num(4.0)), { "z" => Id("y") })]);
        e!("[box: z=y=4]" => [
            (0:9, 0:9, "expected comma"),
            (0:9, 0:10, "expected value, found equals sign"),
        ]);

        // Invalid colon after keyable positional argument
        p!("[val: key:12]" => [func!("val", (Id("key"), Num(12.0)), {})]);
        e!("[val: key:12]" => [
            (0:9, 0:9, "expected comma"),
            (0:9, 0:10, "expected value, found colon"),
        ]);
        d!("[val: key:12]" => [(0:1, 0:4, ValidFuncName)]);

        // Invalid colon after non-keyable positional argument
        p!("[val: true:12]" => [func!("val", (Bool(true), Num(12.0)), {})]);
        e!("[val: true:12]" => [
            (0:10, 0:10, "expected comma"),
            (0:10, 0:11, "expected value, found colon"),
        ]);
        d!("[val: true:12]" => [(0:1, 0:4, ValidFuncName)]);
    }

    #[test]
    fn parse_invalid_commas() {
        // Missing commas
        p!("[val: 1pt 1]" => [func!("val", (Pt(1.0), Num(1.0)), {})]);
        e!("[val: 1pt 1]" => [(0:9, 0:9, "expected comma")]);
        p!(r#"[val: _"s"]"# => [func!("val", (Id("_"), Str("s")), {})]);
        e!(r#"[val: _"s"]"# => [(0:7, 0:7, "expected comma")]);

        // Unexpected commas
        p!("[val:,]" => [func!("val")]);
        e!("[val:,]" => [(0:5, 0:6, "expected value, found comma")]);
        p!("[val:, true]" => [func!("val", (Bool(true)), {})]);
        e!("[val:, true]" => [(0:5, 0:6, "expected value, found comma")]);
        p!("[val: key=,]" => [func!("val")]);
        e!("[val: key=,]" => [(0:10, 0:11, "expected value, found comma")]);
    }

    #[test]
    fn parse_bodies() {
        p!("[val][Hi]" => [func!("val"; [T("Hi")])]);

        // Body nodes in bodies.
        p!("[val:*][*Hi*]" => [func!("val"; [Bold, T("Hi"), Bold])]);
        e!("[val:*][*Hi*]" => [(0:5, 0:6, "expected value, found invalid token")]);

        // Errors in bodies.
        p!(" [val][ */ ]" => [S, func!("val"; [S, S])]);
        e!(" [val][ */ ]" => [(0:8, 0:10, "unexpected end of block comment")]);
    }

    #[test]
    fn parse_spanned_functions() {
        // Space before function
        p!(" [val]" => [(0:0, 0:1, S), (0:1, 0:6, func!((0:1, 0:4, "val")))]);
        d!(" [val]" => [(0:2, 0:5, ValidFuncName)]);

        // Newline before function
        p!(" \n\r\n[val]" => [(0:0, 2:0, N), (2:0, 2:5, func!((0:1, 0:4, "val")))]);
        d!(" \n\r\n[val]" => [(2:1, 2:4, ValidFuncName)]);

        // Content before function
        p!("hello [val][world] 🌎" => [
            (0:0, 0:5, T("hello")),
            (0:5, 0:6, S),
            (0:6, 0:18, func!((0:1, 0:4, "val"); [(0:6, 0:11, T("world"))])),
            (0:18, 0:19, S),
            (0:19, 0:20, T("🌎")),
        ]);
        d!("hello [val][world] 🌎" => [(0:7, 0:10, ValidFuncName)]);
        e!("hello [val][world] 🌎" => []);

        // Nested function
        p!(" [val][\nbody[ box]\n ]" => [
            (0:0, 0:1, S),
            (0:1, 2:2, func!((0:1, 0:4, "val"); [
                (0:6, 1:0, S),
                (1:0, 1:4, T("body")),
                (1:4, 1:10, func!((0:2, 0:5, "box"))),
                (1:10, 2:1, S),
            ]))
        ]);
        d!(" [val][\nbody[ box]\n ]" => [
            (0:2, 0:5, ValidFuncName),
            (1:6, 1:9, ValidFuncName)
        ]);
    }
}
