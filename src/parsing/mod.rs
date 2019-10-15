//! Parsing of source code into token streams and syntax trees.

use std::collections::HashMap;

use unicode_xid::UnicodeXID;

use crate::func::{Function, Scope};
use crate::size::Size;
use crate::syntax::*;

mod tokens;

pub use tokens::{tokenize, Tokens};

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
        self.append(Node::Func(FuncCall { header, body }));

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
            }
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
            Some(Token::RightBracket) => {}
            Some(Token::Colon) => {
                let (args, kwargs) = self.parse_func_args()?;
                header.args = args;
                header.kwargs = kwargs;
            }
            _ => {
                return Err(ParseError::new(
                    "expected function arguments or closing bracket",
                ))
            }
        }

        // Store the header information of the function invocation.
        Ok(header)
    }

    /// Parse the arguments to a function.
    fn parse_func_args(&mut self) -> ParseResult<(Vec<Expression>, HashMap<String, Expression>)> {
        let mut args = Vec::new();
        let kwargs = HashMap::new();

        let mut comma = false;
        loop {
            self.skip_white();

            match self.tokens.peek() {
                Some(Token::Text(_)) | Some(Token::Quoted(_)) if !comma => {
                    args.push(self.parse_expression()?);
                    comma = true;
                }

                Some(Token::Comma) if comma => {
                    self.advance();
                    comma = false
                }
                Some(Token::RightBracket) => {
                    self.advance();
                    break;
                }

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
            }
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
        let parser = self
            .ctx
            .scope
            .get_parser(&header.name)
            .ok_or_else(|| ParseError::new(format!("unknown function: '{}'", &header.name)))?;

        // Do the parsing dependent on whether the function has a body.
        Ok(if has_body {
            // Find out the string which makes the body of this function.
            let (start, end) = self
                .tokens
                .current_index()
                .and_then(|index| {
                    find_closing_bracket(&self.src[index..]).map(|end| (index, index + end))
                })
                .ok_or_else(|| ParseError::new("expected closing bracket"))?;

            // Parse the body.
            let body_string = &self.src[start..end];
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
                    }
                    Token::Space => self.append_space_consumed(),
                    _ => {
                        self.append_space();
                        self.switch(ParserState::Body);
                    }
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
                    }

                    // Comments
                    Token::LineComment(_) | Token::BlockComment(_) => self.advance(),
                    Token::StarSlash => {
                        return Err(ParseError::new("unexpected end of block comment"));
                    }

                    // Anything else skips out of the function.
                    _ => break,
                },
            }
        }

        Ok(())
    }

    /// Skip over whitespace and comments.
    fn skip_white(&mut self) {
        while let Some(token) = self.tokens.peek() {
            match token {
                Token::Space | Token::Newline | Token::LineComment(_) | Token::BlockComment(_) => {
                    self.advance()
                }
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
            }
            ']' if !escaped && parens == 0 => return Some(index),
            '[' if !escaped => parens += 1,
            ']' if !escaped => parens -= 1,
            _ => {}
        }
        escaped = false;
    }
    None
}

/// A peekable iterator for tokens which allows access to the original iterator
/// inside this module (which is needed by the parser).
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
mod tests {
    use super::*;
    use crate::func::{FuncCommands, Function, Scope};
    use crate::layout::{LayoutContext, LayoutResult};
    use funcs::*;
    use Node::{Func as F, Newline as N, Space as S};

    /// Two test functions, one which parses it's body as another syntax tree
    /// and another one which does not expect a body.
    mod funcs {
        use super::*;

        /// A testing function which just parses it's body into a syntax tree.
        #[derive(Debug, PartialEq)]
        pub struct TreeFn(pub SyntaxTree);

        impl Function for TreeFn {
            fn parse(_: &FuncHeader, body: Option<&str>, ctx: ParseContext) -> ParseResult<Self>
            where Self: Sized {
                if let Some(src) = body {
                    parse(src, ctx).map(|tree| TreeFn(tree))
                } else {
                    Err(ParseError::new("expected body for tree fn"))
                }
            }

            fn layout(&self, _: LayoutContext) -> LayoutResult<FuncCommands> {
                Ok(FuncCommands::new())
            }
        }

        /// A testing function without a body.
        #[derive(Debug, PartialEq)]
        pub struct BodylessFn;

        impl Function for BodylessFn {
            fn parse(_: &FuncHeader, body: Option<&str>, _: ParseContext) -> ParseResult<Self>
            where Self: Sized {
                if body.is_none() {
                    Ok(BodylessFn)
                } else {
                    Err(ParseError::new("unexpected body for bodyless fn"))
                }
            }

            fn layout(&self, _: LayoutContext) -> LayoutResult<FuncCommands> {
                Ok(FuncCommands::new())
            }
        }
    }

    /// Test if the source code parses into the syntax tree.
    fn test(src: &str, tree: SyntaxTree) {
        let ctx = ParseContext {
            scope: &Scope::new(),
        };
        assert_eq!(parse(src, ctx).unwrap(), tree);
    }

    /// Test with a scope containing function definitions.
    fn test_scoped(scope: &Scope, src: &str, tree: SyntaxTree) {
        let ctx = ParseContext { scope };
        assert_eq!(parse(src, ctx).unwrap(), tree);
    }

    /// Test if the source parses into the error.
    fn test_err(src: &str, err: &str) {
        let ctx = ParseContext {
            scope: &Scope::new(),
        };
        assert_eq!(parse(src, ctx).unwrap_err().to_string(), err);
    }

    /// Test with a scope if the source parses into the error.
    fn test_err_scoped(scope: &Scope, src: &str, err: &str) {
        let ctx = ParseContext { scope };
        assert_eq!(parse(src, ctx).unwrap_err().to_string(), err);
    }

    /// Create a text node.
    #[allow(non_snake_case)]
    fn T(s: &str) -> Node {
        Node::Text(s.to_owned())
    }

    /// Shortcut macro to create a syntax tree. Is `vec`-like and the elements
    /// are the nodes.
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
    #[rustfmt::skip]
    fn parse_base() {
        test("", tree! []);
        test("Hello World!", tree! [ T("Hello"), S, T("World!") ]);
    }

    /// Test whether newlines generate the correct whitespace.
    #[test]
    #[rustfmt::skip]
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
    #[rustfmt::skip]
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
    #[rustfmt::skip]
    fn parse_function_args() {
        use Expression::{Number as N, Size as Z, Bool as B};

        #[allow(non_snake_case)]
        fn S(string: &str) -> Expression { Expression::Str(string.to_owned()) }
        #[allow(non_snake_case)]
        fn I(string: &str) -> Expression { Expression::Ident(string.to_owned()) }

        fn func(name: &str, args: Vec<Expression>) -> SyntaxTree {
            tree! [
                F(FuncCall {
                    header: FuncHeader {
                        name: name.to_string(),
                        args,
                        kwargs: HashMap::new(),
                    },
                    body: Box::new(BodylessFn)
                })
            ]
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
    #[rustfmt::skip]
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
    #[rustfmt::skip]
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
    #[rustfmt::skip]
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
    #[rustfmt::skip]
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
