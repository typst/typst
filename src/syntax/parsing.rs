//! Parsing of source code into syntax models.

use crate::{Pass, Feedback};
use super::expr::*;
use super::func::{FuncHeader, FuncArgs, FuncArg};
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

    /// The spanned body string if there is a body. The string itself is just
    /// the parsed without the brackets, while the span includes the brackets.
    /// ```typst
    /// [tokens][body]
    ///         ^^^^^^
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
                spanned(Expr::Str(string.to_string()))
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


#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::size::Size;
    use super::super::test::{DebugFn, SpanlessEq};
    use super::*;

    use Node::{
        Space as S, Newline as N,
        ToggleItalic as Italic, ToggleBolder as Bold, ToggleMonospace as Mono,
    };
    use Decoration::*;

    pub use Expr::{Number as Num, Bool};
    pub fn Id(text: &str) -> Expr { Expr::Ident(Ident(text.to_string())) }
    pub fn Str(text: &str) -> Expr { Expr::Str(text.to_string()) }

    fn T(text: &str) -> Node { Node::Text(text.to_string()) }

    /// Test whether the given string parses into the given node list.
    macro_rules! p {
        ($s:expr => [$($b:tt)*]) => {
            let ctx = ParseContext { scope: &scope() };
            let model = parse(Position::ZERO, $s, ctx).output;
            let (expected, cmp) = model!([$($b)*]);

            if !cmp(&model, &expected) {
                fail($s, model, expected);
            }
        };
    }

    /// Test whether the given string yields the given parse errors.
    macro_rules! e {
        ($s:expr => [$(($sl:tt:$sc:tt, $el:tt:$ec:tt, $e:expr)),* $(,)?]) => {
            let ctx = ParseContext { scope: &scope() };
            let errors = parse(Position::ZERO, $s, ctx).feedback
                .errors
                .into_iter()
                .map(|s| s.map(|e| e.message))
                .collect::<Vec<_>>();

            let expected = vec![
                $(Spanned {
                    v: $e.to_string(),
                    span: Span {
                        start: Position { line: $sl, column: $sc },
                        end:   Position { line: $el, column: $ec },
                    },
                }),*
            ];

            if errors != expected {
                fail($s, errors, expected);
            }
        };
    }

    /// Test whether the given string yields the given decorations.
    macro_rules! d {
        ($s:expr => [$(($sl:tt:$sc:tt, $el:tt:$ec:tt, $d:expr)),* $(,)?]) => {
            let ctx = ParseContext { scope: &scope() };
            let decos = parse(Position::ZERO, $s, ctx).feedback.decos;

            let expected = vec![
                $(Spanned {
                    v: $d,
                    span: Span {
                        start: Position { line: $sl, column: $sc },
                        end:   Position { line: $el, column: $ec },
                    },
                }),*
            ];

            if decos != expected {
                fail($s, decos, expected);
            }
        };
    }

    fn scope() -> Scope {
        let mut scope = Scope::new::<DebugFn>();
        scope.add::<DebugFn>("f");
        scope.add::<DebugFn>("n");
        scope.add::<DebugFn>("box");
        scope
    }

    fn fail(src: &str, found: impl Debug, expected: impl Debug) {
        eprintln!("source:   {:?}", src);
        eprintln!("found:    {:#?}", found);
        eprintln!("expected: {:#?}", expected);
        panic!("test failed");
    }

    /// Parse a list of optionally spanned nodes into a syntax model.
    macro_rules! model {
        ([$(($sl:tt:$sc:tt, $el:tt:$ec:tt, $n:expr)),* $(,)?]) => ((SyntaxModel {
            nodes: vec![
                $(Spanned { v: $n, span: Span {
                    start: Position { line: $sl, column: $sc },
                    end:   Position { line: $el, column: $ec },
                }}),*
            ]
        }, <SyntaxModel as PartialEq>::eq));

        ([$($e:tt)*]) => ((SyntaxModel {
            nodes: vec![$($e)*].into_iter().map(zspan).collect::<Vec<_>>()
        }, <SyntaxModel as SpanlessEq>::spanless_eq));
    }

    /// Build a `DebugFn` function model.
    macro_rules! func {
        ($name:expr
         $(,pos: [$($item:expr),* $(,)?])?
         $(,key: [$($key:expr => $value:expr),* $(,)?])?;
         $($b:tt)*) => ({
            #![allow(unused_mut, unused_assignments)]

            let mut pos = Tuple::new();
            let mut key = Object::new();
            $(pos = Tuple { items: vec![$(zspan($item)),*] };)?
            $(key = Object {
                pairs: vec![$(Pair {
                    key: zspan(Ident($key.to_string())),
                    value: zspan($value),
                }),*]
            };)?

            Node::Model(Box::new(DebugFn {
                header: FuncHeader {
                    name: zspan(Ident($name.to_string())),
                    args: FuncArgs {
                        pos,
                        key,
                    },
                },
                body: func!(@body $($b)*),
            }))
        });

        (@body Some([$($b:tt)*])) => (Some(model!([$($b)*]).0));
        (@body None) => (None);
    }

    /// Span an element with a zero span.
    fn zspan<T>(v: T) -> Spanned<T> {
        Spanned { v, span: Span::ZERO }
    }

    #[test]
    fn parse_flat_nodes() {
        p!(""           => []);
        p!("hi"         => [T("hi")]);
        p!("*hi"        => [Bold, T("hi")]);
        p!("hi_"        => [T("hi"), Italic]);
        p!("`py`"       => [Mono, T("py"), Mono]);
        p!("hi you"     => [T("hi"), S, T("you")]);
        p!("💜\n\n 🌍"  => [T("💜"), N, T("🌍")]);
    }

    #[test]
    fn parse_functions() {
        p!("[func]" => [func!("func"; None)]);
        p!("[tree][hi *you*]" => [func!("tree"; Some([T("hi"), S, Bold, T("you"), Bold]))]);
        p!("[f: , hi, * \"du\"]" => [func!("f", pos: [Id("hi"), Str("du")]; None)]);
        p!("from [align: left] to" => [
            T("from"), S, func!("align", pos: [Id("left")]; None), S, T("to")
        ]);

        p!("[f: left, 12pt, false]" => [
            func!("f", pos: [Id("left"), Expr::Size(Size::pt(12.0)), Bool(false)]; None)
        ]);

        p!("[box: x=1.2pt, false][a b c] bye" => [
            func!(
                "box",
                pos: [Bool(false)],
                key: ["x" => Expr::Size(Size::pt(1.2))];
                Some([T("a"), S, T("b"), S, T("c")])
            ),
            S, T("bye"),
        ]);
    }

    #[test]
    fn parse_spanned() {
        p!("hi you" => [(0:0, 0:2, T("hi")), (0:2, 0:3, S), (0:3, 0:6, T("you"))]);
    }

    #[test]
    fn parse_errors() {
        e!("[f: , hi, * \"du\"]" => [
            (0:4,  0:5,  "expected value, found comma"),
            (0:10, 0:11, "expected value, found invalid token"),
        ]);
        e!("[f:, , ,]" => [
            (0:3, 0:4, "expected value, found comma"),
            (0:5, 0:6, "expected value, found comma"),
            (0:7, 0:8, "expected value, found comma"),
        ]);
        e!("[f:" => [(0:3, 0:3, "expected closing bracket")]);
        e!("[f: hi" => [(0:6, 0:6, "expected closing bracket")]);
        e!("[f: hey   12pt]" => [(0:7, 0:7, "expected comma")]);
        e!("[box: x=, false z=y=4" => [
            (0:8,  0:9,  "expected value, found comma"),
            (0:15, 0:15, "expected comma"),
            (0:19, 0:19, "expected comma"),
            (0:19, 0:20, "expected value, found equals sign"),
            (0:21, 0:21, "expected closing bracket"),
        ]);
    }

    #[test]
    fn parse_decos() {
        d!("*Technische Universität Berlin* [n]\n                                [n]"
            => [(0:33, 0:34, ValidFuncName), (1:33, 1:34, ValidFuncName)]);
    }
}
