//! Tokenization and parsing of source code into syntax trees.

use std::collections::HashMap;
use std::str::CharIndices;

use smallvec::SmallVec;
use unicode_xid::UnicodeXID;

use crate::func::{Function, Scope};
use crate::syntax::*;
use crate::size::Size;


/// Builds an iterator over the tokens of the source code.
#[inline]
pub fn tokenize(src: &str) -> Tokens {
    Tokens::new(src)
}

/// An iterator over the tokens of source code.
#[derive(Debug, Clone)]
pub struct Tokens<'s> {
    src: &'s str,
    chars: PeekableChars<'s>,
    state: TokensState,
    stack: SmallVec<[TokensState; 1]>,
}

/// The state the tokenizer is in.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum TokensState {
    /// The base state if there is nothing special we are in.
    Body,
    /// Inside a function header. Here colons and equal signs get parsed
    /// as distinct tokens rather than text.
    Function,
    /// We expect either the end of the function or the beginning of the body.
    MaybeBody,
}

impl<'s> Tokens<'s> {
    /// Create a new token stream from source code.
    fn new(src: &'s str) -> Tokens<'s> {
        Tokens {
            src,
            chars: PeekableChars::new(src),
            state: TokensState::Body,
            stack: SmallVec::new(),
        }
    }

    /// Advance the iterator by one step.
    fn advance(&mut self) {
        self.chars.next();
    }

    /// Switch to the given state.
    fn switch(&mut self, state: TokensState) {
        self.stack.push(self.state);
        self.state = state;
    }

    /// Go back to the top-of-stack state.
    fn unswitch(&mut self) {
         self.state = self.stack.pop().unwrap_or(TokensState::Body);
    }

    /// Advance and return the given token.
    fn consumed(&mut self, token: Token<'s>) -> Token<'s> {
        self.advance();
        token
    }

    /// Returns a word containing the string bounded by the given indices.
    fn text(&self, start: usize, end: usize) -> Token<'s> {
        Token::Text(&self.src[start .. end])
    }
}

impl<'s> Iterator for Tokens<'s> {
    type Item = Token<'s>;

    /// Advance the iterator, return the next token or nothing.
    fn next(&mut self) -> Option<Token<'s>> {
        use TokensState as TU;

        // Go to the body state if the function has a body or return to the top-of-stack state.
        if self.state == TU::MaybeBody {
            if self.chars.peek()?.1 == '[' {
                self.state = TU::Body;
                return Some(self.consumed(Token::LeftBracket));
            } else {
                self.unswitch();
            }
        }

        // Take the next char and peek at the one behind.
        let (next_pos, next) = self.chars.next()?;
        let afterwards = self.chars.peek().map(|p| p.1);

        Some(match next {
            // Functions
            '[' => {
                self.switch(TU::Function);
                Token::LeftBracket
            },
            ']' => {
                if self.state == TU::Function {
                    self.state = TU::MaybeBody;
                } else {
                    self.unswitch();
                }
                Token::RightBracket
            },

            // Line comment
            '/' if afterwards == Some('/') => {
                let mut end = self.chars.next().unwrap();
                let start = end.0 + end.1.len_utf8();

                while let Some((index, c)) = self.chars.peek() {
                    if is_newline_char(c) {
                        break;
                    }
                    self.advance();
                    end = (index, c);
                }

                let end = end.0 + end.1.len_utf8();
                Token::LineComment(&self.src[start .. end])
            },

            // Block comment
            '/' if afterwards == Some('*') => {
                let mut end = self.chars.next().unwrap();
                let start = end.0 + end.1.len_utf8();

                let mut nested = 0;
                while let Some((index, c)) = self.chars.next() {
                    let after = self.chars.peek().map(|p| p.1);
                    match (c, after) {
                        ('*', Some('/')) if nested == 0 => { self.advance(); break },
                        ('/', Some('*')) => { self.advance(); nested += 1 },
                        ('*', Some('/')) => { self.advance(); nested -= 1 },
                        _ => {},
                    }
                    end = (index, c);
                }

                let end = end.0 + end.1.len_utf8();
                Token::BlockComment(&self.src[start .. end])
            },

            // Unexpected end of block comment
            '*' if afterwards == Some('/') => self.consumed(Token::StarSlash),

            // Whitespace
            ' ' | '\t' => {
                while let Some((_, c)) = self.chars.peek() {
                    match c {
                        ' ' | '\t' => self.advance(),
                        _ => break,
                    }
                }
                Token::Space
            }

            // Newlines
            '\r' if afterwards == Some('\n') => self.consumed(Token::Newline),
            c if is_newline_char(c) => Token::Newline,

            // Star/Underscore/Backtick in bodies
            '*' if self.state == TU::Body => Token::Star,
            '_' if self.state == TU::Body => Token::Underscore,
            '`' if self.state == TU::Body => Token::Backtick,

            // Context sensitive operators in headers
            ':' if self.state == TU::Function => Token::Colon,
            '=' if self.state == TU::Function => Token::Equals,
            ',' if self.state == TU::Function => Token::Comma,

            // A string value.
            '"' if self.state == TU::Function => {
                // Find out when the word ends.
                let mut escaped = false;
                let mut end = (next_pos, next);

                while let Some((index, c)) = self.chars.next() {
                    if c == '"' && !escaped {
                        break;
                    }

                    escaped = c == '\\';
                    end = (index, c);
                }

                let end_pos = end.0 + end.1.len_utf8();
                Token::Quoted(&self.src[next_pos + 1 .. end_pos])
            }

            // Escaping
            '\\' => {
                if let Some((index, c)) = self.chars.peek() {
                    let escapable = match c {
                        '[' | ']' | '\\' | '*' | '_' | '`' | ':' | '=' | '/' => true,
                        _ => false,
                    };

                    if escapable {
                        self.advance();
                        return Some(self.text(index, index + c.len_utf8()));
                    }
                }

                Token::Text("\\")
            },

            // Normal text
            _ => {
                // Find out when the word ends.
                let mut end = (next_pos, next);
                while let Some((index, c)) = self.chars.peek() {
                    let second = self.chars.peek_second().map(|p| p.1);

                    // Whether the next token is still from the text or not.
                    let continues = match c {
                        '[' | ']' | '\\' => false,
                        '*' | '_' | '`' if self.state == TU::Body => false,
                        ':' | '=' | ',' | '"' if self.state == TU::Function => false,

                        '/' => second != Some('/') && second != Some('*'),
                        '*' => second != Some('/'),

                        ' ' | '\t' => false,
                        c if is_newline_char(c) => false,

                        _ => true,
                    };

                    if !continues {
                        break;
                    }

                    end = (index, c);
                    self.advance();
                }

                let end_pos = end.0 + end.1.len_utf8();
                self.text(next_pos, end_pos)
            },
        })
    }
}

/// Whether this character is a newline (or starts one).
fn is_newline_char(character: char) -> bool {
    match character {
        '\n' | '\r' | '\u{000c}' | '\u{0085}' | '\u{2028}' | '\u{2029}' => true,
        _ => false,
    }
}

/// A (index, char) iterator with double lookahead.
#[derive(Debug, Clone)]
struct PeekableChars<'s> {
    offset: usize,
    string: &'s str,
    chars: CharIndices<'s>,
    peek1: Option<Option<(usize, char)>>,
    peek2: Option<Option<(usize, char)>>,
}

impl<'s> PeekableChars<'s> {
    /// Create a new iterator from a string.
    fn new(string: &'s str) -> PeekableChars<'s> {
        PeekableChars {
            offset: 0,
            string,
            chars: string.char_indices(),
            peek1: None,
            peek2: None,
        }
    }

    /// Peek at the next element.
    fn peek(&mut self) -> Option<(usize, char)> {
        match self.peek1 {
            Some(peeked) => peeked,
            None => {
                let next = self.next_inner();
                self.peek1 = Some(next);
                next
            }
        }
    }

    /// Peek at the element after the next element.
    fn peek_second(&mut self) -> Option<(usize, char)> {
        match self.peek2 {
            Some(peeked) => peeked,
            None => {
                self.peek();
                let next = self.next_inner();
                self.peek2 = Some(next);
                next
            }
        }
    }

    /// Return the next value of the inner iterator mapped with the offset.
    fn next_inner(&mut self) -> Option<(usize, char)> {
        self.chars.next().map(|(i, c)| (i + self.offset, c))
    }

    /// The index of the first character of the next token in the source string.
    fn current_index(&mut self) -> Option<usize> {
        self.peek().map(|p| p.0)
    }

    /// Go to a new position in the underlying string.
    fn goto(&mut self, index: usize) {
        self.offset = index;
        self.chars = self.string[index..].char_indices();
        self.peek1 = None;
        self.peek2 = None;
    }
}

impl Iterator for PeekableChars<'_> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<(usize, char)> {
        match self.peek1.take() {
            Some(value) => {
                self.peek1 = self.peek2.take();
                value
            },
            None => self.next_inner(),
        }
    }
}

//------------------------------------------------------------------------------------------------//

/// Parses source code into a syntax tree given a context.
#[inline]
pub fn parse(src: &str, ctx: ParseContext) -> ParseResult<SyntaxTree> {
    Parser::new(src, ctx).parse()
}

/// The context for parsing.
#[derive(Debug, Copy, Clone)]
pub struct ParseContext<'a> {
    /// The scope containing function definitions.
    pub scope: &'a Scope,
}

/// Transforms token streams to syntax trees.
#[derive(Debug)]
struct Parser<'s> {
    src: &'s str,
    tokens: PeekableTokens<'s>,
    state: ParserState,
    ctx: ParseContext<'s>,
    tree: SyntaxTree,
}

/// The state the parser is in.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ParserState {
    /// The base state of the parser.
    Body,
    /// We saw one newline already.
    FirstNewline,
    /// We wrote a newline.
    WroteNewline,
}

impl<'s> Parser<'s> {
    /// Create a new parser from the source and the context.
    fn new(src: &'s str, ctx: ParseContext<'s>) -> Parser<'s> {
        Parser {
            src,
            tokens: PeekableTokens::new(tokenize(src)),
            state: ParserState::Body,
            ctx,
            tree: SyntaxTree::new(),
        }
    }

    /// Parse the source into an abstract syntax tree.
    fn parse(mut self) -> ParseResult<SyntaxTree> {
        // Loop through all the tokens.
        while self.tokens.peek().is_some() {
            self.parse_white()?;
            self.parse_body_part()?;
        }

        Ok(self.tree)
    }

    /// Parse the next part of the body.
    fn parse_body_part(&mut self) -> ParseResult<()> {
        if let Some(token) = self.tokens.peek() {
            match token {
                // Functions
                Token::LeftBracket => self.parse_func()?,
                Token::RightBracket => return Err(ParseError::new("unexpected closing bracket")),

                // Modifiers
                Token::Underscore => self.append_consumed(Node::ToggleItalics),
                Token::Star => self.append_consumed(Node::ToggleBold),
                Token::Backtick => self.append_consumed(Node::ToggleMonospace),

                // Normal text
                Token::Text(word) => self.append_consumed(Node::Text(word.to_owned())),

                Token::Colon | Token::Equals => panic!("bad token for body: {:?}", token),

                // The rest is handled elsewhere or should not happen, because `Tokens` does not
                // yield colons or equals in the body, but their text equivalents instead.
                _ => panic!("unexpected token: {:?}", token),
            }
        }
        Ok(())
    }

    /// Parse a complete function from the current position.
    fn parse_func(&mut self) -> ParseResult<()> {
        // This should only be called if a left bracket was seen.
        assert!(self.tokens.next() == Some(Token::LeftBracket));

        let header = self.parse_func_header()?;
        let body = self.parse_func_body(&header)?;

        // Finally this function is parsed to the end.
        self.append(Node::Func(FuncCall {
            header,
            body,
        }));

        Ok(self.switch(ParserState::Body))
    }

    /// Parse a function header.
    fn parse_func_header(&mut self) -> ParseResult<FuncHeader> {
        // The next token should be the name of the function.
        self.skip_white();
        let name = match self.tokens.next() {
            Some(Token::Text(word)) => {
                if is_identifier(word) {
                    Ok(word.to_owned())
                } else {
                    Err(ParseError::new(format!("invalid identifier: '{}'", word)))
                }
            },
            _ => Err(ParseError::new("expected identifier")),
        }?;

        let mut header = FuncHeader {
            name,
            args: vec![],
            kwargs: HashMap::new(),
        };

        self.skip_white();

        // Check for arguments
        match self.tokens.next() {
            Some(Token::RightBracket) => {},
            Some(Token::Colon) => {
                let (args, kwargs) = self.parse_func_args()?;
                header.args = args;
                header.kwargs = kwargs;
            },
            _ => return Err(ParseError::new("expected function arguments or closing bracket")),
        }

        // Store the header information of the function invocation.
        Ok(header)
    }

    /// Parse the arguments to a function.
    fn parse_func_args(&mut self) -> ParseResult<(Vec<Expression>, HashMap<String, Expression>)> {
        let mut args = vec![];
        let kwargs = HashMap::new();

        let mut comma = false;
        loop {
            self.skip_white();

            match self.tokens.peek() {
                Some(Token::Text(_)) | Some(Token::Quoted(_)) if !comma => {
                    args.push(self.parse_expression()?);
                    comma = true;
                },

                Some(Token::Comma) if comma => {
                    self.advance();
                    comma = false
                },
                Some(Token::RightBracket) => {
                    self.advance();
                    break
                },

                _ if comma => return Err(ParseError::new("expected comma or closing bracket")),
                _ => return Err(ParseError::new("expected closing bracket")),
            }
        }

        Ok((args, kwargs))
    }

    /// Parse an expression.
    fn parse_expression(&mut self) -> ParseResult<Expression> {
        Ok(match self.tokens.next() {
            Some(Token::Quoted(text)) => Expression::Str(text.to_owned()),
            Some(Token::Text(text)) => {
                if let Ok(b) = text.parse::<bool>() {
                    Expression::Bool(b)
                } else if let Ok(num) = text.parse::<f64>() {
                    Expression::Number(num)
                } else if let Ok(size) = text.parse::<Size>() {
                    Expression::Size(size)
                } else {
                    Expression::Ident(text.to_owned())
                }
            },
            _ => return Err(ParseError::new("expected expression")),
        })
    }

    /// Parse the body of a function.
    fn parse_func_body(&mut self, header: &FuncHeader) -> ParseResult<Box<dyn Function>> {
        // Whether the function has a body.
        let has_body = self.tokens.peek() == Some(Token::LeftBracket);
        if has_body {
            self.advance();
        }

        // Now we want to parse this function dynamically.
        let parser = self.ctx.scope.get_parser(&header.name)
            .ok_or_else(|| ParseError::new(format!("unknown function: '{}'", &header.name)))?;

        // Do the parsing dependent on whether the function has a body.
        Ok(if has_body {
            // Find out the string which makes the body of this function.
            let (start, end) = self.tokens.current_index().and_then(|index| {
                find_closing_bracket(&self.src[index..])
                    .map(|end| (index, index + end))
            }).ok_or_else(|| ParseError::new("expected closing bracket"))?;

            // Parse the body.
            let body_string = &self.src[start .. end];
            let body = parser(&header, Some(body_string), self.ctx)?;

            // Skip to the end of the function in the token stream.
            self.tokens.goto(end);

            // Now the body should be closed.
            assert!(self.tokens.next() == Some(Token::RightBracket));

            body
        } else {
            parser(&header, None, self.ctx)?
        })
    }

    /// Parse whitespace (as long as there is any) and skip over comments.
    fn parse_white(&mut self) -> ParseResult<()> {
        while let Some(token) = self.tokens.peek() {
            match self.state {
                ParserState::FirstNewline => match token {
                    Token::Newline => {
                        self.append_consumed(Node::Newline);
                        self.switch(ParserState::WroteNewline);
                    },
                    Token::Space => self.append_space_consumed(),
                    _ => {
                        self.append_space();
                        self.switch(ParserState::Body);
                    },
                },
                ParserState::WroteNewline => match token {
                    Token::Newline | Token::Space => self.append_space_consumed(),
                    _ => self.switch(ParserState::Body),
                },
                ParserState::Body => match token {
                    // Whitespace
                    Token::Space => self.append_space_consumed(),
                    Token::Newline => {
                        self.advance();
                        self.switch(ParserState::FirstNewline);
                    },

                    // Comments
                    Token::LineComment(_) | Token::BlockComment(_) => self.advance(),
                    Token::StarSlash => {
                        return Err(ParseError::new("unexpected end of block comment"));
                    },

                    // Anything else skips out of the function.
                    _ => break,
                }
            }
        }

        Ok(())
    }

    /// Skip over whitespace and comments.
    fn skip_white(&mut self) {
        while let Some(token) = self.tokens.peek() {
            match token {
                Token::Space | Token::Newline
                | Token::LineComment(_) | Token::BlockComment(_) => self.advance(),
                _ => break,
            }
        }
    }

    /// Advance the iterator by one step.
    fn advance(&mut self) {
        self.tokens.next();
    }

    /// Switch the state.
    fn switch(&mut self, state: ParserState) {
        self.state = state;
    }

    /// Append a node to the tree.
    fn append(&mut self, node: Node) {
        self.tree.nodes.push(node);
    }

    /// Append a space if there is not one already.
    fn append_space(&mut self) {
        if self.tree.nodes.last() != Some(&Node::Space) {
            self.append(Node::Space);
        }
    }

    /// Advance and return the given node.
    fn append_consumed(&mut self, node: Node) {
        self.advance();
        self.append(node);
    }

    /// Advance and append a space if there is not one already.
    fn append_space_consumed(&mut self) {
        self.advance();
        self.append_space();
    }
}

/// Find the index of the first unbalanced and unescaped closing bracket.
fn find_closing_bracket(src: &str) -> Option<usize> {
    let mut parens = 0;
    let mut escaped = false;
    for (index, c) in src.char_indices() {
        match c {
            '\\' => {
                escaped = !escaped;
                continue;
            },
            ']' if !escaped && parens == 0 => return Some(index),
            '[' if !escaped => parens += 1,
            ']' if !escaped => parens -= 1,
            _ => {},
        }
        escaped = false;
    }
    None
}

/// A peekable iterator for tokens which allows access to the original iterator inside this module
/// (which is needed by the parser).
#[derive(Debug, Clone)]
struct PeekableTokens<'s> {
    tokens: Tokens<'s>,
    peeked: Option<Option<Token<'s>>>,
}

impl<'s> PeekableTokens<'s> {
    /// Create a new iterator from a string.
    fn new(tokens: Tokens<'s>) -> PeekableTokens<'s> {
        PeekableTokens {
            tokens,
            peeked: None,
        }
    }

    /// Peek at the next element.
    fn peek(&mut self) -> Option<Token<'s>> {
        let iter = &mut self.tokens;
        *self.peeked.get_or_insert_with(|| iter.next())
    }

    /// The index of the first character of the next token in the source string.
    fn current_index(&mut self) -> Option<usize> {
        self.tokens.chars.current_index()
    }

    /// Go to a new position in the underlying string.
    fn goto(&mut self, index: usize) {
        self.tokens.chars.goto(index);
        self.peeked = None;
    }
}

impl<'s> Iterator for PeekableTokens<'s> {
    type Item = Token<'s>;

    fn next(&mut self) -> Option<Token<'s>> {
        match self.peeked.take() {
            Some(value) => value,
            None => self.tokens.next(),
        }
    }
}

/// Whether this word is a valid unicode identifier.
fn is_identifier(string: &str) -> bool {
    let mut chars = string.chars();

    match chars.next() {
        Some(c) if !UnicodeXID::is_xid_start(c) => return false,
        None => return false,
        _ => (),
    }

    while let Some(c) = chars.next() {
        if !UnicodeXID::is_xid_continue(c) {
            return false;
        }
    }

    true
}

//------------------------------------------------------------------------------------------------//

/// The error type for parsing.
pub struct ParseError(String);

/// The result type for parsing.
pub type ParseResult<T> = Result<T, ParseError>;

impl ParseError {
    /// Create a new parse error with a message.
    pub fn new<S: Into<String>>(message: S) -> ParseError {
        ParseError(message.into())
    }
}

error_type! {
    err: ParseError,
    show: f => f.write_str(&err.0),
}


#[cfg(test)]
mod token_tests {
    use super::*;
    use Token::{Space as S, Newline as N, LeftBracket as L, RightBracket as R,
                Colon as C, Equals as E, Quoted as Q, Underscore as TU, Star as TS,
                Backtick as TB, Text as T, LineComment as LC, BlockComment as BC,
                StarSlash as SS};

    /// Test if the source code tokenizes to the tokens.
    fn test(src: &str, tokens: Vec<Token>) {
        assert_eq!(Tokens::new(src).collect::<Vec<_>>(), tokens);
    }

    /// Tokenizes the basic building blocks.
    #[test]
    fn tokenize_base() {
        test("", vec![]);
        test("Hallo", vec![T("Hallo")]);
        test("[", vec![L]);
        test("]", vec![R]);
        test("*", vec![TS]);
        test("_", vec![TU]);
        test("`", vec![TB]);
        test("\n", vec![N]);
    }

    /// This test looks if LF- and CRLF-style newlines get both identified correctly.
    #[test]
    fn tokenize_whitespace_newlines() {
        test(" \t", vec![S]);
        test("First line\r\nSecond line\nThird line\n",
             vec![T("First"), S, T("line"), N, T("Second"), S, T("line"), N,
                  T("Third"), S, T("line"), N]);
        test("Hello \n ", vec![T("Hello"), S, N, S]);
        test("Dense\nTimes", vec![T("Dense"), N, T("Times")]);
    }

    /// Tests if escaping with backslash works as it should.
    #[test]
    fn tokenize_escape() {
        test(r"\[", vec![T("[")]);
        test(r"\]", vec![T("]")]);
        test(r"\**", vec![T("*"), TS]);
        test(r"\*", vec![T("*")]);
        test(r"\__", vec![T("_"), TU]);
        test(r"\_", vec![T("_")]);
        test(r"\hello", vec![T("\\"), T("hello")]);
    }

    /// Tests if escaped strings work.
    #[test]
    fn tokenize_quoted() {
        test(r#"[align: "hello\"world"]"#, vec![L, T("align"), C, S, Q(r#"hello\"world"#), R]);
    }

    /// Tokenizes some more realistic examples.
    #[test]
    fn tokenize_examples() {
        test(r"
            [function][
                Test [italic][example]!
            ]
        ", vec![
            N, S, L, T("function"), R, L, N, S, T("Test"), S, L, T("italic"), R, L,
            T("example"), R, T("!"), N, S, R, N, S
        ]);

        test(r"
            [page: size=A4]
            [font: size=12pt]

            Das ist ein Beispielsatz mit *fetter* Schrift.
        ", vec![
            N, S, L, T("page"), C, S, T("size"), E, T("A4"), R, N, S,
            L, T("font"), C, S, T("size"), E, T("12pt"), R, N, N, S,
            T("Das"), S, T("ist"), S, T("ein"), S, T("Beispielsatz"), S, T("mit"), S,
            TS, T("fetter"), TS, S, T("Schrift."), N, S
        ]);
    }

    /// This test checks whether the colon and equals symbols get parsed correctly depending on the
    /// context: Either in a function header or in a body.
    #[test]
    fn tokenize_symbols_context() {
        test("[func: key=value][Answer: 7]",
             vec![L, T("func"), C, S, T("key"), E, T("value"), R, L,
                  T("Answer:"), S, T("7"), R]);
        test("[[n: k=v]:x][:[=]]:=",
             vec![L, L, T("n"), C, S, T("k"), E, T("v"), R, C, T("x"), R,
                  L, T(":"), L, E, R, R, T(":=")]);
        test("[hi: k=[func][body] v=1][hello]",
            vec![L, T("hi"), C, S, T("k"), E, L, T("func"), R, L, T("body"), R, S,
                 T("v"), E, T("1"), R, L, T("hello"), R]);
        test("[func: __key__=value]",
             vec![L, T("func"), C, S, T("__key__"), E, T("value"), R]);
        test("The /*[*/ answer: 7.",
            vec![T("The"), S, BC("["), S, T("answer:"), S, T("7.")]);
    }

    /// Test if block and line comments get tokenized as expected.
    #[test]
    fn tokenize_comments() {
        test("These // Line comments.",
            vec![T("These"), S, LC(" Line comments.")]);
        test("This /* is */ a comment.",
            vec![T("This"), S, BC(" is "), S, T("a"), S, T("comment.")]);
        test("[Head/*of*/][Body]", vec![L, T("Head"), BC("of"), R, L, T("Body"), R]);
        test("/* Hey */ */", vec![BC(" Hey "), S, SS]);
        test("Hey\n// Yoo /*\n*/", vec![T("Hey"), N, LC(" Yoo /*"), N, SS]);
        test("/* My /* line // */ comment */", vec![BC(" My /* line // */ comment ")])
    }

    /// This test has a special look at the underscore syntax.
    #[test]
    fn tokenize_underscores() {
        test("he_llo_world_ __ Now this_ is_ special!",
             vec![T("he"), TU, T("llo"), TU, T("world"), TU, S, TU, TU, S, T("Now"), S,
                  T("this"), TU, S, T("is"), TU, S, T("special!")]);
    }

    /// This test is for checking if non-ASCII characters get parsed correctly.
    #[test]
    fn tokenize_unicode() {
        test("[document][Hello 🌍!]",
             vec![L, T("document"), R, L, T("Hello"), S, T("🌍!"), R]);
        test("[f]⺐.", vec![L, T("f"), R, T("⺐.")]);
    }
}


#[cfg(test)]
mod parse_tests {
    use super::*;
    use crate::func::{Function, Scope};
    use crate::layout::{LayoutContext, LayoutResult, Layout};
    use Node::{Space as S, Newline as N, Func as F};
    use funcs::*;

    /// Two test functions, one which parses it's body as another syntax tree and another one which
    /// does not expect a body.
    mod funcs {
        use super::*;

        /// A testing function which just parses it's body into a syntax tree.
        #[derive(Debug, PartialEq)]
        pub struct TreeFn(pub SyntaxTree);

        impl Function for TreeFn {
            fn parse(_: &FuncHeader, body: Option<&str>, ctx: ParseContext)
                -> ParseResult<Self> where Self: Sized {
                if let Some(src) = body {
                    parse(src, ctx).map(|tree| TreeFn(tree))
                } else {
                    Err(ParseError::new("expected body for tree fn"))
                }
            }

            fn layout(&self, _: LayoutContext) -> LayoutResult<Option<Layout>> { Ok(None) }
        }

        /// A testing function without a body.
        #[derive(Debug, PartialEq)]
        pub struct BodylessFn;

        impl Function for BodylessFn {
            fn parse(_: &FuncHeader, body: Option<&str>, _: ParseContext)
                -> ParseResult<Self> where Self: Sized {
                if body.is_none() {
                    Ok(BodylessFn)
                } else {
                    Err(ParseError::new("unexpected body for bodyless fn"))
                }
            }

            fn layout(&self, _: LayoutContext) -> LayoutResult<Option<Layout>> { Ok(None) }
        }
    }

    /// Test if the source code parses into the syntax tree.
    fn test(src: &str, tree: SyntaxTree) {
        let ctx = ParseContext { scope: &Scope::new() };
        assert_eq!(parse(src, ctx).unwrap(), tree);
    }

    /// Test with a scope containing function definitions.
    fn test_scoped(scope: &Scope, src: &str, tree: SyntaxTree) {
        let ctx = ParseContext { scope };
        assert_eq!(parse(src, ctx).unwrap(), tree);
    }

    /// Test if the source parses into the error.
    fn test_err(src: &str, err: &str) {
        let ctx = ParseContext { scope: &Scope::new() };
        assert_eq!(parse(src, ctx).unwrap_err().to_string(), err);
    }

    /// Test with a scope if the source parses into the error.
    fn test_err_scoped(scope: &Scope, src: &str, err: &str) {
        let ctx = ParseContext { scope };
        assert_eq!(parse(src, ctx).unwrap_err().to_string(), err);
    }

    /// Create a text node.
    #[allow(non_snake_case)]
    fn T(s: &str) -> Node { Node::Text(s.to_owned()) }

    /// Shortcut macro to create a syntax tree. Is `vec`-like and the elements are the nodes.
    macro_rules! tree {
        ($($x:expr),*) => (
            SyntaxTree { nodes: vec![$($x),*] }
        );
        ($($x:expr,)*) => (tree![$($x),*])
    }

    /// Shortcut macro to create a function.
    macro_rules! func {
        (name => $name:expr, body => None $(,)*) => {
            func!(@$name, Box::new(BodylessFn))
        };
        (name => $name:expr, body => $tree:expr $(,)*) => {
            func!(@$name, Box::new(TreeFn($tree)))
        };
        (@$name:expr, $body:expr) => {
            FuncCall {
                header: FuncHeader {
                    name: $name.to_string(),
                    args: vec![],
                    kwargs: HashMap::new(),
                },
                body: $body,
            }
        }
    }

    /// Parse the basic cases.
    #[test]
    fn parse_base() {
        test("", tree! []);
        test("Hello World!", tree! [ T("Hello"), S, T("World!") ]);
    }

    /// Test whether newlines generate the correct whitespace.
    #[test]
    fn parse_newlines_whitespace() {
        test("Hello\nWorld", tree! [ T("Hello"), S, T("World") ]);
        test("Hello \n World", tree! [ T("Hello"), S, T("World") ]);
        test("Hello\n\nWorld", tree! [ T("Hello"), N, T("World") ]);
        test("Hello \n\nWorld", tree! [ T("Hello"), S, N, T("World") ]);
        test("Hello\n\n  World", tree! [ T("Hello"), N, S, T("World") ]);
        test("Hello \n \n \n  World", tree! [ T("Hello"), S, N, S, T("World") ]);
        test("Hello\n \n\n  World", tree! [ T("Hello"), S, N, S, T("World") ]);
    }

    /// Parse things dealing with functions.
    #[test]
    fn parse_functions() {
        let mut scope = Scope::new();
        scope.add::<BodylessFn>("test");
        scope.add::<BodylessFn>("end");
        scope.add::<TreeFn>("modifier");
        scope.add::<TreeFn>("func");

        test_scoped(&scope,"[test]", tree! [ F(func! { name => "test", body => None }) ]);
        test_scoped(&scope,"[ test]", tree! [ F(func! { name => "test", body => None }) ]);
        test_scoped(&scope, "This is an [modifier][example] of a function invocation.", tree! [
            T("This"), S, T("is"), S, T("an"), S,
            F(func! { name => "modifier", body => tree! [ T("example") ] }), S,
            T("of"), S, T("a"), S, T("function"), S, T("invocation.")
        ]);
        test_scoped(&scope, "[func][Hello][modifier][Here][end]",  tree! [
            F(func! { name => "func", body => tree! [ T("Hello") ] }),
            F(func! { name => "modifier", body => tree! [ T("Here") ] }),
            F(func! { name => "end", body => None }),
        ]);
        test_scoped(&scope, "[func][]", tree! [ F(func! { name => "func", body => tree! [] }) ]);
        test_scoped(&scope, "[modifier][[func][call]] outside", tree! [
            F(func! {
                name => "modifier",
                body => tree! [ F(func! { name => "func", body => tree! [ T("call") ] }) ],
            }),
            S, T("outside")
        ]);

    }

    /// Parse functions with arguments.
    #[test]
    fn parse_function_args() {
        use Expression::{Number as N, Size as Z, Bool as B};

        #[allow(non_snake_case)]
        fn S(string: &str) -> Expression { Expression::Str(string.to_owned()) }
        #[allow(non_snake_case)]
        fn I(string: &str) -> Expression { Expression::Ident(string.to_owned()) }

        fn func(name: &str, args: Vec<Expression>) -> SyntaxTree {
            tree! [ F(FuncCall {
                header: FuncHeader {
                    name: name.to_string(),
                    args,
                    kwargs: HashMap::new(),
                },
                body: Box::new(BodylessFn)
            }) ]
        }

        let mut scope = Scope::new();
        scope.add::<BodylessFn>("align");

        test_scoped(&scope, "[align: left]", func("align", vec![I("left")]));
        test_scoped(&scope, "[align: left,right]", func("align", vec![I("left"), I("right")]));
        test_scoped(&scope, "[align: left, right]", func("align", vec![I("left"), I("right")]));
        test_scoped(&scope, "[align: \"hello\"]", func("align", vec![S("hello")]));
        test_scoped(&scope, r#"[align: "hello\"world"]"#, func("align", vec![S(r#"hello\"world"#)]));
        test_scoped(&scope, "[align: 12]", func("align", vec![N(12.0)]));
        test_scoped(&scope, "[align: 17.53pt]", func("align", vec![Z(Size::pt(17.53))]));
        test_scoped(&scope, "[align: 2.4in]", func("align", vec![Z(Size::inches(2.4))]));
        test_scoped(&scope, "[align: true, 10mm, left, \"hi, there\"]",
            func("align", vec![B(true), Z(Size::mm(10.0)), I("left"), S("hi, there")]));
    }

    /// Parse comments (line and block).
    #[test]
    fn parse_comments() {
        let mut scope = Scope::new();
        scope.add::<BodylessFn>("test");
        scope.add::<TreeFn>("func");

        test_scoped(&scope, "Text\n// Comment\n More text",
            tree! [ T("Text"), S, T("More"), S, T("text") ]);
        test_scoped(&scope, "[test/*world*/]",
            tree! [ F(func! { name => "test", body => None }) ]);
        test_scoped(&scope, "[test/*]*/]",
            tree! [ F(func! { name => "test", body => None }) ]);
    }

    /// Test if escaped, but unbalanced parens are correctly parsed.
    #[test]
    fn parse_unbalanced_body_parens() {
        let mut scope = Scope::new();
        scope.add::<TreeFn>("code");

        test_scoped(&scope, r"My [code][Close \]] end", tree! [
            T("My"), S, F(func! {
                name => "code",
                body => tree! [ T("Close"), S, T("]") ]
            }), S, T("end")
        ]);
        test_scoped(&scope, r"My [code][\[ Open] end", tree! [
            T("My"), S, F(func! {
                name => "code",
                body => tree! [ T("["), S, T("Open") ]
            }), S, T("end")
        ]);
        test_scoped(&scope, r"My [code][Open \]  and  \[ close]end", tree! [
            T("My"), S, F(func! {
                name => "code",
                body => tree! [ T("Open"), S, T("]"), S, T("and"), S, T("["), S, T("close") ]
            }), T("end")
        ]);
    }

    /// Tests if the parser handles non-ASCII stuff correctly.
    #[test]
    fn parse_unicode() {
        let mut scope = Scope::new();
        scope.add::<BodylessFn>("func");
        scope.add::<TreeFn>("bold");

        test_scoped(&scope, "[func] ⺐.", tree! [
            F(func! {
                name => "func",
                body => None,
            }),
            S, T("⺐.")
        ]);
        test_scoped(&scope, "[bold][Hello 🌍!]", tree! [
            F(func! {
                name => "bold",
                body => tree! [ T("Hello"), S, T("🌍!") ],
            })
        ]);
    }

    /// Tests whether errors get reported correctly.
    #[test]
    fn parse_errors() {
        let mut scope = Scope::new();
        scope.add::<TreeFn>("hello");

        test_err("No functions here]", "unexpected closing bracket");
        test_err_scoped(&scope, "[hello][world", "expected closing bracket");
        test_err("[hello world", "expected function arguments or closing bracket");
        test_err("[ no-name][Why?]", "invalid identifier: 'no-name'");
        test_err("Hello */", "unexpected end of block comment");
    }
}
