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
    color_tokens: Vec<Spanned<ColorToken>>,
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
            color_tokens: vec![],
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
                Underscore => self.add_consumed(Node::ToggleItalics, token.span),
                Star => self.add_consumed(Node::ToggleBolder, token.span),
                Backtick => self.add_consumed(Node::ToggleMonospace, token.span),

                // Normal text.
                Text(word) => self.add_consumed(Node::Text(word.to_owned()), token.span),

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

        self.add_color_token(ColorToken::Bracket, token.span);

        let mut span = token.span;
        let name = self.parse_func_name()?;

        // Check for arguments
        let args = match self.tokens.next() {
            Some(Spanned { v: Token::RightBracket, span }) => {
                self.add_color_token(ColorToken::Bracket, span);
                FuncArgs::new()
            },
            Some(Spanned { v: Token::Colon, span }) => {
                self.add_color_token(ColorToken::Colon, span);
                self.parse_func_args()?
            }
            _ => error!("expected arguments or closing bracket"),
        };

        span.end = self.tokens.get_position();
        let (func, body_span) = self.parse_func_call(name, args)?;

        if let Some(body_span) = body_span {
            span.expand(body_span);
        }

        // Finally this function is parsed to the end.
        self.add(Node::Func(func), span);

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

        self.add_color_token(ColorToken::FuncName, name.span);
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

            match self.tokens.next() {
                Some(Spanned { v: Token::Comma, span }) => {
                    self.add_color_token(ColorToken::Comma, span);
                }
                Some(Spanned { v: Token::RightBracket, span }) => {
                    self.add_color_token(ColorToken::Bracket, span);
                    break;
                }
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

                Some(match self.tokens.peek() {
                    Some(Spanned { v: Token::Equals, span }) => {
                        self.advance();
                        self.skip_white();

                        let name = Ident::new(name.to_string())?;
                        let key = Spanned::new(name, token.span);

                        self.add_color_token(ColorToken::KeyArg, key.span);
                        self.add_color_token(ColorToken::Equals, span);

                        let next = self.tokens.next()
                            .ok_or_else(|| error!(@"expected expression"))?;

                        let value = Self::parse_expression(next)?;

                        self.add_expr_token(&value);

                        let span = Span::merge(key.span, value.span);
                        let arg = KeyArg { key, value };

                        DynArg::Key(Spanned::new(arg, span))
                    }

                    _ => {
                        let expr = Self::parse_expression(token)?;
                        self.add_expr_token(&expr);
                        DynArg::Pos(expr)
                    }
                })
            }

            Token::Quoted(_) => {
                self.advance();
                self.skip_white();

                self.add_color_token(ColorToken::ExprStr, token.span);

                Some(DynArg::Pos(Self::parse_expression(token)?))
            }

            _ => None,
        })
    }

    /// Parse a function call.
    fn parse_func_call(&mut self, name: Spanned<Ident>, args: FuncArgs)
    -> ParseResult<(FuncCall, Option<Span>)> {
        // Now we want to parse this function dynamically.
        let parser = self
            .ctx
            .scope
            .get_parser(&name.v.0)
            .ok_or_else(|| error!(@"unknown function: `{}`", &name.v))?;

        let has_body = self.tokens.peek().map(Spanned::value) == Some(Token::LeftBracket);

        // Do the parsing dependent on whether the function has a body.
        Ok(if has_body {
            self.advance();

            // Find out the string which makes the body of this function.
            let start_index = self.tokens.string_index();
            let mut start_pos = self.tokens.get_position();
            start_pos.column -= 1;

            let (mut end_index, mut end_pos) =
                find_closing_bracket(&self.src[start_index..])
                    .ok_or_else(|| error!(@"expected closing bracket"))?;

            end_index += start_index;
            end_pos.column += 1;

            let span = Span::new(start_pos, end_pos);

            // Parse the body.
            let body_string = &self.src[start_index..end_index];
            let body = parser(args, Some(body_string), self.ctx)?;

            // Skip to the end of the function in the token stream.
            self.tokens.set_string_index(end_index);

            // Now the body should be closed.
            let token = self.tokens.next().expect("parse_func_body: expected token");
            assert!(token.v == Token::RightBracket);

            (FuncCall(body), Some(span))
        } else {
            (FuncCall(parser(args, None, self.ctx)?), None)
        })
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
                            self.add_space(token.span);
                        }
                        _ => {}
                    }
                }

                Token::Newline => {
                    self.advance();
                    match state {
                        NewlineState::Zero => state = NewlineState::One(token.span),
                        NewlineState::One(span) => {
                            self.add(Node::Newline, Span::merge(span, token.span));
                            state = NewlineState::TwoOrMore;
                        },
                        NewlineState::TwoOrMore => self.add_space(token.span),
                    }
                }

                _ => {
                    if let NewlineState::One(span) = state {
                        self.add_space(Span::new(span.start, token.span.start));
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
    fn add(&mut self, node: Node, span: Span) {
        self.tree.nodes.push(Spanned::new(node, span));
    }

    /// Append a space, merging with a previous space if there is one.
    fn add_space(&mut self, span: Span) {
        match self.tree.nodes.last_mut() {
            Some(ref mut node) if node.v == Node::Space => node.span.expand(span),
            _ => self.add(Node::Space, span),
        }
    }

    /// Advance and return the given node.
    fn add_consumed(&mut self, node: Node, span: Span) {
        self.advance();
        self.add(node, span);
    }

    /// Add a color token to the list.
    fn add_color_token(&mut self, token: ColorToken, span: Span) {
        self.color_tokens.push(Spanned::new(token, span));
    }

    /// Add a color token for an expression.
    fn add_expr_token(&mut self, expr: &Spanned<Expression>) {
        let kind = match expr.v {
            Expression::Bool(_) => ColorToken::ExprBool,
            Expression::Ident(_) => ColorToken::ExprIdent,
            Expression::Num(_) => ColorToken::ExprNumber,
            Expression::Size(_) => ColorToken::ExprSize,
            Expression::Str(_) => ColorToken::ExprStr,
        };

        self.add_color_token(kind, expr.span);
    }
}

/// Find the index of the first unbalanced and unescaped closing bracket.
fn find_closing_bracket(src: &str) -> Option<(usize, Position)> {
    let mut parens = 0;
    let mut escaped = false;
    let mut line = 1;
    let mut line_start_index = 0;

    for (index, c) in src.char_indices() {
        match c {
            '\\' => {
                escaped = !escaped;
                continue;
            }
            c if is_newline_char(c) => {
                line += 1;
                line_start_index = index + c.len_utf8();
            }
            ']' if !escaped && parens == 0 => {
                let position = Position {
                    line,
                    column: index - line_start_index,
                };

                return Some((index, position))
            }
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

    fn get_position(&self) -> Position {
        match self.peeked {
            Some(Some(peeked)) => peeked.span.start,
            _ => self.tokens.get_position(),
        }
    }

    fn string_index(&self) -> usize {
        match self.peeked {
            Some(Some(peeked)) => peeked.span.start.line,
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

    fn test_color(scope: &Scope, src: &str, tokens: Vec<(usize, usize, ColorToken)>) {
        let ctx = ParseContext { scope };
        let tree = parse(src, ctx).unwrap();
        // assert_eq!(tree.tokens,
        //     tokens.into_iter()
        //         .map(|(s, e, t)| Spanned::new(t, Span::new(s, e)))
        //         .collect::<Vec<_>>()
        // );
    }

    /// Create a text node.
    fn T(s: &str) -> Node {
        Node::Text(s.to_owned())
    }

    fn zerospan<T>(val: T) -> Spanned<T> {
        Spanned::new(val, Span::new(Position::new(0, 0), Position::new(0, 0)))
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
        fn test_span(src: &str, correct: Vec<(usize, usize, usize, usize)>) {
            let mut scope = Scope::new();
            scope.add::<TreeFn>("hello");
            let tree = parse(src, ParseContext { scope: &scope }).unwrap();
            let spans = tree.nodes.into_iter()
                .map(|node| {
                    let Span { start, end } = node.span;
                    (start.line, start.column, end.line, end.column)
                })
                .collect::<Vec<_>>();

            assert_eq!(spans, correct);
        }

        test_span("hello world", vec![(1, 0, 1, 5), (1, 5, 1, 6), (1, 6, 1, 11)]);
        test_span("p1\n \np2", vec![(1, 0, 1, 2), (1, 2, 2, 2), (3, 0, 3, 2)]);

        let src = "func\n [hello: pos, other][body\r\n _🌍_\n]";
        test_span(src, vec![
            (1, 0, 1, 4),
            (1, 4, 2, 1),
            (2, 1, 4, 1)
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
        test_err("[hello world", "expected arguments or closing bracket");
        test_err("[ no^name][Why?]", "invalid identifier: `no^name`");
        test_err("Hello */", "unexpected end of block comment");
    }

    /// Tests syntax highlighting.
    #[test]
    #[rustfmt::skip]
    fn test_highlighting() {
        use ColorToken::{Bracket as B, FuncName as F, *};

        let mut scope = Scope::new();
        scope.add::<BodylessFn>("func");
        scope.add::<TreeFn>("tree");

        test_color(&scope, "[func]", vec![(0, 1, B), (1, 5, F), (5, 6, B)]);
        test_color(&scope, "[func: 12pt]", vec![
            (0, 1, B), (1, 5, F), (5, 6, Colon), (7, 11, ExprSize), (11, 12, B)
        ]);
        test_color(&scope, "[func: x=25.3, y=\"hi\"]", vec![
            (0, 1, B), (1, 5, F), (5, 6, Colon),
            (7, 8, KeyArg), (8, 9, Equals), (9, 13, ExprNumber),
            (13, 14, Comma),
            (15, 16, KeyArg), (16, 17, Equals), (17, 21, ExprStr),
            (21, 22, B),
        ]);

        test_color(&scope, "Hello [tree][With [func: 3]]", vec![
            (6, 7, B), (7, 11, F), (11, 12, B),
            (12, 13, B), (18, 19, B)
        ]);
    }
}
