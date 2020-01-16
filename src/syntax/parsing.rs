use crate::func::Scope;
use super::*;
use Token::*;


/// Parses source code into a syntax tree given a context.
pub fn parse(src: &str, ctx: ParseContext) -> (SyntaxTree, Colorization, ErrorMap) {
    Parser::new(src, ctx).parse()
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
    colorization: Colorization,
    error_map: ErrorMap,
    tokens: Tokens<'s>,
    peeked: Option<Option<Spanned<Token<'s>>>>,
    position: Position,
    last_position: Position,
}

impl<'s> Parser<'s> {
    fn new(src: &'s str, ctx: ParseContext<'s>) -> Parser<'s> {
        Parser {
            src,
            ctx,
            error_map: ErrorMap::new(),
            colorization: Colorization::new(),
            tokens: Tokens::new(src),
            peeked: None,
            position: Position::ZERO,
            last_position: Position::ZERO,
        }
    }

    /// The main parsing entrypoint.
    fn parse(mut self) -> (SyntaxTree, Colorization, ErrorMap) {
        let mut tree = SyntaxTree::new();

        loop {
            if let Some(spanned) = self.eat() {
                match spanned.v {
                    LineComment(_) | BlockComment(_) => {}

                    Whitespace(newlines) => {
                        tree.add(spanned.map_v(if newlines >= 2 {
                            Node::Newline
                        } else {
                            Node::Space
                        }));
                    }

                    LeftBracket => {
                        if let Some(func) = self.parse_func() {
                            tree.add(func);
                        }
                    }

                    Star       => tree.add(spanned.map_v(Node::ToggleBolder)),
                    Underscore => tree.add(spanned.map_v(Node::ToggleItalic)),
                    Backtick   => tree.add(spanned.map_v(Node::ToggleMonospace)),
                    Text(text) => tree.add(spanned.map_v(Node::Text(text.to_owned()))),

                    _ => self.unexpected(spanned),
                }
            } else {
                break;
            }
        }

        (tree, self.colorization, self.error_map)
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

        let call = self.parse_func_call(header)?;

        let end = self.pos();
        let span = Span { start, end };

        Some(Spanned { v: Node::Func(call), span })
    }

    /// Parses a function header including the closing bracket.
    fn parse_func_header(&mut self) -> Option<FuncHeader> {
        self.skip_whitespace();
        let name = self.parse_func_name()?;

        self.skip_whitespace();
        let args = match self.eat() {
            Some(Spanned { v: Colon, .. }) => self.parse_func_args(),
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
        match self.eat() {
            Some(Spanned { v: ExprIdent(ident), span }) => {
                self.colorization.replace_last(ColorToken::FuncName);
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
                    Some(Arg::Pos(item)) => args.add_pos(item),
                    Some(Arg::Key(pair)) => args.add_key_pair(pair),
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
                self.colorization.replace_last(ColorToken::Key);

                self.eat();
                self.skip_whitespace();

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

    /// Parse the body of a function invocation.
    fn parse_func_call(&mut self, header: Option<FuncHeader>) -> Option<FuncCall> {
        let body = if self.peekv() == Some(LeftBracket) {
            self.eat();

            let start = self.tokens.index();
            let found = self.tokens.move_to_closing_bracket();
            let end = self.tokens.index();

            self.last_position = self.position;
            self.position = self.tokens.pos();

            let body = &self.src[start .. end];

            if found {
                let next = self.eat().map(Spanned::value);
                debug_assert_eq!(next, Some(RightBracket));
            } else {
                self.expected_at("closing bracket", self.pos());
            }

            Some(body)
        } else {
            None
        };

        let header = header?;
        let parser = self.ctx.scope.get_parser(header.name.v.as_str()).or_else(|| {
            let message = format!("unknown function: `{}`", header.name.v);
            self.error_map.add(message, header.name.span);
            None
        })?;

        Some(FuncCall(parser(header, body, self.ctx).unwrap()))
    }

    /// Skip all whitespace/comment tokens.
    fn skip_whitespace(&mut self) {
        self.eat_until(|t|
            !matches!(t, Whitespace(_) | LineComment(_) | BlockComment(_)), false)
    }

    /// Add an error about an `thing` which was expected but not found at the
    /// given position.
    fn expected_at(&mut self, thing: &str, pos: Position) {
        self.error_map.add_at(format!("expected {}", thing), pos);
    }

    /// Add an error about an expected `thing` which was not found, showing
    /// what was found instead.
    fn expected_found(&mut self, thing: &str, found: Spanned<Token>) {
        let message = format!("expected {}, found {}", thing, name(found.v));
        self.error_map.add(message, found.span);
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
        self.error_map.add(format!("unexpected {}", name(found.v)), found.span);
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
            if let Some(color) = color(token.v) {
                self.colorization.add(color, token.span);
            }

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

/// The color token corresponding to a token.
fn color(token: Token) -> Option<ColorToken> {
    Some(match token {
        LineComment(_) | BlockComment(_) => ColorToken::Comment,
        LeftBracket    | RightBracket    => ColorToken::Bracket,
        LeftParen      | RightParen      => ColorToken::Paren,
        LeftBrace      | RightBrace      => ColorToken::Brace,
        Colon         => ColorToken::Colon,
        Comma         => ColorToken::Comma,
        Equals        => ColorToken::Equals,
        ExprIdent(_)  => ColorToken::ExprIdent,
        ExprStr(_)    => ColorToken::ExprStr,
        ExprNumber(_) => ColorToken::ExprNumber,
        ExprSize(_)   => ColorToken::ExprSize,
        ExprBool(_)   => ColorToken::ExprBool,
        StarSlash     => ColorToken::Invalid,
        _ => return None,
    })
}
