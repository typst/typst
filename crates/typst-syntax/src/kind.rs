/// A syntactical building block of a Typst file.
///
/// Can be created by the lexer or by the parser.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum SyntaxKind {
    /// The end of token stream.
    End,
    /// An invalid sequence of characters.
    Error,

    /// A shebang: `#! ...`
    Shebang,
    /// A line comment: `// ...`.
    LineComment,
    /// A block comment: `/* ... */`.
    BlockComment,

    /// The contents of a file or content block.
    Markup,
    /// Plain text without markup.
    Text,
    /// Whitespace. Contains at most one newline in markup, as more indicate a
    /// paragraph break.
    Space,
    /// A forced line break: `\`.
    Linebreak,
    /// A paragraph break, indicated by one or multiple blank lines.
    Parbreak,
    /// An escape sequence: `\#`, `\u{1F5FA}`.
    Escape,
    /// A shorthand for a unicode codepoint. For example, `~` for non-breaking
    /// space or `-?` for a soft hyphen.
    Shorthand,
    /// A smart quote: `'` or `"`.
    SmartQuote,
    /// Strong content: `*Strong*`.
    Strong,
    /// Emphasized content: `_Emphasized_`.
    Emph,
    /// Raw text with optional syntax highlighting: `` `...` ``.
    Raw,
    /// A language tag at the start of raw text: ``typ ``.
    RawLang,
    /// A raw delimiter consisting of 1 or 3+ backticks: `` ` ``.
    RawDelim,
    /// A sequence of whitespace to ignore in a raw text: `    `.
    RawTrimmed,
    /// A hyperlink: `https://typst.org`.
    Link,
    /// A label: `<intro>`.
    Label,
    /// A reference: `@target`, `@target[..]`.
    Ref,
    /// Introduces a reference: `@target`.
    RefMarker,
    /// A section heading: `= Introduction`.
    Heading,
    /// Introduces a section heading: `=`, `==`, ...
    HeadingMarker,
    /// An item in a bullet list: `- ...`.
    ListItem,
    /// Introduces a list item: `-`.
    ListMarker,
    /// An item in an enumeration (numbered list): `+ ...` or `1. ...`.
    EnumItem,
    /// Introduces an enumeration item: `+`, `1.`.
    EnumMarker,
    /// An item in a term list: `/ Term: Details`.
    TermItem,
    /// Introduces a term item: `/`.
    TermMarker,
    /// A mathematical equation: `$x$`, `$ x^2 $`.
    Equation,

    /// The contents of a mathematical equation: `x^2 + 1`.
    Math,
    /// A lone text fragment in math: `x`, `25`, `3.1415`, `=`, `|`, `[`.
    MathText,
    /// An identifier in math: `pi`.
    MathIdent,
    /// A shorthand for a unicode codepoint in math: `a <= b`.
    MathShorthand,
    /// An alignment point in math: `&`.
    MathAlignPoint,
    /// Matched delimiters in math: `[x + y]`.
    MathDelimited,
    /// A base with optional attachments in math: `a_1^2`.
    MathAttach,
    /// Grouped primes in math: `a'''`.
    MathPrimes,
    /// A fraction in math: `x/2`.
    MathFrac,
    /// A root in math: `√x`, `∛x` or `∜x`.
    MathRoot,

    /// A hash that switches into code mode: `#`.
    Hash,
    /// A left curly brace, starting a code block: `{`.
    LeftBrace,
    /// A right curly brace, terminating a code block: `}`.
    RightBrace,
    /// A left square bracket, starting a content block: `[`.
    LeftBracket,
    /// A right square bracket, terminating a content block: `]`.
    RightBracket,
    /// A left round parenthesis, starting a grouped expression, collection,
    /// argument or parameter list: `(`.
    LeftParen,
    /// A right round parenthesis, terminating a grouped expression, collection,
    /// argument or parameter list: `)`.
    RightParen,
    /// A comma separator in a sequence: `,`.
    Comma,
    /// A semicolon terminating an expression: `;`.
    Semicolon,
    /// A colon between name/key and value in a dictionary, argument or
    /// parameter list, or between the term and body of a term list term: `:`.
    Colon,
    /// The strong text toggle, multiplication operator, and wildcard import
    /// symbol: `*`.
    Star,
    /// Toggles emphasized text and indicates a subscript in math: `_`.
    Underscore,
    /// Starts and ends a mathematical equation: `$`.
    Dollar,
    /// The unary plus and binary addition operator: `+`.
    Plus,
    /// The unary negation and binary subtraction operator: `-`.
    Minus,
    /// The division operator and fraction operator in math: `/`.
    Slash,
    /// The superscript operator in math: `^`.
    Hat,
    /// The field access and method call operator: `.`.
    Dot,
    /// The assignment operator: `=`.
    Eq,
    /// The equality operator: `==`.
    EqEq,
    /// The inequality operator: `!=`.
    ExclEq,
    /// The less-than operator: `<`.
    Lt,
    /// The less-than or equal operator: `<=`.
    LtEq,
    /// The greater-than operator: `>`.
    Gt,
    /// The greater-than or equal operator: `>=`.
    GtEq,
    /// The add-assign operator: `+=`.
    PlusEq,
    /// The subtract-assign operator: `-=`.
    HyphEq,
    /// The multiply-assign operator: `*=`.
    StarEq,
    /// The divide-assign operator: `/=`.
    SlashEq,
    /// Indicates a spread or sink: `..`.
    Dots,
    /// An arrow between a closure's parameters and body: `=>`.
    Arrow,
    /// A root: `√`, `∛` or `∜`.
    Root,
    /// An exclamation mark; groups with directly preceding text in math: `!`.
    Bang,

    /// The `not` operator.
    Not,
    /// The `and` operator.
    And,
    /// The `or` operator.
    Or,
    /// The `none` literal.
    None,
    /// The `auto` literal.
    Auto,
    /// The `let` keyword.
    Let,
    /// The `set` keyword.
    Set,
    /// The `show` keyword.
    Show,
    /// The `context` keyword.
    Context,
    /// The `if` keyword.
    If,
    /// The `else` keyword.
    Else,
    /// The `for` keyword.
    For,
    /// The `in` keyword.
    In,
    /// The `while` keyword.
    While,
    /// The `break` keyword.
    Break,
    /// The `continue` keyword.
    Continue,
    /// The `return` keyword.
    Return,
    /// The `import` keyword.
    Import,
    /// The `include` keyword.
    Include,
    /// The `as` keyword.
    As,

    /// The contents of a code block.
    Code,
    /// An identifier: `it`.
    Ident,
    /// A boolean: `true`, `false`.
    Bool,
    /// An integer: `120`.
    Int,
    /// A floating-point number: `1.2`, `10e-4`.
    Float,
    /// A numeric value with a unit: `12pt`, `3cm`, `2em`, `90deg`, `50%`.
    Numeric,
    /// A quoted string: `"..."`.
    Str,
    /// A code block: `{ let x = 1; x + 2 }`.
    CodeBlock,
    /// A content block: `[*Hi* there!]`.
    ContentBlock,
    /// A grouped expression: `(1 + 2)`.
    Parenthesized,
    /// An array: `(1, "hi", 12cm)`.
    Array,
    /// A dictionary: `(thickness: 3pt, dash: "solid")`.
    Dict,
    /// A named pair: `thickness: 3pt`.
    Named,
    /// A keyed pair: `"spacy key": true`.
    Keyed,
    /// A unary operation: `-x`.
    Unary,
    /// A binary operation: `a + b`.
    Binary,
    /// A field access: `properties.age`.
    FieldAccess,
    /// An invocation of a function or method: `f(x, y)`.
    FuncCall,
    /// A function call's argument list: `(12pt, y)`.
    Args,
    /// Spread arguments or an argument sink: `..x`.
    Spread,
    /// A closure: `(x, y) => z`.
    Closure,
    /// A closure's parameters: `(x, y)`.
    Params,
    /// A let binding: `let x = 1`.
    LetBinding,
    /// A set rule: `set text(...)`.
    SetRule,
    /// A show rule: `show heading: it => emph(it.body)`.
    ShowRule,
    /// A contextual expression: `context text.lang`.
    Contextual,
    /// An if-else conditional: `if x { y } else { z }`.
    Conditional,
    /// A while loop: `while x { y }`.
    WhileLoop,
    /// A for loop: `for x in y { z }`.
    ForLoop,
    /// A module import: `import "utils.typ": a, b, c`.
    ModuleImport,
    /// Items to import from a module: `a, b, c`.
    ImportItems,
    /// A path to an imported name from a submodule: `a.b.c`.
    ImportItemPath,
    /// A renamed import item: `a as d`.
    RenamedImportItem,
    /// A module include: `include "chapter1.typ"`.
    ModuleInclude,
    /// A break from a loop: `break`.
    LoopBreak,
    /// A continue in a loop: `continue`.
    LoopContinue,
    /// A return from a function: `return`, `return x + 1`.
    FuncReturn,
    /// A destructuring pattern: `(x, _, ..y)`.
    Destructuring,
    /// A destructuring assignment expression: `(x, y) = (1, 2)`.
    DestructAssignment,
}

impl SyntaxKind {
    /// Is this a bracket, brace, or parenthesis?
    pub fn is_grouping(self) -> bool {
        matches!(
            self,
            Self::LeftBracket
                | Self::LeftBrace
                | Self::LeftParen
                | Self::RightBracket
                | Self::RightBrace
                | Self::RightParen
        )
    }

    /// Does this node terminate a preceding expression?
    pub fn is_terminator(self) -> bool {
        matches!(
            self,
            Self::End
                | Self::Semicolon
                | Self::RightBrace
                | Self::RightParen
                | Self::RightBracket
        )
    }

    /// Is this a code or content block.
    pub fn is_block(self) -> bool {
        matches!(self, Self::CodeBlock | Self::ContentBlock)
    }

    /// Does this node need termination through a semicolon or linebreak?
    pub fn is_stmt(self) -> bool {
        matches!(
            self,
            Self::LetBinding
                | Self::SetRule
                | Self::ShowRule
                | Self::ModuleImport
                | Self::ModuleInclude
        )
    }

    /// Is this node is a keyword.
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            Self::Not
                | Self::And
                | Self::Or
                | Self::None
                | Self::Auto
                | Self::Let
                | Self::Set
                | Self::Show
                | Self::Context
                | Self::If
                | Self::Else
                | Self::For
                | Self::In
                | Self::While
                | Self::Break
                | Self::Continue
                | Self::Return
                | Self::Import
                | Self::Include
                | Self::As
        )
    }

    /// Whether this kind of node is automatically skipped by the parser in
    /// code and math mode.
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::Shebang
                | Self::LineComment
                | Self::BlockComment
                | Self::Space
                | Self::Parbreak
        )
    }

    /// Whether this is an error.
    pub fn is_error(self) -> bool {
        self == Self::Error
    }

    /// A human-readable name for the kind.
    pub fn name(self) -> &'static str {
        match self {
            Self::End => "end of tokens",
            Self::Error => "syntax error",
            Self::Shebang => "shebang",
            Self::LineComment => "line comment",
            Self::BlockComment => "block comment",
            Self::Markup => "markup",
            Self::Text => "text",
            Self::Space => "space",
            Self::Linebreak => "line break",
            Self::Parbreak => "paragraph break",
            Self::Escape => "escape sequence",
            Self::Shorthand => "shorthand",
            Self::SmartQuote => "smart quote",
            Self::Strong => "strong content",
            Self::Emph => "emphasized content",
            Self::Raw => "raw block",
            Self::RawLang => "raw language tag",
            Self::RawTrimmed => "raw trimmed",
            Self::RawDelim => "raw delimiter",
            Self::Link => "link",
            Self::Label => "label",
            Self::Ref => "reference",
            Self::RefMarker => "reference marker",
            Self::Heading => "heading",
            Self::HeadingMarker => "heading marker",
            Self::ListItem => "list item",
            Self::ListMarker => "list marker",
            Self::EnumItem => "enum item",
            Self::EnumMarker => "enum marker",
            Self::TermItem => "term list item",
            Self::TermMarker => "term marker",
            Self::Equation => "equation",
            Self::Math => "math",
            Self::MathText => "math text",
            Self::MathIdent => "math identifier",
            Self::MathShorthand => "math shorthand",
            Self::MathAlignPoint => "math alignment point",
            Self::MathDelimited => "delimited math",
            Self::MathAttach => "math attachments",
            Self::MathFrac => "math fraction",
            Self::MathRoot => "math root",
            Self::MathPrimes => "math primes",
            Self::Hash => "hash",
            Self::LeftBrace => "opening brace",
            Self::RightBrace => "closing brace",
            Self::LeftBracket => "opening bracket",
            Self::RightBracket => "closing bracket",
            Self::LeftParen => "opening paren",
            Self::RightParen => "closing paren",
            Self::Comma => "comma",
            Self::Semicolon => "semicolon",
            Self::Colon => "colon",
            Self::Star => "star",
            Self::Underscore => "underscore",
            Self::Dollar => "dollar sign",
            Self::Plus => "plus",
            Self::Minus => "minus",
            Self::Slash => "slash",
            Self::Hat => "hat",
            Self::Dot => "dot",
            Self::Eq => "equals sign",
            Self::EqEq => "equality operator",
            Self::ExclEq => "inequality operator",
            Self::Lt => "less-than operator",
            Self::LtEq => "less-than or equal operator",
            Self::Gt => "greater-than operator",
            Self::GtEq => "greater-than or equal operator",
            Self::PlusEq => "add-assign operator",
            Self::HyphEq => "subtract-assign operator",
            Self::StarEq => "multiply-assign operator",
            Self::SlashEq => "divide-assign operator",
            Self::Dots => "dots",
            Self::Arrow => "arrow",
            Self::Root => "root",
            Self::Bang => "exclamation mark",
            Self::Not => "operator `not`",
            Self::And => "operator `and`",
            Self::Or => "operator `or`",
            Self::None => "`none`",
            Self::Auto => "`auto`",
            Self::Let => "keyword `let`",
            Self::Set => "keyword `set`",
            Self::Show => "keyword `show`",
            Self::Context => "keyword `context`",
            Self::If => "keyword `if`",
            Self::Else => "keyword `else`",
            Self::For => "keyword `for`",
            Self::In => "keyword `in`",
            Self::While => "keyword `while`",
            Self::Break => "keyword `break`",
            Self::Continue => "keyword `continue`",
            Self::Return => "keyword `return`",
            Self::Import => "keyword `import`",
            Self::Include => "keyword `include`",
            Self::As => "keyword `as`",
            Self::Code => "code",
            Self::Ident => "identifier",
            Self::Bool => "boolean",
            Self::Int => "integer",
            Self::Float => "float",
            Self::Numeric => "numeric value",
            Self::Str => "string",
            Self::CodeBlock => "code block",
            Self::ContentBlock => "content block",
            Self::Parenthesized => "group",
            Self::Array => "array",
            Self::Dict => "dictionary",
            Self::Named => "named pair",
            Self::Keyed => "keyed pair",
            Self::Unary => "unary expression",
            Self::Binary => "binary expression",
            Self::FieldAccess => "field access",
            Self::FuncCall => "function call",
            Self::Args => "call arguments",
            Self::Spread => "spread",
            Self::Closure => "closure",
            Self::Params => "closure parameters",
            Self::LetBinding => "`let` expression",
            Self::SetRule => "`set` expression",
            Self::ShowRule => "`show` expression",
            Self::Contextual => "`context` expression",
            Self::Conditional => "`if` expression",
            Self::WhileLoop => "while-loop expression",
            Self::ForLoop => "for-loop expression",
            Self::ModuleImport => "`import` expression",
            Self::ImportItems => "import items",
            Self::ImportItemPath => "imported item path",
            Self::RenamedImportItem => "renamed import item",
            Self::ModuleInclude => "`include` expression",
            Self::LoopBreak => "`break` expression",
            Self::LoopContinue => "`continue` expression",
            Self::FuncReturn => "`return` expression",
            Self::Destructuring => "destructuring pattern",
            Self::DestructAssignment => "destructuring assignment expression",
        }
    }
}
