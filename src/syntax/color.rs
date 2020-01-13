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
