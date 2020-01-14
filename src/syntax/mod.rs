//! Tokenization and parsing of source code.

use std::fmt::{self, Display, Formatter};
use unicode_xid::UnicodeXID;

use crate::func::LayoutFunc;
use crate::size::{Size, ScaleSize};


pub type ParseResult<T> = crate::TypesetResult<T>;

pub_use_mod!(color);
pub_use_mod!(expr);
pub_use_mod!(tokens);
pub_use_mod!(parsing);
pub_use_mod!(span);


/// A minimal semantic entity of source code.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Token<'s> {
    /// One or more whitespace characters. The contained `usize` denotes the
    /// number of newlines that were contained in the whitespace.
    Whitespace(usize),

    /// A line comment with inner string contents `//<&'s str>\n`.
    LineComment(&'s str),
    /// A block comment with inner string contents `/*<&'s str>*/`. The comment
    /// can contain nested block comments.
    BlockComment(&'s str),
    /// An erroneous `*/` without an opening block comment.
    StarSlash,

    /// A left bracket: `[`.
    LeftBracket,
    /// A right bracket: `]`.
    RightBracket,

    /// A left parenthesis in a function header: `(`.
    LeftParen,
    /// A right parenthesis in a function header: `)`.
    RightParen,
    /// A left brace in a function header: `{`.
    LeftBrace,
    /// A right brace in a function header: `}`.
    RightBrace,

    /// A colon in a function header: `:`.
    Colon,
    /// A comma in a function header: `:`.
    Comma,
    /// An equals sign in a function header: `=`.
    Equals,

    /// An identifier in a function header: `center`.
    ExprIdent(&'s str),
    /// A quoted string in a function header: `"..."`.
    ExprStr(&'s str),
    /// A number in a function header: `3.14`.
    ExprNumber(f64),
    /// A size in a function header: `12pt`.
    ExprSize(Size),
    /// A boolean in a function header: `true | false`.
    ExprBool(bool),

    /// A star in body-text.
    Star,
    /// An underscore in body-text.
    Underscore,
    /// A backtick in body-text.
    Backtick,

    /// Any other consecutive string.
    Text(&'s str),
}

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

    /// Add a node to the tree.
    pub fn add(&mut self, node: Spanned<Node>) {
        self.nodes.push(node);
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Colorization {
    pub colors: Vec<Spanned<ColorToken>>,
}

/// Entities which can be colored by syntax highlighting.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ColorToken {
    Comment,

    Bracket,
    FuncName,
    Colon,

    Key,
    Equals,
    Comma,

    Paren,
    Brace,

    ExprIdent,
    ExprStr,
    ExprNumber,
    ExprSize,
    ExprBool,

    Bold,
    Italic,
    Monospace,

    Invalid,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ErrorMap {
    pub errors: Vec<Spanned<String>>,
}

#[derive(Debug)]
pub struct FuncHeader {
    pub name: Spanned<Ident>,
    pub args: FuncArgs,
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
