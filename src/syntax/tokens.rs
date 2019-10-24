use std::str::CharIndices;
use smallvec::SmallVec;
use super::*;

/// Builds an iterator over the tokens of the source code.
#[inline]
pub fn tokenize(src: &str) -> Tokens {
    Tokens::new(src)
}

/// An iterator over the tokens of source code.
#[derive(Debug, Clone)]
pub struct Tokens<'s> {
    src: &'s str,
    pub(super) chars: PeekableChars<'s>,
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
        Token::Text(&self.src[start..end])
    }
}

impl<'s> Iterator for Tokens<'s> {
    type Item = Token<'s>;

    /// Advance the iterator, return the next token or nothing.
    fn next(&mut self) -> Option<Token<'s>> {
        use TokensState as TU;

        // Go to the body state if the function has a body or return to the top-of-stack
        // state.
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
            }
            ']' => {
                if self.state == TU::Function {
                    self.state = TU::MaybeBody;
                } else {
                    self.unswitch();
                }
                Token::RightBracket
            }

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
                Token::LineComment(&self.src[start..end])
            }

            // Block comment
            '/' if afterwards == Some('*') => {
                let mut end = self.chars.next().unwrap();
                let start = end.0 + end.1.len_utf8();

                let mut nested = 0;
                while let Some((index, c)) = self.chars.next() {
                    let after = self.chars.peek().map(|p| p.1);
                    match (c, after) {
                        ('*', Some('/')) if nested == 0 => {
                            self.advance();
                            break;
                        }
                        ('/', Some('*')) => {
                            self.advance();
                            nested += 1
                        }
                        ('*', Some('/')) => {
                            self.advance();
                            nested -= 1
                        }
                        _ => {}
                    }
                    end = (index, c);
                }

                let end = end.0 + end.1.len_utf8();
                Token::BlockComment(&self.src[start..end])
            }

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
                Token::Quoted(&self.src[next_pos + 1..end_pos])
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
            }

            // Normal text
            _ => {
                // Find out when the word ends.
                let mut end = (next_pos, next);
                while let Some((index, c)) = self.chars.peek() {
                    let second = self.chars.peekn(1).map(|p| p.1);

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
            }
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
pub struct PeekableChars<'s> {
    string: &'s str,
    chars: CharIndices<'s>,
    base: usize,
    peeked: SmallVec<[Option<(usize, char)>; 2]>,
}

impl<'s> PeekableChars<'s> {
    /// Create a new iterator from a string.
    pub fn new(string: &'s str) -> PeekableChars<'s> {
        PeekableChars {
            string,
            chars: string.char_indices(),
            base: 0,
            peeked: SmallVec::new(),
        }
    }

    /// Peek at the next element.
    pub fn peek(&mut self) -> Option<(usize, char)> {
        self.peekn(0)
    }

    /// Peek at the element after the next element.
    pub fn peekn(&mut self, n: usize) -> Option<(usize, char)> {
        while self.peeked.len() <= n {
            let next = self.next_inner();
            self.peeked.push(next);
        }

        self.peeked[n]
    }

    /// Return the next value of the inner iterator mapped with the offset.
    pub fn next_inner(&mut self) -> Option<(usize, char)> {
        self.chars.next().map(|(i, c)| (i + self.base, c))
    }

    /// The index of the first character of the next token in the source string.
    pub fn string_index(&mut self) -> Option<usize> {
        self.peek().map(|p| p.0)
    }

    /// Go to a new position in the underlying string.
    pub fn set_string_index(&mut self, index: usize) {
        self.chars = self.string[index..].char_indices();
        self.base = index;
        self.peeked.clear();
    }
}

impl Iterator for PeekableChars<'_> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<(usize, char)> {
        if !self.peeked.is_empty() {
            self.peeked.remove(0)
        } else {
            self.next_inner()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Token::{
        Backtick as TB, BlockComment as BC, Colon as C, Equals as E, LeftBracket as L,
        LineComment as LC, Newline as N, Quoted as Q, RightBracket as R, Space as S, Star as TS,
        StarSlash as SS, Text as T, Underscore as TU,
    };

    /// Test if the source code tokenizes to the tokens.
    fn test(src: &str, tokens: Vec<Token>) {
        assert_eq!(Tokens::new(src).collect::<Vec<_>>(), tokens);
    }

    /// Tokenizes the basic building blocks.
    #[test]
    #[rustfmt::skip]
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
    #[rustfmt::skip]
    fn tokenize_whitespace_newlines() {
        test(" \t", vec![S]);
        test("First line\r\nSecond line\nThird line\n", vec![
            T("First"), S, T("line"), N, T("Second"), S, T("line"), N,
            T("Third"), S, T("line"), N
        ]);
        test("Hello \n ", vec![T("Hello"), S, N, S]);
        test("Dense\nTimes", vec![T("Dense"), N, T("Times")]);
    }

    /// Tests if escaping with backslash works as it should.
    #[test]
    #[rustfmt::skip]
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
    #[rustfmt::skip]
    fn tokenize_quoted() {
        test(r#"[align: "hello\"world"]"#, vec![L, T("align"), C, S, Q(r#"hello\"world"#), R]);
    }

    /// Tokenizes some more realistic examples.
    #[test]
    #[rustfmt::skip]
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
    #[rustfmt::skip]
    fn tokenize_symbols_context() {
        test("[func: key=value][Answer: 7]", vec![
            L, T("func"), C, S, T("key"), E, T("value"), R, L,
            T("Answer:"), S, T("7"), R
        ]);
        test("[[n: k=v]:x][:[=]]:=", vec![
            L, L, T("n"), C, S, T("k"), E, T("v"), R, C, T("x"), R,
            L, T(":"), L, E, R, R, T(":=")
        ]);
        test("[hi: k=[func][body] v=1][hello]", vec![
            L, T("hi"), C, S, T("k"), E, L, T("func"), R, L, T("body"), R, S,
            T("v"), E, T("1"), R, L, T("hello"), R
        ]);
        test("[func: __key__=value]", vec![L, T("func"), C, S, T("__key__"), E, T("value"), R]);
        test("The /*[*/ answer: 7.", vec![T("The"), S, BC("["), S, T("answer:"), S, T("7.")]);
    }

    /// Test if block and line comments get tokenized as expected.
    #[test]
    #[rustfmt::skip]
    fn tokenize_comments() {
        test("These // Line comments.", vec![T("These"), S, LC(" Line comments.")]);
        test("This /* is */ a comment.", vec![T("This"), S, BC(" is "), S, T("a"), S, T("comment.")]);
        test("[Head/*of*/][Body]", vec![L, T("Head"), BC("of"), R, L, T("Body"), R]);
        test("/* Hey */ */", vec![BC(" Hey "), S, SS]);
        test("Hey\n// Yoo /*\n*/", vec![T("Hey"), N, LC(" Yoo /*"), N, SS]);
        test("/* My /* line // */ comment */", vec![BC(" My /* line // */ comment ")])
    }

    /// This test has a special look at the underscore syntax.
    #[test]
    #[rustfmt::skip]
    fn tokenize_underscores() {
        test("he_llo_world_ __ Now this_ is_ special!",
             vec![T("he"), TU, T("llo"), TU, T("world"), TU, S, TU, TU, S, T("Now"), S,
                  T("this"), TU, S, T("is"), TU, S, T("special!")]);
    }

    /// This test is for checking if non-ASCII characters get parsed correctly.
    #[test]
    #[rustfmt::skip]
    fn tokenize_unicode() {
        test("[document][Hello 🌍!]", vec![L, T("document"), R, L, T("Hello"), S, T("🌍!"), R]);
        test("[f]⺐.", vec![L, T("f"), R, T("⺐.")]);
    }
}
