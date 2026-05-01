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

/// How to determine the [`SyntaxMode`] we will be in when immediately after a
/// node of this kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModeAfter {
    /// The mode is known!
    Known(SyntaxMode),
    /// The syntax mode after this kind is based on its parent.
    Parent,
    /// We treat comments and the bodies of raw text as not producing any syntax
    /// mode.
    None,
    /// Text under `Raw` is `None`, otherwise it is markup.
    Text,
    /// After the opening raw delimiter is `None`, but after the closing one is
    /// the same as the parent `Raw`.
    RawDelim,
    /// Dollar signs in an equation. After the opening dollar sign is math mode,
    /// after the closing dollar sign is the same as the parent `Equation`.
    Dollar,
    /// Spaces are usually only based on their parent, but an edge case with
    /// `Equation` requires special handling.
    ///
    /// In equations, unlike in code and content blocks, the spaces at the edges
    /// are not included in the `Math` wrapper kind, and require special
    /// handling to still return as math mode.
    Space,
    /// Kinds that can be embedded in markup or math with a hash, but can also
    /// be used without a hash in other contexts.
    ///
    /// As an example, the mode after `<label>` in these will be:
    /// - Code in `#let x = <label>`
    /// - Markup in `<label>`
    /// - Code in `#<label>`
    Embeddable,
}

impl SyntaxKind {
    /// How to determine the [`SyntaxMode`] we will be in when immediately after
    /// a node of this kind.
    ///
    /// The high-level interface for this is [`crate::LinkedNode::mode_after`].
    pub(crate) fn mode_after(self) -> ModeAfter {
        use ModeAfter::*;
        use SyntaxMode::{Code, Markup, Math};

        // For the kinds which are not `Known`, the possible modes are listed in
        // a comment with the parent kinds that can cause that mode, although
        // `expr` and `part` are used as shorthands when the list of parent
        // kinds would be too long (otherwise, the list of kinds should be
        // exhaustive). `expr` is a kind which can be a top-level expression in
        // its mode, and `part` is a kind which is just a part of an expression.
        match self {
            Self::End => None,     // none
            Self::Error => Parent, // code/math/markup

            Self::Shebang => None,      // none
            Self::LineComment => None,  // none
            Self::BlockComment => None, // none

            Self::Markup => Known(Markup),
            Self::Text => Text,        // none: Raw | markup: expr
            Self::Space => Space,      // code/math: part | markup: expr/part
            Self::Linebreak => Parent, // math/markup: expr
            Self::Parbreak => Known(Markup),
            Self::Escape => Parent, // math/markup: expr
            Self::Shorthand => Known(Markup),
            Self::SmartQuote => Known(Markup),
            Self::Strong => Known(Markup),
            Self::Emph => Known(Markup),
            Self::Raw => Embeddable,    // code/markup: expr
            Self::RawLang => None,      // none
            Self::RawDelim => RawDelim, // none: opening backticks | code/markup: Raw
            Self::RawTrimmed => None,   // none
            Self::Link => Known(Markup),
            Self::Label => Embeddable, // code/markup: expr
            Self::Ref => Known(Markup),
            Self::RefMarker => Known(Markup),
            Self::Heading => Known(Markup),
            Self::HeadingMarker => Known(Markup),
            Self::ListItem => Known(Markup),
            Self::ListMarker => Known(Markup),
            Self::EnumItem => Known(Markup),
            Self::EnumMarker => Known(Markup),
            Self::TermItem => Known(Markup),
            Self::TermMarker => Known(Markup),

            Self::Equation => Embeddable, // code/markup: expr
            Self::Math => Known(Math),
            Self::MathText => Known(Math),
            Self::MathIdent => Known(Math),
            Self::MathFieldAccess => Known(Math),
            Self::MathShorthand => Known(Math),
            Self::MathAlignPoint => Known(Math),
            Self::MathCall => Known(Math),
            Self::MathArgs => Known(Math),
            Self::MathDelimited => Known(Math),
            Self::MathAttach => Known(Math),
            Self::MathPrimes => Known(Math),
            Self::MathFrac => Known(Math),
            Self::MathRoot => Known(Math),

            Self::Hash => Known(Code),
            Self::LeftBrace => Known(Code),
            Self::RightBrace => Known(Code),
            Self::LeftBracket => Known(Markup),
            Self::RightBracket => Parent, // code/markup: ContentBlock
            Self::LeftParen => Parent,    // code: part | math: MathArgs/MathDelimited
            Self::RightParen => Parent,   // code: part | math: MathArgs/MathDelimited
            Self::Comma => Parent,        // code: part | math: MathArgs
            Self::Semicolon => Parent,    // code: CodeBlock | math/markup: after embedded
            Self::Colon => Parent,        // code: part | math: Named | markup: TermItem
            Self::Star => Parent,         // code: Binary/ModuleImport | markup: Strong
            Self::Underscore => Parent,   // code: part | math: MathAttach | markup: Emph
            Self::Dollar => Dollar,       // code/markup: Equation | math: opening dollar
            Self::Plus => Known(Code),
            Self::Minus => Known(Code),
            Self::Slash => Parent, // code: Binary | math: MathFrac
            Self::Hat => Known(Math),
            Self::Dot => Parent, // code: part | math: MathFieldAccess
            Self::Eq => Known(Code),
            Self::EqEq => Known(Code),
            Self::ExclEq => Known(Code),
            Self::Lt => Known(Code),
            Self::LtEq => Known(Code),
            Self::Gt => Known(Code),
            Self::GtEq => Known(Code),
            Self::PlusEq => Known(Code),
            Self::HyphEq => Known(Code),
            Self::StarEq => Known(Code),
            Self::SlashEq => Known(Code),
            Self::Dots => Parent, // code/math: Spread
            Self::Arrow => Known(Code),
            Self::Root => Known(Math),
            Self::Bang => Known(Math),

            Self::Not => Known(Code),
            Self::And => Known(Code),
            Self::Or => Known(Code),
            Self::None => Known(Code),
            Self::Auto => Known(Code),
            Self::Let => Known(Code),
            Self::Set => Known(Code),
            Self::Show => Known(Code),
            Self::Context => Known(Code),
            Self::If => Known(Code),
            Self::Else => Known(Code),
            Self::For => Known(Code),
            Self::In => Known(Code),
            Self::While => Known(Code),
            Self::Break => Known(Code),
            Self::Continue => Known(Code),
            Self::Return => Known(Code),
            Self::Import => Known(Code),
            Self::Include => Known(Code),
            Self::As => Known(Code),

            Self::Code => Known(Code),
            Self::Ident => Embeddable, // code: expr/part | math: Named
            Self::Bool => Known(Code),
            Self::Int => Known(Code),
            Self::Float => Known(Code),
            Self::Numeric => Known(Code),
            Self::Str => Embeddable, // code/math: expr
            Self::CodeBlock => Known(Code),
            Self::ContentBlock => Embeddable, // code: expr | markup: Ref
            Self::Parenthesized => Known(Code),
            Self::Array => Known(Code),
            Self::Dict => Known(Code),
            Self::Named => Parent, // code: part | math: MathArgs
            Self::Keyed => Known(Code),
            Self::Unary => Known(Code),
            Self::Binary => Known(Code),
            Self::FieldAccess => Known(Code),
            Self::FuncCall => Known(Code),
            Self::Args => Known(Code),
            Self::Spread => Parent, // code: part | math: MathArgs
            Self::Closure => Known(Code),
            Self::Params => Known(Code),
            Self::LetBinding => Known(Code),
            Self::SetRule => Known(Code),
            Self::ShowRule => Known(Code),
            Self::Contextual => Known(Code),
            Self::Conditional => Known(Code),
            Self::WhileLoop => Known(Code),
            Self::ForLoop => Known(Code),
            Self::ModuleImport => Known(Code),
            Self::ImportItems => Known(Code),
            Self::ImportItemPath => Known(Code),
            Self::RenamedImportItem => Known(Code),
            Self::ModuleInclude => Known(Code),
            Self::LoopBreak => Known(Code),
            Self::LoopContinue => Known(Code),
            Self::FuncReturn => Known(Code),
            Self::Destructuring => Known(Code),
            Self::DestructAssignment => Known(Code),
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
            let c = source.text()[cursor..].chars().next().unwrap();
            assert_eq!(leaf.mode_after(), expected, "different at '{c}' index {cursor}");
        }
    }

    #[test]
    fn test_mode_after() {
        use SyntaxMode::{Code, Markup, Math};

        // Trivia
        test_mode("#! typ", [0, 1, 2, 3], None);
        test_mode("#! typ\n", [6], Some(Markup));
        test_mode("// xxx", [0, 1, 2, 3], None);
        test_mode("\n// xxx\n", [0, 7], Some(Markup));
        test_mode("/* xxx */", [0, 1, 2, 3, 4, 7, 8], None);

        // Syntax errors
        test_mode("*/", [0, 1], Some(Markup));
        test_mode("#{*/}", [2, 3], Some(Code));
        test_mode("$*/$", [1, 2], Some(Math));

        // Markup
        test_mode("https://typst.org", [0], Some(Markup));
        test_mode("a\\bcd", [0, 1, 2, 3], Some(Markup));
        test_mode("a   c\n\n d\\\nef", [1, 5, 9], Some(Markup));
        test_mode("\\u{41}", [0, 1, 2, 3, 5], Some(Markup));
        test_mode("\"abc\"", [0, 1, 4], Some(Markup));
        test_mode("a-?b", [1, 2], Some(Markup));
        test_mode("_*abcd*_", [0, 1, 6, 7], Some(Markup));
        test_mode("<label>", [0, 1], Some(Markup));
        test_mode("@label[x @y]", [0, 1, 6, 7, 8, 9, 11], Some(Markup));
        test_mode("= marker", [0, 1, 2], Some(Markup));
        test_mode("- marker", [0, 1, 2], Some(Markup));
        test_mode("+ marker", [0, 1, 2], Some(Markup));
        test_mode("/ marker: y", [0, 1, 2, 8, 9, 10], Some(Markup));

        // Basic code
        test_mode("#{x;1}", [0, 1, 2, 3, 4], Some(Code));
        test_mode("#(x)", [1, 2, 3], Some(Code));
        test_mode("#(1,2,)", [1, 2, 3, 5, 6], Some(Code));
        test_mode("#(a:1,\"b\":2)", [1, 2, 3, 4, 5, 6, 7, 8, 9, 11], Some(Code));
        test_mode("#{-x}", [2], Some(Code));
        test_mode("#{a / b}", [4], Some(Code));
        test_mode("#a.b", [1, 2, 3], Some(Code));
        test_mode("#$$.at()", [2, 3], Some(Code));
        test_mode("#[].at()", [2, 3], Some(Code));
        test_mode("#f(x, ..y)", [2, 4, 6], Some(Code));
        test_mode("#{(x) => {}}", [3, 6], Some(Code));
        test_mode("#let x = 1", [1, 7], Some(Code));
        test_mode("#let x;", [6], Some(Markup)); // After semicolon is the parent
        test_mode("#set text()", [1, 4], Some(Code));
        test_mode("#show text : it => it", [1, 11], Some(Code));
        test_mode("#context 1", [1, 8], Some(Code));
        test_mode("#while true {break;continue;}", [1, 13, 19], Some(Code));
        test_mode("#for a in b {}", [1, 7], Some(Code));
        test_mode("#if true {} else {}", [1, 12], Some(Code));
        test_mode("#import \"lib.typ\" : a, b as d, e.f", [2, 8, 21, 25, 32], Some(Code));
        test_mode("#include \"lib.typ\"", [1], Some(Code));
        test_mode("#let f() = { return 1 }", [13], Some(Code));
        test_mode("#{(x, _, ..y) = (1, 2, ..z)}", [2, 14], Some(Code));
        test_mode("= #1.1", [2, 3], Some(Code));

        // Math
        test_mode("$$", [0], Some(Math)); // Opening dollar is math
        test_mode("$$", [1], Some(Markup)); // Closing dollar is parent
        test_mode("#$$", [2], Some(Code)); // Closing dollar is parent
        test_mode("$ a b $", [1, 3, 5], Some(Math)); // Just the spaces
        test_mode("$\na\nb\n$", [1, 3, 5], Some(Math)); // Just the newlines
        test_mode("$arrow$", [1], Some(Math));
        test_mode("$123.32$", [1, 2, 4, 5], Some(Math));
        test_mode("$+12 * y!", [1, 5, 8], Some(Math));
        test_mode("$1/2$", [2], Some(Math));
        test_mode("$f''$", [2], Some(Math));
        test_mode("$f_(x)^y$", [2, 3, 6, 7], Some(Math));
        test_mode("$a>=b$", [2], Some(Math));
        test_mode("$√x$", [1, 4], Some(Math));
        test_mode("$&x$", [1], Some(Math));
        test_mode("$\\#\\u{41}$", [1, 2, 3, 4, 5, 6], Some(Math));
        test_mode("$ff(x, sin(y), abs(z))$", [3, 4, 5, 7, 10, 15, 18], Some(Math));
        test_mode("$ff(..args, named: key)$", [4, 6, 16, 17], Some(Math));
        test_mode("$arrow.r$", [6], Some(Math));

        // Raw text
        test_mode("`r`", [0, 1], None);
        test_mode("`r`", [2], Some(Markup));
        test_mode("` \n r\n `", [1, 2, 3, 4, 5, 6], None);
        test_mode("#`r`", [1, 2], None);
        test_mode("#`r`", [3], Some(Code));
        test_mode("```l r\n```", [7], Some(Markup));
        test_mode("```l r\n```", [0, 3, 4, 5, 6], None);
        test_mode("#```l r\n```", [8], Some(Code));
        test_mode("#```l r\n```", [1, 4, 5, 6, 7], None);

        // Edge cases with embedded code expressions
        test_mode("#<l>", [1, 2, 3], Some(Code));
        test_mode("$#pa$", [2, 3], Some(Code));
        test_mode("$#{x}$", [2], Some(Code));
        test_mode("$#[x]$", [1, 4], Some(Code));
        test_mode("$#[x]$", [2, 3], Some(Markup));
        test_mode("$#f(x, ..args, named: key)$", [3, 4, 5, 7, 9, 19, 20], Some(Code));
        test_mode("$#context 1$", [9, 10], Some(Code));
        test_mode("$#context $", [9], Some(Code));
        test_mode("$#std.align$", [2, 5, 6], Some(Code));
        test_mode("$ff(named: #ident)$", [12], Some(Code));
        test_mode("$ #$x$; $", [0, 1, 3, 4, 6, 7], Some(Math));
        test_mode("$ #$x$; $", [8], Some(Markup));
        test_mode("$ #$x$; $", [2, 5], Some(Code));
        test_mode("#[$x$]", [2, 3], Some(Math));
        test_mode("#[$x$]", [1, 4], Some(Markup));
    }
}
