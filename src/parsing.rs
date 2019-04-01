//! Tokenization and parsing of source code into syntax trees.

use std::fmt;
use std::iter::Peekable;
use std::mem::swap;
use unicode_segmentation::{UnicodeSegmentation, UWordBounds};
use crate::syntax::*;
use crate::utility::{Splinor, Spline, Splined, StrExt};


/// An iterator over the tokens of source code.
#[derive(Clone)]
pub struct Tokens<'s> {
    source: &'s str,
    words: Peekable<UWordBounds<'s>>,
    state: TokensState<'s>,
    stack: Vec<TokensState<'s>>,
}

impl fmt::Debug for Tokens<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Tokens")
            .field("source", &self.source)
            .field("words", &"Peekable<UWordBounds>")
            .field("state", &self.state)
            .field("stack", &self.stack)
            .finish()
    }
}

/// The state the tokenizer is in.
#[derive(Debug, Clone)]
enum TokensState<'s> {
    /// The base state if there is nothing special we are in.
    Body,
    /// Inside a function header. Here colons and equal signs get parsed
    /// as distinct tokens rather than text.
    Function,
    /// We expect either the end of the function or the beginning of the body.
    MaybeBody,
    /// We are inside one unicode word that consists of multiple tokens,
    /// because it contains double underscores.
    DoubleUnderscore(Spline<'s, Token<'s>>),
}

impl PartialEq for TokensState<'_> {
    fn eq(&self, other: &TokensState) -> bool {
        use TokensState as TS;

        match (self, other) {
            (TS::Body, TS::Body) => true,
            (TS::Function, TS::Function) => true,
            (TS::MaybeBody, TS::MaybeBody) => true,
            // They are not necessarily different, but we don't care
            _ => false,
        }
    }
}

impl<'s> Iterator for Tokens<'s> {
    type Item = Token<'s>;

    /// Advance the iterator, return the next token or nothing.
    fn next(&mut self) -> Option<Token<'s>> {
        use TokensState as TS;

        // Return the remaining words and double underscores.
        if let TS::DoubleUnderscore(splinor) = &mut self.state {
            loop {
                if let Some(splined) = splinor.next() {
                    return Some(match splined {
                        Splined::Value(word) if word != "" => Token::Word(word),
                        Splined::Splinor(s) => s,
                        _ => continue,
                    });
                } else {
                    self.unswitch();
                    break;
                }
            }
        }

        // Skip whitespace, but if at least one whitespace word existed,
        // remember that, because we return a space token.
        let mut whitespace = false;
        while let Some(word) = self.words.peek() {
            if !word.is_whitespace() {
                break;
            }
            whitespace = true;
            self.advance();
        }
        if whitespace {
            return Some(Token::Space);
        }

        // Function maybe has a body
        if self.state == TS::MaybeBody {
            match *self.words.peek()? {
                "[" => {
                    self.state = TS::Body;
                    return Some(self.consumed(Token::LeftBracket));
                },
                _ => self.unswitch(),
            }
        }

        // Now all special cases are handled and we can finally look at the
        // next words.
        let next = self.words.next()?;
        let afterwards = self.words.peek();

        Some(match next {
            // Special characters
            "[" => {
                self.switch(TS::Function);
                Token::LeftBracket
            },
            "]" => {
                if self.state == TS::Function {
                    self.state = TS::MaybeBody;
                }
                Token::RightBracket
            },
            "$" => Token::Dollar,
            "#" => Token::Hashtag,

            // Context sensitive operators
            ":" if self.state == TS::Function => Token::Colon,
            "=" if self.state == TS::Function => Token::Equals,

            // Double star/underscore
            "*" if afterwards == Some(&"*") => {
                self.consumed(Token::DoubleStar)
            },
            "__" => Token::DoubleUnderscore,

            // Newlines
            "\n" | "\r\n" => Token::Newline,

            // Escaping
            r"\" => {
                if let Some(next) = afterwards {
                    let escapable = match *next {
                        "[" | "]" | "$" | "#" | r"\" | ":" | "=" | "*" | "_" => true,
                        w if w.starts_with("__") => true,
                        _ => false,
                    };

                    if escapable {
                        let next = *next;
                        self.advance();
                        return Some(Token::Word(next));
                    }
                }

                Token::Word(r"\")
            },

            // Double underscores hidden in words.
            word if word.contains("__") => {
                let spline = word.spline("__", Token::DoubleUnderscore);
                self.switch(TS::DoubleUnderscore(spline));
                return self.next();
            },

            // Now it seems like it's just a normal word.
            word => Token::Word(word),
        })
    }
}

impl<'s> Tokens<'s> {
    /// Create a new token stream from text.
    #[inline]
    pub fn new(source: &'s str) -> Tokens<'s> {
        Tokens {
            source,
            words: source.split_word_bounds().peekable(),
            state: TokensState::Body,
            stack: vec![],
        }
    }

    /// Advance the iterator by one step.
    fn advance(&mut self) {
        self.words.next();
    }

    /// Switch to the given state.
    fn switch(&mut self, mut state: TokensState<'s>) {
        swap(&mut state, &mut self.state);
        self.stack.push(state);
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
}

/// Transforms token streams to syntax trees.
#[derive(Debug)]
pub struct Parser<'s, T> where T: Iterator<Item=Token<'s>> {
    tokens: Peekable<T>,
    state: ParserState,
    stack: Vec<Function<'s>>,
    tree: SyntaxTree<'s>,
}

/// The state the parser is in.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ParserState {
    /// The base state of the parser.
    Body,
    /// Inside a function header.
    Function,
}

impl<'s, T> Parser<'s, T> where T: Iterator<Item=Token<'s>> {
    /// Create a new parser from a type that emits results of tokens.
    pub(crate) fn new(tokens: T) -> Parser<'s, T> {
        Parser {
            tokens: tokens.peekable(),
            state: ParserState::Body,
            stack: vec![],
            tree: SyntaxTree::new(),
        }
    }

    /// Parse into an abstract syntax tree.
    pub(crate) fn parse(mut self) -> ParseResult<SyntaxTree<'s>> {
        use ParserState as PS;

        while let Some(token) = self.tokens.next() {
            // Comment
            if token == Token::Hashtag {
                self.skip_while(|&t| t != Token::Newline);
                self.advance();
            }

            match self.state {
                PS::Body => match token {
                    // Whitespace
                    Token::Space => self.append(Node::Space),
                    Token::Newline => {
                        self.append(Node::Newline);
                        if self.tokens.peek() != Some(&Token::Space) {
                            self.append(Node::Space);
                        }
                    },

                    // Words
                    Token::Word(word) => self.append(Node::Word(word)),

                    // Functions
                    Token::LeftBracket => self.switch(PS::Function),
                    Token::RightBracket => {
                        match self.stack.pop() {
                            Some(func) => self.append(Node::Func(func)),
                            None => return self.err("unexpected closing bracket"),
                        }
                    },

                    // Modifiers
                    Token::DoubleUnderscore => self.append(Node::ToggleItalics),
                    Token::DoubleStar => self.append(Node::ToggleBold),
                    Token::Dollar => self.append(Node::ToggleMath),

                    // Should not happen
                    Token::Colon | Token::Equals | Token::Hashtag => unreachable!(),
                },

                PS::Function => {
                    let name = match token {
                        Token::Word(word) if word.is_identifier() => word,
                        _ => return self.err("expected identifier"),
                    };

                    if self.tokens.next() != Some(Token::RightBracket) {
                        return self.err("expected closing bracket");
                    }

                    let mut func = Function {
                        name,
                        body: None,
                    };

                    // This function has a body.
                    if let Some(Token::LeftBracket) = self.tokens.peek() {
                        self.advance();
                        func.body = Some(SyntaxTree::new());
                        self.stack.push(func);
                    } else {
                        self.append(Node::Func(func));
                    }

                    self.switch(PS::Body);
                },
            }
        }

        if !self.stack.is_empty() {
            return self.err("expected closing bracket");
        }

        Ok(self.tree)
    }

    /// Advance the iterator by one step.
    fn advance(&mut self) {
        self.tokens.next();
    }

    /// Skip tokens until the condition is met.
    fn skip_while<F>(&mut self, f: F) where F: Fn(&Token) -> bool {
        while let Some(token) = self.tokens.peek() {
            if !f(token) {
                break;
            }
            self.advance();
        }
    }

    /// Switch the state.
    fn switch(&mut self, state: ParserState) {
        self.state = state;
    }

    /// Append a node to the top-of-stack function or the main tree itself.
    fn append(&mut self, node: Node<'s>) {
        let tree = match self.stack.last_mut() {
            Some(func) => func.body.as_mut().unwrap(),
            None => &mut self.tree,
        };

        tree.nodes.push(node);
    }

    /// Gives a parsing error with a message.
    fn err<R, S: Into<String>>(&self, message: S) -> ParseResult<R> {
        Err(ParseError { message: message.into() })
    }
}

/// The error type for parsing.
pub struct ParseError {
    message: String,
}

error_type! {
    err: ParseError,
    res: ParseResult,
    show: f => f.write_str(&err.message),
}


#[cfg(test)]
mod token_tests {
    use super::*;
    use Token::{Space as S, Newline as N, LeftBracket as L, RightBracket as R,
                Colon as C, Equals as E, DoubleUnderscore as DU, DoubleStar as DS,
                Dollar as D, Hashtag as H, Word as W};

    /// Test if the source code tokenizes to the tokens.
    fn test(src: &str, tokens: Vec<Token>) {
        assert_eq!(Tokens::new(src).collect::<Vec<_>>(), tokens);
    }

    /// Tokenizes the basic building blocks.
    #[test]
    fn tokenize_base() {
        test("", vec![]);
        test("Hallo", vec![W("Hallo")]);
        test("[", vec![L]);
        test("]", vec![R]);
        test("$", vec![D]);
        test("#", vec![H]);
        test("**", vec![DS]);
        test("__", vec![DU]);
        test("\n", vec![N]);
    }

    /// This test looks if LF- and CRLF-style newlines get both identified correctly
    #[test]
    fn tokenize_whitespace_newlines() {
        test(" \t", vec![S]);
        test("First line\r\nSecond line\nThird line\n",
             vec![W("First"), S, W("line"), N, W("Second"), S, W("line"), N,
                  W("Third"), S, W("line"), N]);
        test("Hello \n ", vec![W("Hello"), S, N, S]);
        test("Dense\nTimes", vec![W("Dense"), N, W("Times")]);
    }

    /// Tests if escaping with backslash works as it should.
    #[test]
    fn tokenize_escape() {
        test(r"\[", vec![W("[")]);
        test(r"\]", vec![W("]")]);
        test(r"\#", vec![W("#")]);
        test(r"\$", vec![W("$")]);
        test(r"\:", vec![W(":")]);
        test(r"\=", vec![W("=")]);
        test(r"\**", vec![W("*"), W("*")]);
        test(r"\*", vec![W("*")]);
        test(r"\__", vec![W("__")]);
        test(r"\_", vec![W("_")]);
        test(r"\hello", vec![W(r"\"), W("hello")]);
    }

    /// Tokenizes some more realistic examples.
    #[test]
    fn tokenize_examples() {
        test(r"
            [function][
                Test [italic][example]!
            ]
        ", vec![
            N, S, L, W("function"), R, L, N, S, W("Test"), S, L, W("italic"), R, L,
            W("example"), R, W("!"), N, S, R, N, S
        ]);

        test(r"
            [page: size=A4]
            [font: size=12pt]

            Das ist ein Beispielsatz mit **fetter** Schrift.
        ", vec![
            N, S, L, W("page"), C, S, W("size"), E, W("A4"), R, N, S,
            L, W("font"), C, S, W("size"), E, W("12pt"), R, N, N, S,
            W("Das"), S, W("ist"), S, W("ein"), S, W("Beispielsatz"), S, W("mit"), S,
            DS, W("fetter"), DS, S, W("Schrift"), W("."), N, S
        ]);
    }

    /// This test checks whether the colon and equals symbols get parsed correctly
    /// depending on the context: Either in a function header or in a body.
    #[test]
    fn tokenize_symbols_context() {
        test("[func: key=value][Answer: 7]",
             vec![L, W("func"), C, S, W("key"), E, W("value"), R, L,
                  W("Answer"), W(":"), S, W("7"), R]);
        test("[[n: k=v]:x][:[=]]:=",
             vec![L, L, W("n"), C, S, W("k"), E, W("v"), R, C, W("x"), R,
                  L, W(":"), L, E, R, R, W(":"), W("=")]);
        test("[func: __key__=value]",
             vec![L, W("func"), C, S, DU, W("key"), DU, E, W("value"), R]);
    }

    /// This test has a special look at the double underscore syntax, because
    /// per Unicode standard they are not separate words and thus harder to parse
    /// than the stars.
    #[test]
    fn tokenize_double_underscore() {
        test("he__llo__world_ _ __ Now this_ is__ special!",
             vec![W("he"), DU, W("llo"), DU, W("world_"), S, W("_"), S, DU, S, W("Now"), S,
                  W("this_"), S, W("is"), DU, S, W("special"), W("!")]);
    }

    /// This test is for checking if non-ASCII characters get parsed correctly.
    #[test]
    fn tokenize_unicode() {
        test("[document][Hello 🌍!]",
             vec![L, W("document"), R, L, W("Hello"), S, W("🌍"), W("!"), R]);
        test("[f]⺐.", vec![L, W("f"), R, W("⺐"), W(".")]);
    }
}

#[cfg(test)]
mod parse_tests {
    use super::*;
    use Node::{Space as S, Word as W, Newline as N, Func as F};

    /// Test if the source code parses into the syntax tree.
    fn test(src: &str, tree: SyntaxTree) {
        assert_eq!(Parser::new(Tokens::new(src)).parse().unwrap(), tree);
    }

    /// Test if the source parses into the error.
    fn test_err(src: &str, err: &str) {
        assert_eq!(Parser::new(Tokens::new(src)).parse().unwrap_err().message, err);
    }

    /// Short cut macro to create a syntax tree.
    /// Is `vec`-like and the elements are the nodes.
    macro_rules! tree {
        ($($x:expr),*) => (
            SyntaxTree { nodes: vec![$($x),*] }
        );
        ($($x:expr,)*) => (tree![$($x),*])
    }

    /// Parse the basic cases.
    #[test]
    fn parse_base() {
        test("", tree! {});
        test("Hello World!", tree! { W("Hello"), S, W("World"), W("!")});
    }

    /// Test whether newlines generate the correct whitespace.
    #[test]
    fn parse_newlines_whitespace() {
        test("Hello \n World", tree! { W("Hello"), S, N, S, W("World") });
        test("Hello\nWorld", tree! { W("Hello"), N, S, W("World") });
        test("Hello\n World", tree! { W("Hello"), N, S, W("World") });
        test("Hello \nWorld", tree! { W("Hello"), S, N, S, W("World") });
    }

    /// Parse things dealing with functions.
    #[test]
    fn parse_functions() {
        test("[test]", tree! { F(Function { name: "test", body: None }) });
        test("This is an [modifier][example] of a function invocation.", tree! {
            W("This"), S, W("is"), S, W("an"), S,
            F(Function { name: "modifier", body: Some(tree! { W("example") }) }), S,
            W("of"), S, W("a"), S, W("function"), S, W("invocation"), W(".")
        });
        test("[func][Hello][links][Here][end]",  tree! {
            F(Function {
                name: "func",
                body: Some(tree! { W("Hello") }),
            }),
            F(Function {
                name: "links",
                body: Some(tree! { W("Here") }),
            }),
            F(Function {
                name: "end",
                body: None,
            }),
        });
        test("[bodyempty][]", tree! {
            F(Function {
                name: "bodyempty",
                body: Some(tree! {})
            })
        });
        test("[nested][[func][call]] outside", tree! {
            F(Function {
                name: "nested",
                body: Some(tree! { F(Function {
                    name: "func",
                    body: Some(tree! { W("call") }),
                }), }),
            }),
            S, W("outside")
        });
    }

    /// Tests if the parser handles non-ASCII stuff correctly.
    #[test]
    fn parse_unicode() {
        test("[lib_parse] ⺐.", tree! {
            F(Function {
                name: "lib_parse",
                body: None
            }),
            S, W("⺐"), W(".")
        });
        test("[func123][Hello 🌍!]", tree! {
            F(Function {
                name: "func123",
                body: Some(tree! { W("Hello"), S, W("🌍"), W("!") }),
            })
        });
    }

    /// Tests whether errors get reported correctly.
    #[test]
    fn parse_errors() {
        test_err("No functions here]", "unexpected closing bracket");
        test_err("[hello][world", "expected closing bracket");
        test_err("[hello world", "expected closing bracket");
        test_err("[ no-name][Why?]", "expected identifier");
    }
}
