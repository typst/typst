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
            error_map: ErrorMap { errors: vec![] },
            colorization: Colorization { tokens: vec![] },

            tokens: Tokens::new(src),
            peeked: None,
            position: Position::ZERO,
            last_position: Position::ZERO,
        }
    }

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

    fn parse_func(&mut self) -> Option<Spanned<Node>> {
        let start = self.last_pos();

        let header = self.parse_func_header();
        let call = self.parse_func_call(header)?;

        let end = self.pos();
        let span = Span { start, end };

        Some(Spanned { v: Node::Func(call), span })
    }

    fn parse_func_header(&mut self) -> Option<FuncHeader> {
        self.skip_whitespace();

        let name = self.parse_func_name().or_else(|| {
            self.eat_until(|t| t == RightBracket, true);
            None
        })?;

        self.skip_whitespace();
        let args = match self.eat() {
            Some(Spanned { v: Colon, .. }) => self.parse_func_args(),
            Some(Spanned { v: RightBracket, .. }) => FuncArgs::new(),
            other => {
                self.expected("colon or closing bracket", other);
                self.eat_until(|t| t == RightBracket, true);
                FuncArgs::new()
            }
        };

        Some(FuncHeader { name, args })
    }

    fn parse_func_call(&mut self, header: Option<FuncHeader>) -> Option<FuncCall> {
        let body = if self.peek() == Some(LeftBracket) {
            self.eat();

            let start = self.tokens.index();
            let found = self.tokens.move_to_closing_bracket();
            let end = self.tokens.index();

            self.last_position = self.position;
            self.position = self.tokens.pos();

            let body = &self.src[start .. end];

            if found {
                assert_eq!(self.eat().map(Spanned::value), Some(RightBracket));
            } else {
                self.error_here("expected closing bracket");
            }

            Some(body)
        } else {
            None
        };

        let header = header?;
        let parser = self.ctx.scope.get_parser(header.name.v.as_str()).or_else(|| {
            self.error(
                format!("unknown function: `{}`", header.name.v),
                header.name.span
            );
            None
        })?;

        Some(FuncCall(parser(header, body, self.ctx).unwrap()))
    }

    fn parse_func_name(&mut self) -> Option<Spanned<Ident>> {
        match self.eat() {
            Some(Spanned { v: ExprIdent(ident), span }) => {
                self.color(Spanned { v: ColorToken::FuncName, span }, true);
                Some(Spanned { v: Ident(ident.to_string()), span })
            }
            other => {
                self.expected("identifier", other);
                None
            }
        }
    }

    fn parse_func_args(&mut self) -> FuncArgs {
        // todo!()
        self.eat_until(|t| t == RightBracket, true);
        FuncArgs::new()
    }

    fn parse_tuple(&mut self) -> Spanned<Expression> {
        todo!("parse_tuple")
    }

    fn parse_object(&mut self) -> Spanned<Expression> {
        todo!("parse_object")
    }

    fn skip_whitespace(&mut self) {
        self.eat_until(|t| match t {
            Whitespace(_) | LineComment(_) | BlockComment(_) => false,
            _ => true,
        }, false)
    }

    fn expected(&mut self, thing: &str, found: Option<Spanned<Token>>) {
        if let Some(Spanned { v: found, span }) = found {
            self.error(
                format!("expected {}, found {}", thing, name(found)),
                span
            );
        } else {
            self.error_here(format!("expected {}", thing));
        }
    }

    fn unexpected(&mut self, found: Spanned<Token>) {
        self.error_map.errors.push(found.map(|t| format!("unexpected {}", name(t))));
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.error_map.errors.push(Spanned { v: message.into(), span });
    }

    fn error_here(&mut self, message: impl Into<String>) {
        self.error(message, Span::at(self.pos()));
    }

    fn color(&mut self, token: Spanned<ColorToken>, replace_last: bool) {
        if replace_last {
            if let Some(last) = self.colorization.tokens.last_mut() {
                *last = token;
                return;
            }
        }

        self.colorization.tokens.push(token);
    }

    fn color_token(&mut self, token: Spanned<Token<'s>>) {
        let colored = match token.v {
            LineComment(_) | BlockComment(_) => Some(ColorToken::Comment),
            StarSlash                  => Some(ColorToken::Invalid),
            LeftBracket | RightBracket => Some(ColorToken::Bracket),
            LeftParen | RightParen     => Some(ColorToken::Paren),
            LeftBrace | RightBrace     => Some(ColorToken::Brace),
            Colon         => Some(ColorToken::Colon),
            Comma         => Some(ColorToken::Comma),
            Equals        => Some(ColorToken::Equals),
            ExprIdent(_)  => Some(ColorToken::ExprIdent),
            ExprStr(_)    => Some(ColorToken::ExprStr),
            ExprNumber(_) => Some(ColorToken::ExprNumber),
            ExprSize(_)   => Some(ColorToken::ExprSize),
            ExprBool(_)   => Some(ColorToken::ExprBool),
            _ => None,
        };

        if let Some(color) = colored {
            self.colorization.tokens.push(Spanned { v: color, span: token.span });
        }
    }

    fn eat_until<F>(&mut self, mut f: F, eat_match: bool)
    where F: FnMut(Token<'s>) -> bool {
        while let Some(token) = self.peek() {
            if f(token) {
                if eat_match {
                    self.eat();
                }
                break;
            }

            self.eat();
        }
    }

    fn eat(&mut self) -> Option<Spanned<Token<'s>>> {
        let token = self.peeked.take().unwrap_or_else(|| self.tokens.next());

        self.last_position = self.position;
        if let Some(spanned) = token {
            self.color_token(spanned);
            self.position = spanned.span.end;
        }

        token
    }

    fn peek(&mut self) -> Option<Token<'s>> {
        let iter = &mut self.tokens;
        self.peeked
            .get_or_insert_with(|| iter.next())
            .map(Spanned::value)
    }

    fn pos(&self) -> Position {
        self.position
    }

    fn last_pos(&self) -> Position {
        self.last_position
    }
}

fn name(token: Token) -> &'static str {
    match token {
        Whitespace(_) => "whitespace",
        LineComment(_) | BlockComment(_) => "comment",
        StarSlash => "end of block comment",
        LeftBracket => "opening bracket",
        RightBracket => "closing bracket",
        LeftParen => "opening paren",
        RightParen => "closing paren",
        LeftBrace => "opening brace",
        RightBrace => "closing brace",
        Colon => "colon",
        Comma => "comma",
        Equals => "equals sign",
        ExprIdent(_) => "identifier",
        ExprStr(_) => "string",
        ExprNumber(_) => "number",
        ExprSize(_) => "size",
        ExprBool(_) => "bool",
        Star => "star",
        Underscore => "underscore",
        Backtick => "backtick",
        Text(_) => "text",
    }
}
