/// All syntactical building blocks that can be part of a Typst document.
///
/// Can be created by the lexer or by the parser.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SyntaxKind {
    /// A line comment: `// ...`.
    LineComment,
    /// A block comment: `/* ... */`.
    BlockComment,
    /// One or more whitespace characters. Single spaces are collapsed into text
    /// nodes if they would otherwise be surrounded by text nodes.
    ///
    /// Also stores how many newlines are contained.
    Space { newlines: usize },

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
    /// Toggles emphasized text and indicates a subscript in a formula: `_`.
    Underscore,
    /// Starts and ends a math formula: `$`.
    Dollar,
    /// The unary plus, binary addition operator, and start of enum items: `+`.
    Plus,
    /// The unary negation, binary subtraction operator, and start of list
    /// items: `-`.
    Minus,
    /// The division operator, start of term list items, and fraction operator
    /// in a formula: `/`.
    Slash,
    /// The superscript operator in a formula: `^`.
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
    /// The spread operator: `..`.
    Dots,
    /// An arrow between a closure's parameters and body: `=>`.
    Arrow,

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

    /// Markup of which all lines must have a minimal indentation.
    ///
    /// Notably, the number does not determine in which column the markup
    /// started, but to the right of which column all markup elements must be,
    /// so it is zero except inside indent-aware constructs like lists.
    Markup { min_indent: usize },
    /// Plain text without markup.
    Text,
    /// A forced line break: `\`.
    Linebreak,
    /// An escape sequence: `\#`, `\u{1F5FA}`.
    Escape,
    /// A shorthand for a unicode codepoint. For example, `~` for non-breaking
    /// space or `-?` for a soft hyphen.
    Shorthand,
    /// Symbol notation: `:arrow:l:`. The string only contains the inner part
    /// without leading and trailing dot.
    Symbol,
    /// A smart quote: `'` or `"`.
    SmartQuote,
    /// Strong content: `*Strong*`.
    Strong,
    /// Emphasized content: `_Emphasized_`.
    Emph,
    /// Raw text with optional syntax highlighting: `` `...` ``.
    Raw { column: usize },
    /// A hyperlink: `https://typst.org`.
    Link,
    /// A label: `<intro>`.
    Label,
    /// A reference: `@target`.
    Ref,
    /// A section heading: `= Introduction`.
    Heading,
    /// An item in a bullet list: `- ...`.
    ListItem,
    /// An item in an enumeration (numbered list): `+ ...` or `1. ...`.
    EnumItem,
    /// An explicit enumeration numbering: `23.`.
    EnumNumbering,
    /// An item in a term list: `/ Term: Details`.
    TermItem,
    /// A mathematical formula: `$x$`, `$ x^2 $`.
    Math,
    /// An atom in a formula: `x`, `+`, `12`.
    Atom,
    /// A base with optional sub- and superscripts in a formula: `a_1^2`.
    Script,
    /// A fraction in a formula: `x/2`.
    Frac,
    /// An alignment point in a formula: `&`.
    AlignPoint,

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
    /// A dictionary: `(thickness: 3pt, pattern: dashed)`.
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
    /// An invocation of a function: `f(x, y)`.
    FuncCall,
    /// An invocation of a method: `array.push(v)`.
    MethodCall,
    /// A function call's argument list: `(12pt, y)`.
    Args,
    /// Spreaded arguments or an argument sink: `..x`.
    Spread,
    /// A closure: `(x, y) => z`.
    Closure,
    /// A closure's parameters: `(x, y)`.
    Params,
    /// A let binding: `let x = 1`.
    LetBinding,
    /// A set rule: `set text(...)`.
    SetRule,
    /// A show rule: `show heading: it => [*{it.body}*]`.
    ShowRule,
    /// An if-else conditional: `if x { y } else { z }`.
    Conditional,
    /// A while loop: `while x { y }`.
    WhileLoop,
    /// A for loop: `for x in y { z }`.
    ForLoop,
    /// A for loop's destructuring pattern: `x` or `x, y`.
    ForPattern,
    /// A module import: `import a, b, c from "utils.typ"`.
    ModuleImport,
    /// Items to import from a module: `a, b, c`.
    ImportItems,
    /// A module include: `include "chapter1.typ"`.
    ModuleInclude,
    /// A break from a loop: `break`.
    LoopBreak,
    /// A continue in a loop: `continue`.
    LoopContinue,
    /// A return from a function: `return`, `return x + 1`.
    FuncReturn,

    /// An invalid sequence of characters.
    Error,
}

impl SyntaxKind {
    /// Whether this is trivia.
    pub fn is_trivia(self) -> bool {
        self.is_space() || self.is_comment() || self.is_error()
    }

    /// Whether this is a space.
    pub fn is_space(self) -> bool {
        matches!(self, Self::Space { .. })
    }

    /// Whether this is a comment.
    pub fn is_comment(self) -> bool {
        matches!(self, Self::LineComment | Self::BlockComment)
    }

    /// Whether this is an error.
    pub fn is_error(self) -> bool {
        matches!(self, SyntaxKind::Error)
    }

    /// Whether this is a left or right parenthesis.
    pub fn is_paren(self) -> bool {
        matches!(self, Self::LeftParen | Self::RightParen)
    }

    /// Does this node need termination through a semicolon or linebreak?
    pub fn is_stmt(self) -> bool {
        matches!(
            self,
            SyntaxKind::LetBinding
                | SyntaxKind::SetRule
                | SyntaxKind::ShowRule
                | SyntaxKind::ModuleImport
                | SyntaxKind::ModuleInclude
        )
    }

    /// A human-readable name for the kind.
    pub fn name(self) -> &'static str {
        match self {
            Self::LineComment => "line comment",
            Self::BlockComment => "block comment",
            Self::Space { .. } => "space",
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
            Self::SmartQuote => "smart quote",
            Self::Plus => "plus",
            Self::Minus => "minus",
            Self::Slash => "slash",
            Self::Hat => "hat",
            Self::Dot => "dot",
            Self::Eq => "assignment operator",
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
            Self::Not => "operator `not`",
            Self::And => "operator `and`",
            Self::Or => "operator `or`",
            Self::None => "`none`",
            Self::Auto => "`auto`",
            Self::Let => "keyword `let`",
            Self::Set => "keyword `set`",
            Self::Show => "keyword `show`",
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
            Self::Markup { .. } => "markup",
            Self::Text => "text",
            Self::Linebreak => "linebreak",
            Self::Escape => "escape sequence",
            Self::Shorthand => "shorthand",
            Self::Symbol => "symbol notation",
            Self::Strong => "strong content",
            Self::Emph => "emphasized content",
            Self::Raw { .. } => "raw block",
            Self::Link => "link",
            Self::Label => "label",
            Self::Ref => "reference",
            Self::Heading => "heading",
            Self::ListItem => "list item",
            Self::EnumItem => "enumeration item",
            Self::EnumNumbering => "enumeration item numbering",
            Self::TermItem => "term list item",
            Self::Math => "math formula",
            Self::Atom => "math atom",
            Self::Script => "script",
            Self::Frac => "fraction",
            Self::AlignPoint => "alignment point",
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
            Self::MethodCall => "method call",
            Self::Args => "call arguments",
            Self::Spread => "spread",
            Self::Closure => "closure",
            Self::Params => "closure parameters",
            Self::LetBinding => "`let` expression",
            Self::SetRule => "`set` expression",
            Self::ShowRule => "`show` expression",
            Self::Conditional => "`if` expression",
            Self::WhileLoop => "while-loop expression",
            Self::ForLoop => "for-loop expression",
            Self::ForPattern => "for-loop destructuring pattern",
            Self::ModuleImport => "`import` expression",
            Self::ImportItems => "import items",
            Self::ModuleInclude => "`include` expression",
            Self::LoopBreak => "`break` expression",
            Self::LoopContinue => "`continue` expression",
            Self::FuncReturn => "`return` expression",
            Self::Error => "syntax error",
        }
    }
}
