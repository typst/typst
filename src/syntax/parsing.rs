//! Parsing of token streams into syntax trees.

use unicode_xid::UnicodeXID;

use crate::func::{Function, Scope};
use crate::size::Size;
use super::*;

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

/// Transforms token streams into syntax trees.
#[derive(Debug)]
struct Parser<'s> {
    src: &'s str,
    tokens: PeekableTokens<'s>,
    ctx: ParseContext<'s>,
    tree: SyntaxTree,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum NewlineState {
    /// No newline yet.
    Zero,
    /// We saw one newline with the given span already and are
    /// looking for another.
    One(Span),
    /// We saw at least two newlines and wrote one, thus not
    /// writing another one for more newlines.
    TwoOrMore,
}

impl<'s> Parser<'s> {
    /// Create a new parser from the source code and the context.
    fn new(src: &'s str, ctx: ParseContext<'s>) -> Parser<'s> {
        Parser {
            src,
            tokens: PeekableTokens::new(tokenize(src)),
            ctx,
            tree: SyntaxTree::new(),
        }
    }

    /// Parse the source into a syntax tree.
    fn parse(mut self) -> ParseResult<SyntaxTree> {
        while self.tokens.peek().is_some() {
            self.parse_white()?;
            self.parse_body_part()?;
        }

        Ok(self.tree)
    }

    /// Parse the next part of the body.
    fn parse_body_part(&mut self) -> ParseResult<()> {
        use Token::*;

        if let Some(token) = self.tokens.peek() {
            match token.val {
                // Functions.
                LeftBracket => self.parse_func()?,
                RightBracket => return Err(ParseError::new("unexpected closing bracket")),

                // Modifiers.
                Underscore => self.append_consumed(Node::ToggleItalics, token.span),
                Star => self.append_consumed(Node::ToggleBold, token.span),
                Backtick => self.append_consumed(Node::ToggleMonospace, token.span),

                // Normal text.
                Text(word) => self.append_consumed(Node::Text(word.to_owned()), token.span),

                // The rest is handled elsewhere or should not happen, because `Tokens` does not
                // yield these in a body.
                Space | Newline | LineComment(_) | BlockComment(_) |
                Colon | Equals | Comma | Quoted(_) | StarSlash
                    => panic!("parse_body_part: unexpected token: {:?}", token),
            }
        }

        Ok(())
    }

    /// Parse a complete function from the current position.
    fn parse_func(&mut self) -> ParseResult<()> {
        // This should only be called if a left bracket was seen.
        let token = self.tokens.next().expect("parse_func: expected token");
        assert!(token.val == Token::LeftBracket);

        let mut span = token.span;

        let header = self.parse_func_header()?;
        let body = self.parse_func_body(&header)?;

        span.end = self.tokens.string_index();

        // Finally this function is parsed to the end.
        self.append(Node::Func(FuncCall { header, body }), span);

        Ok(())
    }

    /// Parse a function header.
    fn parse_func_header(&mut self) -> ParseResult<FuncHeader> {
        self.skip_white();

        let name = match self.tokens.next().map(|token| token.val) {
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
            kwargs: vec![],
        };

        self.skip_white();

        // Check for arguments
        match self.tokens.next().map(|token| token.val) {
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
    fn parse_func_args(&mut self) -> ParseResult<(Vec<Expression>, Vec<(String, Expression)>)> {
        let mut args = Vec::new();
        let kwargs = Vec::new();

        let mut comma = false;
        loop {
            self.skip_white();

            match self.tokens.peek().map(|token| token.val) {
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
        Ok(match self.tokens.next().map(|token| token.val) {
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
        let has_body = self.tokens.peek().map(|token| token.val) == Some(Token::LeftBracket);
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
            let start = self.tokens.string_index();
            let end = find_closing_bracket(&self.src[start..])
                .map(|end| start + end)
                .ok_or_else(|| ParseError::new("expected closing bracket"))?;

            // Parse the body.
            let body_string = &self.src[start..end];
            let body = parser(&header, Some(body_string), self.ctx)?;

            // Skip to the end of the function in the token stream.
            self.tokens.set_string_index(end);

            // Now the body should be closed.
            let token = self.tokens.next().expect("parse_func_body: expected token");
            assert!(token.val == Token::RightBracket);

            body
        } else {
            parser(&header, None, self.ctx)?
        })
    }

    /// Parse whitespace (as long as there is any) and skip over comments.
    fn parse_white(&mut self) -> ParseResult<()> {
        let mut state = NewlineState::Zero;

        while let Some(token) = self.tokens.peek() {
            match token.val {
                Token::Space => {
                    self.advance();
                    match state {
                        NewlineState::Zero | NewlineState::TwoOrMore => {
                            self.append_space(token.span);
                        }
                        _ => {}
                    }
                }

                Token::Newline => {
                    self.advance();
                    match state {
                        NewlineState::Zero => state = NewlineState::One(token.span),
                        NewlineState::One(mut span) => {
                            span.expand(token.span);
                            state = NewlineState::TwoOrMore;
                            self.append(Node::Newline, span);
                        },
                        NewlineState::TwoOrMore => self.append_space(token.span),
                    }
                }

                _ => {
                    if let NewlineState::One(span) = state {
                        self.append_space(span);
                    }

                    state = NewlineState::Zero;
                    match token.val {
                        Token::LineComment(_) | Token::BlockComment(_) => self.advance(),
                        Token::StarSlash => err!("unexpected end of block comment"),
                        _ => break,
                    }
                }
            }
        }

        Ok(())
    }

    /// Skip over whitespace and comments.
    fn skip_white(&mut self) {
        while let Some(token) = self.tokens.peek() {
            match token.val {
                Token::Space | Token::Newline |
                Token::LineComment(_) | Token::BlockComment(_) => self.advance(),
                _ => break,
            }
        }
    }

    /// Advance the iterator by one step.
    fn advance(&mut self) {
        self.tokens.next();
    }

    /// Append a node to the tree.
    fn append(&mut self, node: Node, span: Span) {
        self.tree.nodes.push(Spanned::new(node, span));
    }

    /// Append a space, merging with a previous space if there is one.
    fn append_space(&mut self, span: Span) {
        match self.tree.nodes.last_mut() {
            Some(ref mut node) if node.val == Node::Space => node.span.expand(span),
            _ => self.append(Node::Space, span),
        }
    }

    /// Advance and return the given node.
    fn append_consumed(&mut self, node: Node, span: Span) {
        self.advance();
        self.append(node, span);
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
    peeked: Option<Option<Spanned<Token<'s>>>>,
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
    fn peek(&mut self) -> Option<Spanned<Token<'s>>> {
        let iter = &mut self.tokens;
        *self.peeked.get_or_insert_with(|| iter.next())
    }

    fn string_index(&mut self) -> usize {
        self.tokens.string_index()
    }

    fn set_string_index(&mut self, index: usize) {
        self.tokens.set_string_index(index);
        self.peeked = None;
    }
}

impl<'s> Iterator for PeekableTokens<'s> {
    type Item = Spanned<Token<'s>>;

    fn next(&mut self) -> Option<Self::Item> {
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
        Some(c) if c != '.' && !UnicodeXID::is_xid_start(c) => return false,
        None => return false,
        _ => (),
    }

    while let Some(c) = chars.next() {
        if c != '.' && !UnicodeXID::is_xid_continue(c) {
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
    use crate::func::{CommandList, Function, Scope};
    use crate::layout::{LayoutContext, LayoutResult};
    use funcs::*;
    use Node::{Func as F, Newline as N, Space as S};

    /// Two test functions, one which parses it's body as another syntax tree
    /// and another one which does not expect a body.
    mod funcs {
        use super::*;

        /// A testing function which just parses it's body into a syntax tree.
        #[derive(Debug)]
        pub struct TreeFn(pub SyntaxTree);

        function! {
            data: TreeFn,

            parse(_args, body, ctx) { Ok(TreeFn(parse!(required: body, ctx))) }
            layout(_, _) { Ok(commands![]) }
        }

        impl PartialEq for TreeFn {
            fn eq(&self, other: &TreeFn) -> bool { tree_equal(&self.0, &other.0) }
        }

        /// A testing function without a body.
        #[derive(Debug)]
        pub struct BodylessFn;

        function! {
            data: BodylessFn,

            parse(_args, body, _ctx) { parse!(forbidden: body); Ok(BodylessFn) }
            layout(_, _) { Ok(commands![]) }
        }

        impl PartialEq for BodylessFn {
            fn eq(&self, _: &BodylessFn) -> bool { true }
        }
    }

    fn tree_equal(a: &SyntaxTree, b: &SyntaxTree) -> bool {
        a.nodes.iter().zip(&b.nodes).all(|(x, y)| x.val == y.val)
    }

    /// Test if the source code parses into the syntax tree.
    fn test(src: &str, tree: SyntaxTree) {
        let ctx = ParseContext {
            scope: &Scope::new(),
        };
        assert!(tree_equal(&parse(src, ctx).unwrap(), &tree));
    }

    /// Test with a scope containing function definitions.
    fn test_scoped(scope: &Scope, src: &str, tree: SyntaxTree) {
        let ctx = ParseContext { scope };
        assert!(tree_equal(&parse(src, ctx).unwrap(), &tree));
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
    /// are the nodes without spans.
    macro_rules! tree {
        ($($x:expr),*) => ({
            #[allow(unused_mut)] let mut nodes = vec![];
            $(
                nodes.push(Spanned::new($x, Span::new(0, 0)));
            )*
            SyntaxTree { nodes }
        });
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
                    kwargs: vec![],
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
        test("Hello\n \n\n  World", tree! [ T("Hello"), N, S, T("World") ]);
        test("Hello\n \nWorld", tree! [ T("Hello"), N, T("World") ]);
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
                        kwargs: vec![],
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

    /// Tests whether spans get calculated correctly.
    #[test]
    #[rustfmt::skip]
    fn parse_spans() {
        let mut scope = Scope::new();
        scope.add::<TreeFn>("hello");

        let parse = |string| {
            parse(string, ParseContext { scope: &scope }).unwrap().nodes
        };

        let tree = parse("hello world");
        assert_eq!(tree[0].span.pair(), (0, 5));
        assert_eq!(tree[2].span.pair(), (6, 11));

        let tree = parse("p1\n \np2");
        assert_eq!(tree[1].span.pair(), (2, 5));

        let tree = parse("func [hello: pos, other][body _🌍_]");
        assert_eq!(tree[0].span.pair(), (0, 4));
        assert_eq!(tree[1].span.pair(), (4, 5));
        assert_eq!(tree[2].span.pair(), (5, 37));

        let func = if let Node::Func(f) = &tree[2].val { f } else { panic!() };
        let body = &func.body.downcast::<TreeFn>().unwrap().0.nodes;
        assert_eq!(body[0].span.pair(), (0, 4));
        assert_eq!(body[1].span.pair(), (4, 5));
        assert_eq!(body[2].span.pair(), (5, 6));
        assert_eq!(body[3].span.pair(), (6, 10));
        assert_eq!(body[4].span.pair(), (10, 11));
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
