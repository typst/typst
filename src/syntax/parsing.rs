//! Parsing of token streams into syntax trees.

use crate::func::Scope;
use crate::size::Size;
use super::*;


/// The result type for parsing.
pub type ParseResult<T> = crate::TypesetResult<T>;

/// Parses source code into a syntax tree given a context.
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
            match token.v {
                // Functions.
                LeftBracket => self.parse_func()?,
                RightBracket => error!("unexpected closing bracket"),

                // Modifiers.
                Underscore => self.append_consumed(Node::ToggleItalics, token.span),
                Star => self.append_consumed(Node::ToggleBolder, token.span),
                Backtick => self.append_consumed(Node::ToggleMonospace, token.span),

                // Normal text.
                Text(word) => self.append_consumed(Node::Text(word.to_owned()), token.span),

                // The rest is handled elsewhere or should not happen, because
                // the tokenizer does not yield these in a body.
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
        assert!(token.v == Token::LeftBracket);

        let mut span = token.span;
        let name = self.parse_func_name()?;

        // Check for arguments
        let args = match self.tokens.next().map(Spanned::value) {
            Some(Token::RightBracket) => FuncArgs::new(),
            Some(Token::Colon) => self.parse_func_args()?,
            _ => error!("expected arguments or closing bracket"),
        };

        let func = self.parse_func_call(name, args)?;
        span.end = self.tokens.string_index();

        // Finally this function is parsed to the end.
        self.append(Node::Func(func), span);

        Ok(())
    }

    /// Parse a function header.
    fn parse_func_name(&mut self) -> ParseResult<Spanned<Ident>> {
        self.skip_white();

        let name = match self.tokens.next() {
            Some(Spanned { v: Token::Text(word), span }) => {
                let ident = Ident::new(word.to_string())?;
                Spanned::new(ident, span)
            }
            _ => error!("expected identifier"),
        };

        self.skip_white();

        Ok(name)
    }

    /// Parse the arguments to a function.
    fn parse_func_args(&mut self) -> ParseResult<FuncArgs> {
        let mut args = FuncArgs::new();

        loop {
            self.skip_white();

            match self.parse_func_arg()? {
                Some(DynArg::Pos(arg)) => args.add_pos(arg),
                Some(DynArg::Key(arg)) => args.add_key(arg),
                None => {},
            }

            match self.tokens.next().map(Spanned::value) {
                Some(Token::Comma) => {},
                Some(Token::RightBracket) => break,
                _ => error!("expected comma or closing bracket"),
            }
        }

        Ok(args)
    }

    /// Parse one argument to a function.
    fn parse_func_arg(&mut self) -> ParseResult<Option<DynArg>> {
        let token = match self.tokens.peek() {
            Some(token) => token,
            None => return Ok(None),
        };

        Ok(match token.v {
            Token::Text(name) => {
                self.advance();
                self.skip_white();

                Some(match self.tokens.peek().map(Spanned::value) {
                    Some(Token::Equals) => {
                        self.advance();
                        self.skip_white();

                        let name = Ident::new(name.to_string())?;
                        let key = Spanned::new(name, token.span);

                        let next = self.tokens.next()
                            .ok_or_else(|| error!(@"expected expression"))?;
                        let value = Self::parse_expression(next)?;

                        let span = Span::merge(key.span, value.span);
                        let arg = KeyArg { key, value };

                        DynArg::Key(Spanned::new(arg, span))
                    }

                    _ => DynArg::Pos(Self::parse_expression(token)?),
                })
            }

            Token::Quoted(_) => {
                self.advance();
                Some(DynArg::Pos(Self::parse_expression(token)?))
            }

            _ => None,
        })
    }

    /// Parse a function call.
    fn parse_func_call(&mut self, name: Spanned<Ident>, args: FuncArgs) -> ParseResult<FuncCall> {
        // Now we want to parse this function dynamically.
        let parser = self
            .ctx
            .scope
            .get_parser(&name.v.0)
            .ok_or_else(|| error!(@"unknown function: `{}`", &name.v))?;

        let has_body = self.tokens.peek().map(Spanned::value) == Some(Token::LeftBracket);

        // Do the parsing dependent on whether the function has a body.
        Ok(FuncCall(if has_body {
            self.advance();

            // Find out the string which makes the body of this function.
            let start = self.tokens.string_index();
            let end = find_closing_bracket(&self.src[start..])
                .map(|end| start + end)
                .ok_or_else(|| error!(@"expected closing bracket"))?;

            let span = Span::new(start - 1, end + 1);

            // Parse the body.
            let body_string = &self.src[start..end];
            let body = parser(args, Some(Spanned::new(body_string, span)), self.ctx)?;

            // Skip to the end of the function in the token stream.
            self.tokens.set_string_index(end);

            // Now the body should be closed.
            let token = self.tokens.next().expect("parse_func_body: expected token");
            assert!(token.v == Token::RightBracket);

            body
        } else {
            parser(args, None, self.ctx)?
        }))
    }

    /// Parse an expression.
    fn parse_expression(token: Spanned<Token>) -> ParseResult<Spanned<Expression>> {
        Ok(Spanned::new(match token.v {
            Token::Quoted(text) => Expression::Str(text.to_owned()),
            Token::Text(text) => {
                if let Ok(b) = text.parse::<bool>() {
                    Expression::Bool(b)
                } else if let Ok(num) = text.parse::<f64>() {
                    Expression::Num(num)
                } else if let Ok(size) = text.parse::<Size>() {
                    Expression::Size(size)
                } else {
                    // This loop does not actually loop, but is used for breaking.
                    loop {
                        if text.ends_with('%') {
                            if let Ok(percent) = text[.. text.len()-1].parse::<f64>() {
                                break Expression::Num(percent / 100.0);
                            }
                        }

                        break Expression::Ident(Ident::new(text.to_string())?);
                    }
                }
            }
            _ => error!("expected expression"),
        }, token.span))
    }

    /// Parse whitespace (as long as there is any) and skip over comments.
    fn parse_white(&mut self) -> ParseResult<()> {
        let mut state = NewlineState::Zero;

        while let Some(token) = self.tokens.peek() {
            match token.v {
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
                        NewlineState::One(span) => {
                            self.append(Node::Newline, Span::merge(span, token.span));
                            state = NewlineState::TwoOrMore;
                        },
                        NewlineState::TwoOrMore => self.append_space(token.span),
                    }
                }

                _ => {
                    if let NewlineState::One(span) = state {
                        self.append_space(Span::new(span.start, token.span.start));
                    }

                    state = NewlineState::Zero;
                    match token.v {
                        Token::LineComment(_) | Token::BlockComment(_) => self.advance(),
                        Token::StarSlash => error!("unexpected end of block comment"),
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
            match token.v {
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
            Some(ref mut node) if node.v == Node::Space => node.span.expand(span),
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
        match self.peeked {
            Some(Some(peeked)) => peeked.span.start,
            _ => self.tokens.string_index(),
        }
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


#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::func::{Commands, Scope};
    use crate::layout::{LayoutContext, LayoutResult};
    use crate::syntax::*;
    use Node::{Func as F, Newline as N, Space as S};

    function! {
        /// A testing function which just parses it's body into a syntax
        /// tree.
        #[derive(Debug)]
        pub struct TreeFn { pub tree: SyntaxTree }

        parse(args, body, ctx) {
            args.clear();
            TreeFn {
                tree: parse!(expected: body, ctx)
            }
        }

        layout() { vec![] }
    }

    impl PartialEq for TreeFn {
        fn eq(&self, other: &TreeFn) -> bool {
            assert_tree_equal(&self.tree, &other.tree);
            true
        }
    }

    function! {
        /// A testing function without a body.
        #[derive(Debug, Default, PartialEq)]
        pub struct BodylessFn(Vec<Expression>, Vec<(Ident, Expression)>);

        parse(args, body) {
            parse!(forbidden: body);
            BodylessFn(
                args.pos().map(Spanned::value).collect(),
                args.keys().map(|arg| (arg.v.key.v, arg.v.value.v)).collect(),
            )
        }

        layout() { vec![] }
    }

    mod args {
        use super::*;
        use super::Expression;
        pub use Expression::{Num as N, Size as Z, Bool as B};

        pub fn S(string: &str) -> Expression { Expression::Str(string.to_owned()) }
        pub fn I(string: &str) -> Expression {
            Expression::Ident(Ident::new(string.to_owned()).unwrap())
        }
    }

    /// Asserts that two syntax trees are equal except for all spans inside them.
    fn assert_tree_equal(a: &SyntaxTree, b: &SyntaxTree) {
        for (x, y) in a.nodes.iter().zip(&b.nodes) {
            if x.v != y.v {
                panic!("trees are not equal: ({:#?}) != ({:#?})", x.v, y.v);
            }
        }
    }

    /// Test if the source code parses into the syntax tree.
    fn test(src: &str, tree: SyntaxTree) {
        let ctx = ParseContext {
            scope: &Scope::new(),
        };
        assert_tree_equal(&parse(src, ctx).unwrap(), &tree);
    }

    /// Test with a scope containing function definitions.
    fn test_scoped(scope: &Scope, src: &str, tree: SyntaxTree) {
        let ctx = ParseContext { scope };
        assert_tree_equal(&parse(src, ctx).unwrap(), &tree);
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
    fn T(s: &str) -> Node {
        Node::Text(s.to_owned())
    }

    fn zerospan<T>(val: T) -> Spanned<T> {
        Spanned::new(val, Span::new(0, 0))
    }

    /// Shortcut macro to create a syntax tree. Is `vec`-like and the elements
    /// are the nodes without spans.
    macro_rules! tree {
        ($($x:expr),*) => ({
            #[allow(unused_mut)] let mut nodes = vec![];
            $(
                nodes.push(zerospan($x));
            )*
            SyntaxTree { nodes }
        });
        ($($x:expr,)*) => (tree![$($x),*])
    }

    /// Shortcut macro to create a function.
    macro_rules! func {
        () => (
            FuncCall(Box::new(BodylessFn(vec![], vec![])))
        );
        (body: $tree:expr $(,)*) => (
            FuncCall(Box::new(TreeFn { tree: $tree }))
        );
        (args: $pos:expr, $key:expr) => (
            FuncCall(Box::new(BodylessFn($pos, $key)))
        );
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

        test_scoped(&scope,"[test]", tree! [ F(func! {}) ]);
        test_scoped(&scope,"[ test]", tree! [ F(func! {}) ]);
        test_scoped(&scope, "This is an [modifier][example] of a function invocation.", tree! [
            T("This"), S, T("is"), S, T("an"), S,
            F(func! { body: tree! [ T("example") ] }), S,
            T("of"), S, T("a"), S, T("function"), S, T("invocation.")
        ]);
        test_scoped(&scope, "[func][Hello][modifier][Here][end]",  tree! [
            F(func! { body: tree! [ T("Hello") ] }),
            F(func! { body: tree! [ T("Here") ] }),
            F(func! {}),
        ]);
        test_scoped(&scope, "[func][]", tree! [ F(func! { body: tree! [] }) ]);
        test_scoped(&scope, "[modifier][[func][call]] outside", tree! [
            F(func! { body: tree! [ F(func! { body: tree! [ T("call") ] }) ] }), S, T("outside")
        ]);

    }

    /// Parse functions with arguments.
    #[test]
    #[rustfmt::skip]
    fn parse_function_args() {
        use args::*;

        fn func(
            pos: Vec<Expression>,
            key: Vec<(&str, Expression)>,
        ) -> SyntaxTree {
            let key = key.into_iter()
                .map(|s| (Ident::new(s.0.to_string()).unwrap(), s.1))
                .collect();

            tree! [ F(func!(args: pos, key)) ]
        }

        let mut scope = Scope::new();
        scope.add::<BodylessFn>("align");

        test_scoped(&scope, "[align: left]", func(vec![I("left")], vec![]));
        test_scoped(&scope, "[align: left,right]", func(vec![I("left"), I("right")], vec![]));
        test_scoped(&scope, "[align: left, right]", func(vec![I("left"), I("right")], vec![]));
        test_scoped(&scope, "[align: \"hello\"]", func(vec![S("hello")], vec![]));
        test_scoped(&scope, r#"[align: "hello\"world"]"#, func(vec![S(r#"hello\"world"#)], vec![]));
        test_scoped(&scope, "[align: 12]", func(vec![N(12.0)], vec![]));
        test_scoped(&scope, "[align: 17.53pt]", func(vec![Z(Size::pt(17.53))], vec![]));
        test_scoped(&scope, "[align: 2.4in]", func(vec![Z(Size::inches(2.4))], vec![]));
        test_scoped(&scope, "[align: true, 10mm, left, \"hi, there\"]",
            func(vec![B(true), Z(Size::mm(10.0)), I("left"), S("hi, there")], vec![]));

        test_scoped(&scope, "[align: right=true]", func(vec![], vec![("right", B(true))]));
        test_scoped(&scope, "[align: flow = horizontal]",
            func(vec![], vec![("flow", I("horizontal"))]));
        test_scoped(&scope, "[align: x=1cm, y=20mm]",
            func(vec![], vec![("x", Z(Size::cm(1.0))), ("y", Z(Size::mm(20.0)))]));
        test_scoped(&scope, "[align: x=5.14,a, \"b\", c=me,d=you]",
            func(vec![I("a"), S("b")], vec![("x", N(5.14)), ("c", I("me")), ("d", I("you"))]));
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
            tree! [ F(func! {}) ]);
        test_scoped(&scope, "[test/*]*/]",
            tree! [ F(func! {}) ]);
    }

    /// Test if escaped, but unbalanced parens are correctly parsed.
    #[test]
    #[rustfmt::skip]
    fn parse_unbalanced_body_parens() {
        let mut scope = Scope::new();
        scope.add::<TreeFn>("code");

        test_scoped(&scope, r"My [code][Close \]] end", tree! [
            T("My"), S, F(func! { body: tree! [ T("Close"), S, T("]") ] }), S, T("end")
        ]);
        test_scoped(&scope, r"My [code][\[ Open] end", tree! [
            T("My"), S, F(func! { body: tree! [ T("["), S, T("Open") ] }), S, T("end")
        ]);
        test_scoped(&scope, r"My [code][Open \]  and  \[ close]end", tree! [
            T("My"), S, F(func! { body:
                tree! [ T("Open"), S, T("]"), S, T("and"), S, T("["), S, T("close") ]
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

        test_scoped(&scope, "[func] ⺐.", tree! [ F(func! {}), S, T("⺐.") ]);
        test_scoped(&scope, "[bold][Hello 🌍!]", tree! [
            F(func! { body: tree! [ T("Hello"), S, T("🌍!") ] })
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

        let tree = parse("p1\n p2");
        assert_eq!(tree[1].span.pair(), (2, 4));

        let src = "func [hello: pos, other][body _🌍_]";
        let tree = parse(src);
        assert_eq!(tree[0].span.pair(), (0, 4));
        assert_eq!(tree[1].span.pair(), (4, 5));
        assert_eq!(tree[2].span.pair(), (5, 37));

        let func = if let Node::Func(f) = &tree[2].v { f } else { panic!() };
        let body = &func.0.downcast::<TreeFn>().unwrap().tree.nodes;
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
        test_err("[hello world", "expected arguments or closing bracket");
        test_err("[ no^name][Why?]", "invalid identifier: `no^name`");
        test_err("Hello */", "unexpected end of block comment");
    }
}
