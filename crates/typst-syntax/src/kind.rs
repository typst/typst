use crate::SyntaxMode;

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
    /// A field access in math: `arrow.r.long.double.bar`.
    MathFieldAccess,
    /// A shorthand for a unicode codepoint in math: `a <= b`.
    MathShorthand,
    /// An alignment point in math: `&`.
    MathAlignPoint,
    /// A function call in math: `mat(delim: "[", a, b; ..#($c$,), d)`.
    MathCall,
    /// Function arguments in math: `(delim: "[", a, b; ..#($c$,), d)`.
    MathArgs,
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
            Self::MathFieldAccess => "math field access",
            Self::MathShorthand => "math shorthand",
            Self::MathAlignPoint => "math alignment point",
            Self::MathCall => "math function call",
            Self::MathArgs => "math call arguments",
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

/// How to determine the [`SyntaxMode`] for a syntax kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModeSearch {
    /// Comments do not have a syntax mode, but whitespace inherits the syntax
    /// mode of its parent.
    Comment,
    /// The mode is based on the parent's mode.
    Parent,
    /// The mode is always the same.
    Known(SyntaxMode),
    /// Normal identifiers are usually treated as code, but may be math if under
    /// a `Named` as part of a `MathArgs`.
    ///
    /// Just checking the parent would be incorrect because these can be
    /// directly embedded in math/markup with a hash as in `$#ident$`. This is
    /// the only `SyntaxKind` which is both embeddable with a hash and used in
    /// multiple modes.
    Ident,
}

impl SyntaxKind {
    /// How to determine the mode of this syntax kind.
    ///
    /// The high-level interface for this is [`crate::node::LinkedNode::mode`].
    pub(crate) fn mode_search(self) -> ModeSearch {
        match self {
            SyntaxKind::End => ModeSearch::Comment,
            SyntaxKind::Error => ModeSearch::Parent, // code | math | markup

            SyntaxKind::Shebang => ModeSearch::Comment,
            SyntaxKind::LineComment => ModeSearch::Comment,
            SyntaxKind::BlockComment => ModeSearch::Comment,

            SyntaxKind::Markup => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::Text => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::Space => ModeSearch::Parent, // code | math | markup
            SyntaxKind::Linebreak => ModeSearch::Parent, // math | markup
            SyntaxKind::Parbreak => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::Escape => ModeSearch::Parent, // math | markup
            SyntaxKind::Shorthand => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::SmartQuote => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::Strong => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::Emph => ModeSearch::Known(SyntaxMode::Markup),
            // TODO: Should we treat raw as 'code | markup'?
            SyntaxKind::Raw => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::RawLang => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::RawDelim => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::RawTrimmed => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::Link => ModeSearch::Known(SyntaxMode::Markup),
            // TODO: Should we treat labels as 'code | markup'?
            SyntaxKind::Label => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::Ref => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::RefMarker => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::Heading => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::HeadingMarker => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::ListItem => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::ListMarker => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::EnumItem => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::EnumMarker => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::TermItem => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::TermMarker => ModeSearch::Known(SyntaxMode::Markup),

            SyntaxKind::Equation => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::Math => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathText => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathIdent => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathFieldAccess => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathShorthand => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathAlignPoint => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathCall => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathArgs => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathDelimited => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathAttach => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathPrimes => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathFrac => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::MathRoot => ModeSearch::Known(SyntaxMode::Math),

            SyntaxKind::Hash => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::LeftBrace => ModeSearch::Parent, // code | markup
            SyntaxKind::RightBrace => ModeSearch::Parent, // code | markup
            SyntaxKind::LeftBracket => ModeSearch::Parent, // code | markup
            SyntaxKind::RightBracket => ModeSearch::Parent, // code | markup
            SyntaxKind::LeftParen => ModeSearch::Parent, // code | math
            SyntaxKind::RightParen => ModeSearch::Parent, // code | math
            SyntaxKind::Comma => ModeSearch::Parent,     // code | math
            // TODO: Semicolon after code is also embedded like a hash, so this
            // is incorrect...
            SyntaxKind::Semicolon => ModeSearch::Parent, // code | math
            SyntaxKind::Colon => ModeSearch::Parent,     // code | math | markup
            SyntaxKind::Star => ModeSearch::Parent,      // code | markup
            SyntaxKind::Underscore => ModeSearch::Parent, // code | math
            SyntaxKind::Dollar => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::Plus => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Minus => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Slash => ModeSearch::Parent, // code | math
            SyntaxKind::Hat => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::Dot => ModeSearch::Parent, // code | math
            SyntaxKind::Eq => ModeSearch::Parent,  // code | markup
            SyntaxKind::EqEq => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::ExclEq => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Lt => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::LtEq => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Gt => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::GtEq => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::PlusEq => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::HyphEq => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::StarEq => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::SlashEq => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Dots => ModeSearch::Parent, // code | math
            SyntaxKind::Arrow => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Root => ModeSearch::Known(SyntaxMode::Math),
            SyntaxKind::Bang => ModeSearch::Known(SyntaxMode::Math),

            SyntaxKind::Not => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::And => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Or => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::None => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Auto => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Let => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Set => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Show => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Context => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::If => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Else => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::For => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::In => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::While => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Break => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Continue => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Return => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Import => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Include => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::As => ModeSearch::Known(SyntaxMode::Code),

            SyntaxKind::Code => ModeSearch::Known(SyntaxMode::Code),
            // Either in code, or in math under a `Named`.
            SyntaxKind::Ident => ModeSearch::Ident, // code | math
            SyntaxKind::Bool => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Int => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Float => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Numeric => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Str => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::CodeBlock => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::ContentBlock => ModeSearch::Known(SyntaxMode::Markup),
            SyntaxKind::Parenthesized => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Array => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Dict => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Named => ModeSearch::Parent, // code | math
            SyntaxKind::Keyed => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Unary => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Binary => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::FieldAccess => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::FuncCall => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Args => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Spread => ModeSearch::Parent, // code | math
            SyntaxKind::Closure => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Params => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::LetBinding => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::SetRule => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::ShowRule => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Contextual => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Conditional => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::WhileLoop => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::ForLoop => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::ModuleImport => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::ImportItems => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::ImportItemPath => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::RenamedImportItem => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::ModuleInclude => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::LoopBreak => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::LoopContinue => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::FuncReturn => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::Destructuring => ModeSearch::Known(SyntaxMode::Code),
            SyntaxKind::DestructAssignment => ModeSearch::Known(SyntaxMode::Code),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{LinkedNode, Side, Source};

    #[track_caller]
    fn test_mode(
        text: &str,
        cursors: impl IntoIterator<Item = usize>,
        expected: Option<SyntaxMode>,
    ) {
        let source = Source::detached(text);
        let root = LinkedNode::new(source.root());
        for cursor in cursors {
            let leaf = root.leaf_at(cursor, Side::After).unwrap();
            assert_eq!(leaf.mode(), expected);
        }
    }

    #[test]
    fn test_linked_node_mode() {
        // Trivia
        test_mode("#! typ", [0], None);
        test_mode("// xxx", [0], None);
        test_mode("/* xxx */", [0], None);

        // Errors
        test_mode("*/", [0], Some(SyntaxMode::Markup));
        test_mode("#{*/}", [2], Some(SyntaxMode::Code));
        test_mode("$*/$", [1], Some(SyntaxMode::Math));

        // Markup
        test_mode("https://typst.org", [1], Some(SyntaxMode::Markup));
        test_mode("a\\bcd", [0, 1], Some(SyntaxMode::Markup));
        test_mode("a   c\n\n d\\\nef", [1, 5, 9], Some(SyntaxMode::Markup));
        test_mode("\"abc\"", [0, 4], Some(SyntaxMode::Markup));
        test_mode("a-?b", [1], Some(SyntaxMode::Markup));
        test_mode("_abcd_", [0], Some(SyntaxMode::Markup));
        test_mode("```typ {}   ```", [0, 3], Some(SyntaxMode::Markup));
        test_mode("```  xx  ```", [3], Some(SyntaxMode::Markup));
        test_mode("<label>", [0], Some(SyntaxMode::Markup));
        test_mode("@label", [0], Some(SyntaxMode::Markup));
        test_mode("= marker", [0], Some(SyntaxMode::Markup));
        test_mode("- marker", [0], Some(SyntaxMode::Markup));
        test_mode("+ marker", [0], Some(SyntaxMode::Markup));
        test_mode("/ marker", [0], Some(SyntaxMode::Markup));

        // Basic code
        test_mode("#{x;1}", [0, 1, 2, 3, 4], Some(SyntaxMode::Code));
        test_mode("#(x)", [1], Some(SyntaxMode::Code));
        test_mode("#(1,2,x)", [1, 2], Some(SyntaxMode::Code));
        test_mode("#(first:1, \"last\": 1)", [1, 7, 17], Some(SyntaxMode::Code));
        test_mode("#{-x}", [2], Some(SyntaxMode::Code));
        test_mode("#{a / b}", [4], Some(SyntaxMode::Code));
        test_mode("#a.b", [2], Some(SyntaxMode::Code));
        test_mode("#$$.at()", [3], Some(SyntaxMode::Code));
        test_mode("#[].at()", [3], Some(SyntaxMode::Code));
        test_mode("#f(x, ..y)", [2, 4, 6], Some(SyntaxMode::Code));
        test_mode("#{(x) => {}}", [3, 6], Some(SyntaxMode::Code));
        test_mode("#let x = 1", [1, 7], Some(SyntaxMode::Code));
        test_mode("#set text()", [1, 4], Some(SyntaxMode::Code));
        test_mode("#show text : it => it", [1, 11], Some(SyntaxMode::Code));
        test_mode("#context 1", [1, 8], Some(SyntaxMode::Code));
        test_mode("#while true {break;continue;}", [1, 13, 19], Some(SyntaxMode::Code));
        test_mode("#for a in b {}", [1, 7], Some(SyntaxMode::Code));
        test_mode("#if true {} else {}", [1, 12], Some(SyntaxMode::Code));
        test_mode(
            "#import \"lib.typ\" : a, b as d, e.f",
            [2, 8, 21, 25, 32],
            Some(SyntaxMode::Code),
        );
        test_mode("#include \"lib.typ\"", [1], Some(SyntaxMode::Code));
        test_mode("#let f() = { return 1 }", [13], Some(SyntaxMode::Code));
        test_mode("#{(x, _, ..y) = (1, 2, ..z)}", [2, 14], Some(SyntaxMode::Code));
        test_mode("= #1.1", [3], Some(SyntaxMode::Code));

        // Math
        test_mode("$ $", [0], Some(SyntaxMode::Math));
        test_mode("$arrow$", [1], Some(SyntaxMode::Math));
        test_mode("$123.32$", [1], Some(SyntaxMode::Math));
        test_mode("$1 2 3$", [2], Some(SyntaxMode::Math));
        test_mode("$+12 * y!", [1, 5, 8], Some(SyntaxMode::Math));
        test_mode("$1/2$", [2], Some(SyntaxMode::Math));
        test_mode("$f''$", [2], Some(SyntaxMode::Math));
        test_mode("$f_(x)^y$", [2, 3, 6, 7], Some(SyntaxMode::Math));
        test_mode("$a>=b$", [2], Some(SyntaxMode::Math));
        test_mode("$√x$", [1, 2], Some(SyntaxMode::Math));
        test_mode("$&x$", [1], Some(SyntaxMode::Math));
        test_mode("$\\#$", [1], Some(SyntaxMode::Math));
        test_mode(
            "$ff(x, sin(y), abs(z))$",
            [3, 4, 5, 7, 10, 15, 18],
            Some(SyntaxMode::Math),
        );
        test_mode("$ff(..args, named: key)$", [4, 6, 16, 17], Some(SyntaxMode::Math));
        test_mode("$arrow.r$", [6], Some(SyntaxMode::Math));

        // Nested math/code
        test_mode("$#$", [1], Some(SyntaxMode::Code));
        test_mode("$#pa$", [2], Some(SyntaxMode::Code));
        test_mode("$#{x}$", [2], Some(SyntaxMode::Code));
        test_mode(
            "$#f(x, ..args, named: key)$",
            [3, 4, 5, 7, 9, 19, 20],
            Some(SyntaxMode::Code),
        );
        test_mode("$#$x$$", [2], Some(SyntaxMode::Math));
        test_mode("$#context 1$", [10], Some(SyntaxMode::Code));
        test_mode("$#context $", [9], Some(SyntaxMode::Code));
        test_mode("$#std.align$", [5, 6], Some(SyntaxMode::Code));
        test_mode("$ff(named: #ident)$", [12], Some(SyntaxMode::Code));

        // Markup in code
        test_mode("$#[x]$", [2], Some(SyntaxMode::Markup));
    }
}
