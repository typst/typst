//! Parsing of source code into syntax models.

use crate::error::Errors;
use super::expr::*;
use super::func::{FuncHeader, FuncArgs, FuncArg};
use super::scope::Scope;
use super::span::{Position, Span, Spanned, SpanVec, offset_spans};
use super::tokens::{Token, Tokens, TokenizationMode};
use super::*;


/// The context for parsing.
#[derive(Debug, Copy, Clone)]
pub struct ParseContext<'a> {
    /// The scope containing function definitions.
    pub scope: &'a Scope,
}

/// The result of parsing: Some parsed thing, errors and decorations for syntax
/// highlighting.
pub struct Parsed<T> {
    /// The result of the parsing process.
    pub output: T,
    /// Errors that arose in the parsing process.
    pub errors: Errors,
    /// Decorations for semantic syntax highlighting.
    pub decorations: SpanVec<Decoration>,
}

impl<T> Parsed<T> {
    /// Map the output type and keep errors and decorations.
    pub fn map<F, U>(self, f: F) -> Parsed<U> where F: FnOnce(T) -> U {
        Parsed {
            output: f(self.output),
            errors: self.errors,
            decorations: self.decorations,
        }
    }
}

/// Parse source code into a syntax model.
///
/// All errors and decorations are offset by the `start` position.
pub fn parse(start: Position, src: &str, ctx: ParseContext) -> Parsed<SyntaxModel> {
    let mut model = SyntaxModel::new();
    let mut errors = Vec::new();
    let mut decorations = Vec::new();

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
                let parsed: Parsed<Node> = FuncParser::new(header, body, ctx).parse();

                // Collect the errors and decorations from the function parsing,
                // but offset their spans by the start of the function since
                // they are function-local.
                errors.extend(offset_spans(parsed.errors, span.start));
                decorations.extend(offset_spans(parsed.decorations, span.start));

                if !terminated {
                    errors.push(err!(Span::at(span.end); "expected closing bracket"));
                }

                parsed.output
            }

            Token::Star       => Node::ToggleBolder,
            Token::Underscore => Node::ToggleItalic,
            Token::Backtick   => Node::ToggleMonospace,
            Token::Text(text) => Node::Text(text.to_string()),

            Token::LineComment(_) | Token::BlockComment(_) => continue,

            other => {
                errors.push(err!(span; "unexpected {}", other.name()));
                continue;
            }
        };

        model.add(Spanned { v: node, span: token.span });
    }

    Parsed { output: model, errors, decorations }
}

/// Performs the function parsing.
struct FuncParser<'s> {
    ctx: ParseContext<'s>,
    errors: Errors,
    decorations: SpanVec<Decoration>,

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
            errors: vec![],
            decorations: vec![],
            tokens: Tokens::new(Position::new(0, 1), header, TokenizationMode::Header),
            peeked: None,
            body,
        }
    }

    /// Do the parsing.
    fn parse(mut self) -> Parsed<Node> {
        let parsed = if let Some(header) = self.parse_func_header() {
            let name = header.name.v.as_str();
            let (parser, deco) = match self.ctx.scope.get_parser(name) {
                // A valid function.
                Ok(parser) => (parser, Decoration::ValidFuncName),

                // The fallback parser was returned. Invalid function.
                Err(parser) => {
                    self.errors.push(err!(header.name.span; "unknown function"));
                    (parser, Decoration::InvalidFuncName)
                }
            };

            self.decorations.push(Spanned::new(deco, header.name.span));

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

        self.errors.extend(parsed.errors);
        self.decorations.extend(parsed.decorations);

        Parsed {
            output: Node::Model(parsed.output),
            errors: self.errors,
            decorations: self.decorations,
        }
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

                self.decorations.push(Spanned::new(Decoration::ArgumentKey, span));

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
        self.eat_until(|t| !matches!(t,
            Token::Space(_) | Token::LineComment(_) |
            Token::BlockComment(_)), false)
    }

    /// Add an error about an expected `thing` which was not found, showing
    /// what was found instead.
    fn expected_found(&mut self, thing: &str, found: Spanned<Token>) {
        self.errors.push(err!(found.span;
            "expected {}, found {}", thing, found.v.name()));
    }

    /// Add an error about an `thing` which was expected but not found at the
    /// given position.
    fn expected_at(&mut self, thing: &str, pos: Position) {
        self.errors.push(err!(Span::at(pos); "expected {}", thing));
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
