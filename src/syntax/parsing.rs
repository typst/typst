use std::iter::Peekable;

use crate::func::Scope;
use super::*;
use Token::*;


/// A tree representation of source code.
#[derive(Debug, PartialEq)]
pub struct SyntaxTree {
    pub nodes: Vec<Spanned<Node>>,
}

impl SyntaxTree {
    /// Create an empty syntax tree.
    pub fn new() -> SyntaxTree {
        SyntaxTree { nodes: vec![] }
    }
}

/// A node in the syntax tree.
#[derive(Debug, PartialEq)]
pub enum Node {
    /// A number of whitespace characters containing less than two newlines.
    Space,
    /// Whitespace characters with more than two newlines.
    Newline,
    /// Plain text.
    Text(String),
    /// Italics enabled / disabled.
    ToggleItalic,
    /// Bolder enabled / disabled.
    ToggleBolder,
    /// Monospace enabled / disabled.
    ToggleMonospace,
    /// A function invocation.
    Func(FuncCall),
}

/// An invocation of a function.
#[derive(Debug)]
pub struct FuncCall(pub Box<dyn LayoutFunc>);

impl PartialEq for FuncCall {
    fn eq(&self, other: &FuncCall) -> bool {
        &self.0 == &other.0
    }
}

#[derive(Debug)]
pub struct FuncArgs {
    positional: Tuple,
    keyword: Object,
}

impl FuncArgs {
    fn new() -> FuncArgs {
        FuncArgs {
            positional: Tuple::new(),
            keyword: Object::new(),
        }
    }

    /// Add a positional argument.
    pub fn add_pos(&mut self, item: Spanned<Expression>) {
        self.positional.add(item);
    }

    /// Force-extract the first positional argument.
    pub fn get_pos<E: ExpressionKind>(&mut self) -> ParseResult<E> {
        expect(self.get_pos_opt())
    }

    /// Extract the first positional argument.
    pub fn get_pos_opt<E: ExpressionKind>(&mut self) -> ParseResult<Option<E>> {
        Ok(if !self.positional.items.is_empty() {
            let spanned = self.positional.items.remove(0);
            Some(E::from_expr(spanned)?)
        } else {
            None
        })
    }

    /// Add a keyword argument.
    pub fn add_key(&mut self, key: Spanned<Ident>, value: Spanned<Expression>) {
        self.keyword.add(key, value);
    }

    /// Add a keyword argument from an existing pair.
    pub fn add_key_pair(&mut self, pair: Pair) {
        self.keyword.add_pair(pair);
    }

    /// Force-extract a keyword argument.
    pub fn get_key<E: ExpressionKind>(&mut self, name: &str) -> ParseResult<E> {
        expect(self.get_key_opt(name))
    }

    /// Extract a keyword argument.
    pub fn get_key_opt<E: ExpressionKind>(&mut self, name: &str) -> ParseResult<Option<E>> {
        self.keyword.pairs.iter()
            .position(|p| p.key.v.0 == name)
            .map(|index| {
                let value = self.keyword.pairs.swap_remove(index).value;
                E::from_expr(value)
            })
            .transpose()
    }

    /// Iterator over positional arguments.
    pub fn iter_pos(&mut self) -> std::vec::IntoIter<Spanned<Expression>> {
        let tuple = std::mem::replace(&mut self.positional, Tuple::new());
        tuple.items.into_iter()
    }

    /// Iterator over all keyword arguments.
    pub fn iter_keys(&mut self) -> std::vec::IntoIter<Pair> {
        let object = std::mem::replace(&mut self.keyword, Object::new());
        object.pairs.into_iter()
    }

    /// Clear the argument lists.
    pub fn clear(&mut self) {
        self.positional.items.clear();
        self.keyword.pairs.clear();
    }

    /// Whether both the positional and keyword argument lists are empty.
    pub fn is_empty(&self) -> bool {
        self.positional.items.is_empty() && self.keyword.pairs.is_empty()
    }
}

/// Extract the option expression kind from the option or return an error.
fn expect<E: ExpressionKind>(opt: ParseResult<Option<E>>) -> ParseResult<E> {
    match opt {
        Ok(Some(spanned)) => Ok(spanned),
        Ok(None) => error!("expected {}", E::NAME),
        Err(e) => Err(e),
    }
}

/// Parses source code into a syntax tree given a context.
pub fn parse(src: &str, ctx: ParseContext) -> SyntaxTree {
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
    tokens: Peekable<Tokens<'s>>,
    errors: Vec<Spanned<String>>,
    colored: Vec<Spanned<ColorToken>>,
    span: Span,
}

macro_rules! defer {
    ($($tts:tt)*) => (
        unimplemented!()
    );
}

impl<'s> Parser<'s> {
    fn new(src: &'s str, ctx: ParseContext<'s>) -> Parser<'s> {
        Parser {
            src,
            ctx,
            tokens: Tokens::new(src).peekable(),
            errors: vec![],
            colored: vec![],
            span: Span::ZERO,
        }
    }

    fn parse(mut self) -> SyntaxTree {
        let mut tree = SyntaxTree::new();

        loop {
            self.skip_whitespace();

            let start = self.position();

            let node = match self.next() {
                Some(LeftBracket) => self.parse_func().map(|f| Node::Func(f)),
                Some(Star) => Some(Node::ToggleBolder),
                Some(Underscore) => Some(Node::ToggleItalic),
                Some(Backtick) => Some(Node::ToggleMonospace),
                Some(Text(text)) => Some(Node::Text(text.to_owned())),
                Some(other) => { self.unexpected(other); None },
                None => break,
            };

            if let Some(node) = node {
                let end = self.position();
                let span = Span { start, end };

                tree.nodes.push(Spanned { v: node, span });
            }
        }

        tree
    }

    fn parse_func(&mut self) -> Option<FuncCall> {
        let (name, args) = self.parse_func_header()?;
        self.parse_func_call(name, args)
    }

    fn parse_func_header(&mut self) -> Option<(Spanned<Ident>, FuncArgs)> {
        defer! { self.eat_until(|t| t == RightBracket, true); }

        self.skip_whitespace();

        let name = self.parse_func_name()?;

        self.skip_whitespace();

        let args = match self.next() {
            Some(Colon) => self.parse_func_args(),
            Some(RightBracket) => FuncArgs::new(),
            other => {
                self.expected("colon or closing bracket", other);
                FuncArgs::new()
            }
        };

        Some((name, args))
    }

    fn parse_func_call(
        &mut self,
        name: Spanned<Ident>,
        args: FuncArgs,
    ) -> Option<FuncCall> {
        unimplemented!()
    }

    fn parse_func_name(&mut self) -> Option<Spanned<Ident>> {
        match self.next() {
            Some(ExprIdent(ident)) => {
                self.color_span(ColorToken::FuncName, self.span(), true);
                Some(Spanned { v: Ident(ident.to_string()), span: self.span() })
            }
            other => {
                self.expected("identifier", other);
                None
            }
        }
    }

    fn parse_func_args(&mut self) -> FuncArgs {
        enum State {
            Start,
            Identifier(Spanned<Ident>),
            Assignment(Spanned<Ident>),
            Value,
        }

        impl State {
            fn expected(&self) -> &'static str {
                match self {
                    State::Start => "value or key",
                    State::Identifier(_) => "comma or assignment",
                    State::Assignment(_) => "value",
                    State::Value => "comma",
                }
            }
        }

        let mut args = FuncArgs::new();
        let mut state = State::Start;

        loop {
            self.skip_whitespace();

            /*
            let token = self.next();
            match token {
                Some(ExprIdent(ident)) => match state {
                    State::Start => {
                        state = State::Identifier(Spanned {
                            v: Ident(ident.to_string()),
                            span: self.span(),
                        });
                    }
                    State::Identifier(prev) => {
                        self.expected(state.expected(), token);
                        args.add_pos(prev.map(|id| Expression::Ident(id)));
                        state = State::Identifier(Spanned {
                            v: Ident(ident.to_string()),
                            span: self.span(),
                        });
                    }
                    State::Assignment(key) => {
                        let span = Span::merge(key.span, self.span());
                        args.add_key(Spanned::new(KeyArg {
                            key,
                            value: Spanned {
                                v: Expression::Ident(Ident(ident.to_string())),
                                span: self.span(),
                            },
                        }, span));
                        state = State::Value;
                    }
                    State::Value => {
                        self.expected(state.expected(), token);
                        state = State::Identifier(Spanned {
                            v: Ident(ident.to_string()),
                            span: self.span(),
                        });
                    }
                }

                // Handle expressions.
                Some(Expr(_)) | Some(LeftParen) | Some(LeftBrace) => {
                    let expr = match token.unwrap() {
                        Expr(e) => e,
                        LeftParen => self.parse_tuple(),
                        LeftBrace => self.parse_object(),
                        _ => unreachable!(),
                    }
                }

                // Handle commas after values.
                Some(Comma) => match state {
                    State::Identifier(ident) => {
                        args.add_pos(ident.map(|id| Expression::Ident(id)));
                        state = State::Start;
                    }
                    State::Value => state = State::Start,
                    _ => self.expected(state.expected(), token),
                }

                // Handle the end of the function header.
                Some(RightBracket) => {
                    match state {
                        State::Identifier(ident) => {
                            args.add_pos(ident.map(|id| Expression::Ident(id)));
                        }
                        State::Assignment(_) => {
                            self.expected(state.expected(), token);
                        }
                        _ => {}
                    }

                    break;
                }
            }
            */
        }

        args
    }

    fn handle_expr(&mut self, expr: Spanned<Expression>) {

    }

    fn parse_tuple(&mut self) -> Spanned<Tuple> {
        unimplemented!()
    }

    fn parse_object(&mut self) -> Spanned<Object> {
        unimplemented!()
    }

    fn skip_whitespace(&mut self) {
        self.eat_until(|t| match t {
            Whitespace(_) | LineComment(_) | BlockComment(_) => false,
            _ => true,
        }, false)
    }

    fn eat_until<F>(&mut self, mut f: F, eat_match: bool)
    where F: FnMut(Token<'s>) -> bool {
        while let Some(token) = self.tokens.peek() {
            if f(token.v) {
                if eat_match {
                    self.next();
                }
                break;
            }

            self.next();
        }
    }

    fn next(&mut self) -> Option<Token<'s>> {
        self.tokens.next().map(|spanned| {
            self.color_token(&spanned.v, spanned.span);
            self.span = spanned.span;
            spanned.v
        })
    }

    fn span(&self) -> Span {
        self.span
    }

    fn position(&self) -> Position {
        self.span.end
    }

    fn unexpected(&mut self, found: Token) {
        self.errors.push(Spanned {
            v: format!("unexpected {}", name(found)),
            span: self.span(),
        });
    }

    fn expected(&mut self, thing: &str, found: Option<Token>) {
        let message = if let Some(found) = found {
            format!("expected {}, found {}", thing, name(found))
        } else {
            format!("expected {}", thing)
        };

        self.errors.push(Spanned {
            v: message,
            span: self.span(),
        });
    }

    fn color_token(&mut self, token: &Token<'s>, span: Span) {
        let colored = match token {
            LineComment(_) | BlockComment(_) => Some(ColorToken::Comment),
            StarSlash => Some(ColorToken::Invalid),
            LeftBracket | RightBracket => Some(ColorToken::Bracket),
            LeftParen | RightParen     => Some(ColorToken::Paren),
            LeftBrace | RightBrace     => Some(ColorToken::Brace),
            Colon  => Some(ColorToken::Colon),
            Comma  => Some(ColorToken::Comma),
            Equals => Some(ColorToken::Equals),
            ExprIdent(_)  =>  Some(ColorToken::ExprIdent),
            ExprStr(_) => Some(ColorToken::ExprStr),
            ExprNumber(_) => Some(ColorToken::ExprNumber),
            ExprSize(_)   => Some(ColorToken::ExprSize),
            ExprBool(_)   => Some(ColorToken::ExprBool),
            _ => None,
        };

        if let Some(color) = colored {
            self.colored.push(Spanned { v: color, span });
        }
    }

    fn color_span(&mut self, color: ColorToken, span: Span, replace_last: bool) {
        let token = Spanned { v: color, span };

        if replace_last {
            if let Some(last) = self.colored.last_mut() {
                *last = token;
                return;
            }
        }

        self.colored.push(token);
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
