use crate::func::Scope;
use super::*;
use Token::*;


pub fn parse(start: Position, src: &str, ctx: ParseContext) -> Parsed<SyntaxModel> {
    Parser::new(start, src, ctx).parse()
}

/// The context for parsing.
#[derive(Debug, Copy, Clone)]
pub struct ParseContext<'a> {
    /// The scope containing function definitions.
    pub scope: &'a Scope,
}

struct Parser<'s> {
    src: &'s str,
    ctx: ParseContext<'s>,
    tokens: Tokens<'s>,
    peeked: Option<Option<Spanned<Token<'s>>>>,
    position: Position,
    last_position: Position,
    errors: SpanVec<Error>,
    decorations: SpanVec<Decoration>,
}

impl<'s> Parser<'s> {
    fn new(start: Position, src: &'s str, ctx: ParseContext<'s>) -> Parser<'s> {
        Parser {
            src,
            ctx,
            tokens: tokenize(start, src),
            peeked: None,
            position: Position::ZERO,
            last_position: Position::ZERO,
            errors: vec![],
            decorations: vec![],
        }
    }

    /// The main parsing entrypoint.
    fn parse(mut self) -> Parsed<SyntaxModel> {
        let mut model = SyntaxModel::new();

        while let Some(token) = self.eat() {
            let mut span = token.span;
            let node = match token.v {
                LineComment(_) | BlockComment(_) => None,
                Whitespace(newlines) => Some(if newlines >= 2 {
                    Node::Newline
                } else {
                    Node::Space
                }),

                LeftBracket => self.parse_func().map(|spanned| {
                    span = spanned.span;
                    spanned.v
                }),

                Star       => Some(Node::ToggleBolder),
                Underscore => Some(Node::ToggleItalic),
                Backtick   => Some(Node::ToggleMonospace),
                Text(text) => Some(Node::Text(text.to_owned())),

                _ => {
                    self.unexpected(token);
                    None
                }
            };

            if let Some(v) = node {
                model.add(Spanned { v, span });
            }
        }

        Parsed {
            output: model,
            errors: self.errors,
            decorations: self.decorations,
        }
    }

    /// Parses a function including header and body with the cursor starting
    /// right behind the first opening bracket.
    fn parse_func(&mut self) -> Option<Spanned<Node>> {
        let start = self.last_pos();

        let header = self.parse_func_header();
        self.eat_until(|t| t == RightBracket, false);

        if self.eat().map(Spanned::value) != Some(RightBracket) {
            self.expected_at("closing bracket", self.pos());
        }

        let body = if self.peekv() == Some(LeftBracket) {
            self.eat();

            let start_index = self.tokens.index();
            let start_position = self.tokens.pos();

            let found = self.tokens.move_to_closing_bracket();

            let end_index = self.tokens.index();
            let end_position = self.tokens.pos();

            let body = &self.src[start_index .. end_index];

            self.position = end_position;

            if found {
                let next = self.eat().map(Spanned::value);
                debug_assert_eq!(next, Some(RightBracket));
            } else {
                self.expected_at("closing bracket", self.pos());
            }

            Some(Spanned::new(body, Span::new(start_position, end_position)))
        } else {
            None
        };

        let header = header?;
        let (parser, decoration) = match self.ctx.scope.get_parser(header.name.v.as_str()) {
            Ok(parser) => (parser, Decoration::ValidFuncName),
            Err(parser) => {
                let error = Error::new(format!("unknown function: `{}`", header.name.v));
                self.errors.push(Spanned::new(error, header.name.span));
                (parser, Decoration::InvalidFuncName)
            }
        };

        self.decorations.push(Spanned::new(decoration, header.name.span));

        let parsed = parser(header, body, self.ctx);
        self.errors.extend(offset_spans(parsed.errors, start));
        self.decorations.extend(offset_spans(parsed.decorations, start));

        let node = Node::Model(parsed.output);

        let end = self.pos();
        let span = Span { start, end };

        Some(Spanned { v: node, span })
    }

    /// Parses a function header including the closing bracket.
    fn parse_func_header(&mut self) -> Option<FuncHeader> {
        self.skip_whitespace();
        let name = self.parse_func_name()?;

        self.skip_whitespace();
        let args = match self.peek() {
            Some(Spanned { v: Colon, .. }) => {
                self.eat();
                self.parse_func_args()
            }
            Some(Spanned { v: RightBracket, .. }) => FuncArgs::new(),
            other => {
                self.expected_at("colon or closing bracket", name.span.end);
                FuncArgs::new()
            }
        };

        Some(FuncHeader { name, args })
    }

    /// Parses the function name if is the next token. Otherwise, it adds an
    /// error and returns `None`.
    fn parse_func_name(&mut self) -> Option<Spanned<Ident>> {
        match self.peek() {
            Some(Spanned { v: ExprIdent(ident), span }) => {
                self.eat();
                return Some(Spanned { v: Ident(ident.to_string()), span });
            }
            other => self.expected_found_or_at("identifier", other, self.pos()),
        }

        None
    }

    /// Parses the function arguments and stops right before the final closing
    /// bracket.
    fn parse_func_args(&mut self) -> FuncArgs {
        let mut args = FuncArgs::new();

        loop {
            self.skip_whitespace();
            match self.peekv() {
                Some(RightBracket) | None => break,
                _ => match self.parse_arg() {
                    Some(arg) => args.add(arg),
                    None => {}
                }
            }
        }

        args
    }

    /// Parse a positional or keyword argument.
    fn parse_arg(&mut self) -> Option<Arg> {
        let first = self.peek()?;
        let span = first.span;

        let arg = if let ExprIdent(ident) = first.v {
            self.eat();
            self.skip_whitespace();

            let ident = Ident(ident.to_string());
            if let Some(Equals) = self.peekv() {
                self.eat();
                self.skip_whitespace();

                self.decorations.push(Spanned::new(Decoration::ArgumentKey, span));

                self.parse_expr().map(|value| {
                    Arg::Key(Pair {
                        key: Spanned { v: ident, span },
                        value,
                    })
                })
            } else {
                Some(Arg::Pos(Spanned::new(Expression::Ident(ident), span)))
            }
        } else {
            self.parse_expr().map(|expr| Arg::Pos(expr))
        };

        if let Some(arg) = &arg {
            self.skip_whitespace();
            match self.peekv() {
                Some(RightBracket) => {}
                Some(Comma) => { self.eat(); }
                Some(_) => self.expected_at("comma", arg.span().end),
                _ => {}
            }
        } else {
            let found = self.eat();
            self.expected_found_or_at("value", found, self.pos());
        }

        arg
    }

    /// Parse a atomic or compound (tuple / object) expression.
    fn parse_expr(&mut self) -> Option<Spanned<Expression>> {
        let first = self.peek()?;
        let mut expr = |v| {
            self.eat();
            Spanned { v, span: first.span }
        };

        Some(match first.v {
            ExprIdent(i) => expr(Expression::Ident(Ident(i.to_string()))),
            ExprStr(s) => expr(Expression::Str(s.to_string())),
            ExprNumber(n) => expr(Expression::Number(n)),
            ExprSize(s) => expr(Expression::Size(s)),
            ExprBool(b) => expr(Expression::Bool(b)),
            LeftParen => self.parse_tuple(),
            LeftBrace => self.parse_object(),
            _ => return None,
        })
    }

    /// Parse a tuple expression.
    fn parse_tuple(&mut self) -> Spanned<Expression> {
        let start = self.pos();

        // TODO: Do the thing.
        self.eat_until(|t| matches!(t, RightParen | RightBracket), false);
        if self.peekv() == Some(RightParen) {
            self.eat();
        }

        let end = self.pos();
        let span = Span { start, end };

        Spanned { v: Expression::Tuple(Tuple::new()), span }
    }

    /// Parse an object expression.
    fn parse_object(&mut self) -> Spanned<Expression> {
        let start = self.pos();

        // TODO: Do the thing.
        self.eat_until(|t| matches!(t, RightBrace | RightBracket), false);
        if self.peekv() == Some(RightBrace) {
            self.eat();
        }

        let end = self.pos();
        let span = Span { start, end };

        Spanned { v: Expression::Object(Object::new()), span }
    }

    /// Skip all whitespace/comment tokens.
    fn skip_whitespace(&mut self) {
        self.eat_until(|t|
            !matches!(t, Whitespace(_) | LineComment(_) | BlockComment(_)), false)
    }

    /// Add an error about an `thing` which was expected but not found at the
    /// given position.
    fn expected_at(&mut self, thing: &str, pos: Position) {
        let error = Error::new(format!("expected {}", thing));
        self.errors.push(Spanned::new(error, Span::at(pos)));
    }

    /// Add an error about an expected `thing` which was not found, showing
    /// what was found instead.
    fn expected_found(&mut self, thing: &str, found: Spanned<Token>) {
        let message = format!("expected {}, found {}", thing, name(found.v));
        let error = Error::new(message);
        self.errors.push(Spanned::new(error, found.span));
    }

    /// Add a found-error if `found` is some and a positional error, otherwise.
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

    /// Add an error about an unexpected token `found`.
    fn unexpected(&mut self, found: Spanned<Token>) {
        let error = Error::new(format!("unexpected {}", name(found.v)));
        self.errors.push(Spanned::new(error, found.span));
    }

    /// Consume tokens until the function returns true and only consume the last
    /// token if instructed to.
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

    /// Consume and return the next token, update positions and colorize the
    /// token. All colorable tokens are per default colorized here, to override
    /// a colorization use `Colorization::replace_last`.
    fn eat(&mut self) -> Option<Spanned<Token<'s>>> {
        let token = self.peeked.take()
            .unwrap_or_else(|| self.tokens.next());

        if let Some(token) = token {
            self.last_position = self.position;
            self.position = token.span.end;
        }

        token
    }

    /// Peek at the next token without consuming it.
    fn peek(&mut self) -> Option<Spanned<Token<'s>>> {
        let iter = &mut self.tokens;
        *self.peeked.get_or_insert_with(|| iter.next())
    }

    fn peekv(&mut self) -> Option<Token<'s>> {
        self.peek().map(Spanned::value)
    }

    /// The position at the end of the last eat token / start of the peekable
    /// token.
    fn pos(&self) -> Position {
        self.position
    }

    /// The position at the start of the last eaten token.
    fn last_pos(&self) -> Position {
        self.last_position
    }
}

/// The name of a token in an `expected <...>` error.
fn name(token: Token) -> &'static str {
    match token {
        Whitespace(_) => "whitespace",
        LineComment(_) | BlockComment(_) => "comment",
        StarSlash     => "end of block comment",
        LeftBracket   => "opening bracket",
        RightBracket  => "closing bracket",
        LeftParen     => "opening paren",
        RightParen    => "closing paren",
        LeftBrace     => "opening brace",
        RightBrace    => "closing brace",
        Colon         => "colon",
        Comma         => "comma",
        Equals        => "equals sign",
        ExprIdent(_)  => "identifier",
        ExprStr(_)    => "string",
        ExprNumber(_) => "number",
        ExprSize(_)   => "size",
        ExprBool(_)   => "bool",
        Star          => "star",
        Underscore    => "underscore",
        Backtick      => "backtick",
        Text(_)       => "invalid identifier",
    }
}
