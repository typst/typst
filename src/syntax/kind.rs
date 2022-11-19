use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::geom::{AbsUnit, AngleUnit};
use crate::util::EcoString;

/// All syntactical building blocks that can be part of a Typst document.
///
/// Can be emitted as a token by the tokenizer or as part of a syntax node by
/// the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
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
    /// parameter list, or between the term and body of a description list
    /// term: `:`.
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
    /// The division operator, start of description list items, and fraction
    /// operator in a formula: `/`.
    Slash,
    /// The superscript operator in a formula: `^`.
    Hat,
    /// The alignment operator in a formula: `&`.
    Amp,
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
    /// The `from` keyword.
    From,

    /// Markup of which all lines must have a minimal indentation.
    ///
    /// Notably, the number does not determine in which column the markup
    /// started, but to the right of which column all markup elements must be,
    /// so it is zero except inside indent-aware constructs like lists.
    Markup { min_indent: usize },
    /// Plain text without markup.
    Text(EcoString),
    /// A forced line break: `\`.
    Linebreak,
    /// An escape sequence: `\#`, `\u{1F5FA}`.
    Escape(char),
    /// A shorthand for a unicode codepoint. For example, `~` for non-breaking
    /// space or `-?` for a soft hyphen.
    Shorthand(char),
    /// A smart quote: `'` or `"`.
    SmartQuote { double: bool },
    /// Strong content: `*Strong*`.
    Strong,
    /// Emphasized content: `_Emphasized_`.
    Emph,
    /// Raw text with optional syntax highlighting: `` `...` ``.
    Raw(Arc<RawFields>),
    /// A hyperlink: `https://typst.org`.
    Link(EcoString),
    /// A label: `<label>`.
    Label(EcoString),
    /// A reference: `@target`.
    Ref(EcoString),
    /// A section heading: `= Introduction`.
    Heading,
    /// An item in an unordered list: `- ...`.
    ListItem,
    /// An item in an enumeration (ordered list): `+ ...` or `1. ...`.
    EnumItem,
    /// An explicit enumeration numbering: `23.`.
    EnumNumbering(usize),
    /// An item in a description list: `/ Term: Details`.
    DescItem,
    /// A mathematical formula: `$x$`, `$ x^2 $`.
    Math,
    /// An atom in a formula: `x`, `+`, `12`.
    Atom(EcoString),
    /// A base with optional sub- and superscripts in a formula: `a_1^2`.
    Script,
    /// A fraction in a formula: `x/2`.
    Frac,
    /// An alignment indicator in a formula: `&`, `&&`.
    Align,

    /// An identifier: `it`.
    Ident(EcoString),
    /// A boolean: `true`, `false`.
    Bool(bool),
    /// An integer: `120`.
    Int(i64),
    /// A floating-point number: `1.2`, `10e-4`.
    Float(f64),
    /// A numeric value with a unit: `12pt`, `3cm`, `2em`, `90deg`, `50%`.
    Numeric(f64, Unit),
    /// A quoted string: `"..."`.
    Str(EcoString),
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
    Error(ErrorPos, EcoString),
}

/// Fields of a [`Raw`](NodeKind::Raw) node.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct RawFields {
    /// An optional identifier specifying the language to syntax-highlight in.
    pub lang: Option<EcoString>,
    /// The raw text, determined as the raw string between the backticks trimmed
    /// according to the above rules.
    pub text: EcoString,
    /// Whether the element is block-level, that is, it has 3+ backticks
    /// and contains at least one newline.
    pub block: bool,
}

/// Unit of a numeric value.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Unit {
    /// An absolute length unit.
    Length(AbsUnit),
    /// An angular unit.
    Angle(AngleUnit),
    /// Font-relative: `1em` is the same as the font size.
    Em,
    /// Fractions: `fr`.
    Fr,
    /// Percentage: `%`.
    Percent,
}

/// Where in a node an error should be annotated,
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ErrorPos {
    /// Over the full width of the node.
    Full,
    /// At the start of the node.
    Start,
    /// At the end of the node.
    End,
}

impl NodeKind {
    /// Whether this is trivia.
    pub fn is_trivia(&self) -> bool {
        self.is_space() || matches!(self, Self::LineComment | Self::BlockComment)
    }

    /// Whether this is a space.
    pub fn is_space(&self) -> bool {
        matches!(self, Self::Space { .. })
    }

    /// Whether this is a left or right parenthesis.
    pub fn is_paren(&self) -> bool {
        matches!(self, Self::LeftParen | Self::RightParen)
    }

    /// Whether this is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, NodeKind::Error(_, _))
    }

    /// A human-readable name for the kind.
    pub fn name(&self) -> &'static str {
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
            Self::SmartQuote { double: false } => "single quote",
            Self::SmartQuote { double: true } => "double quote",
            Self::Plus => "plus",
            Self::Minus => "minus",
            Self::Slash => "slash",
            Self::Hat => "hat",
            Self::Amp => "ampersand",
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
            Self::From => "keyword `from`",
            Self::Markup { .. } => "markup",
            Self::Text(_) => "text",
            Self::Linebreak => "linebreak",
            Self::Escape(_) => "escape sequence",
            Self::Shorthand(_) => "shorthand",
            Self::Strong => "strong content",
            Self::Emph => "emphasized content",
            Self::Raw(_) => "raw block",
            Self::Link(_) => "link",
            Self::Label(_) => "label",
            Self::Ref(_) => "reference",
            Self::Heading => "heading",
            Self::ListItem => "list item",
            Self::EnumItem => "enumeration item",
            Self::EnumNumbering(_) => "enumeration item numbering",
            Self::DescItem => "description list item",
            Self::Math => "math formula",
            Self::Atom(_) => "math atom",
            Self::Script => "script",
            Self::Frac => "fraction",
            Self::Align => "alignment indicator",
            Self::Ident(_) => "identifier",
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Numeric(_, _) => "numeric value",
            Self::Str(_) => "string",
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
            Self::Error(_, _) => "syntax error",
        }
    }
}

impl Hash for NodeKind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::LineComment => {}
            Self::BlockComment => {}
            Self::Space { newlines } => newlines.hash(state),
            Self::LeftBrace => {}
            Self::RightBrace => {}
            Self::LeftBracket => {}
            Self::RightBracket => {}
            Self::LeftParen => {}
            Self::RightParen => {}
            Self::Comma => {}
            Self::Semicolon => {}
            Self::Colon => {}
            Self::Star => {}
            Self::Underscore => {}
            Self::Dollar => {}
            Self::Plus => {}
            Self::Minus => {}
            Self::Slash => {}
            Self::Hat => {}
            Self::Amp => {}
            Self::Dot => {}
            Self::Eq => {}
            Self::EqEq => {}
            Self::ExclEq => {}
            Self::Lt => {}
            Self::LtEq => {}
            Self::Gt => {}
            Self::GtEq => {}
            Self::PlusEq => {}
            Self::HyphEq => {}
            Self::StarEq => {}
            Self::SlashEq => {}
            Self::Dots => {}
            Self::Arrow => {}
            Self::Not => {}
            Self::And => {}
            Self::Or => {}
            Self::None => {}
            Self::Auto => {}
            Self::Let => {}
            Self::Set => {}
            Self::Show => {}
            Self::If => {}
            Self::Else => {}
            Self::For => {}
            Self::In => {}
            Self::While => {}
            Self::Break => {}
            Self::Continue => {}
            Self::Return => {}
            Self::Import => {}
            Self::Include => {}
            Self::From => {}
            Self::Markup { min_indent } => min_indent.hash(state),
            Self::Text(s) => s.hash(state),
            Self::Linebreak => {}
            Self::Escape(c) => c.hash(state),
            Self::Shorthand(c) => c.hash(state),
            Self::SmartQuote { double } => double.hash(state),
            Self::Strong => {}
            Self::Emph => {}
            Self::Raw(raw) => raw.hash(state),
            Self::Link(link) => link.hash(state),
            Self::Label(c) => c.hash(state),
            Self::Ref(c) => c.hash(state),
            Self::Heading => {}
            Self::ListItem => {}
            Self::EnumItem => {}
            Self::EnumNumbering(num) => num.hash(state),
            Self::DescItem => {}
            Self::Math => {}
            Self::Atom(c) => c.hash(state),
            Self::Script => {}
            Self::Frac => {}
            Self::Align => {}
            Self::Ident(v) => v.hash(state),
            Self::Bool(v) => v.hash(state),
            Self::Int(v) => v.hash(state),
            Self::Float(v) => v.to_bits().hash(state),
            Self::Numeric(v, u) => (v.to_bits(), u).hash(state),
            Self::Str(v) => v.hash(state),
            Self::CodeBlock => {}
            Self::ContentBlock => {}
            Self::Parenthesized => {}
            Self::Array => {}
            Self::Dict => {}
            Self::Named => {}
            Self::Keyed => {}
            Self::Unary => {}
            Self::Binary => {}
            Self::FieldAccess => {}
            Self::FuncCall => {}
            Self::MethodCall => {}
            Self::Args => {}
            Self::Spread => {}
            Self::Closure => {}
            Self::Params => {}
            Self::LetBinding => {}
            Self::SetRule => {}
            Self::ShowRule => {}
            Self::Conditional => {}
            Self::WhileLoop => {}
            Self::ForLoop => {}
            Self::ForPattern => {}
            Self::ModuleImport => {}
            Self::ImportItems => {}
            Self::ModuleInclude => {}
            Self::LoopBreak => {}
            Self::LoopContinue => {}
            Self::FuncReturn => {}
            Self::Error(pos, msg) => (pos, msg).hash(state),
        }
    }
}
