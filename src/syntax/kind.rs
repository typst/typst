use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::ast::{RawNode, Unit};
use super::SpanPos;
use crate::util::EcoString;

/// All syntactical building blocks that can be part of a Typst document.
///
/// Can be emitted as a token by the tokenizer or as part of a syntax node by
/// the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    /// A line comment, two slashes followed by inner contents, terminated with
    /// a newline: `//<str>\n`.
    LineComment,
    /// A block comment, a slash and a star followed by inner contents,
    /// terminated with a star and a slash: `/*<str>*/`.
    ///
    /// The comment can contain nested block comments.
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
    /// A colon between name / key and value in a dictionary, argument or
    /// parameter list, or between the term and body of a description list
    /// term: `:`.
    Colon,
    /// The strong text toggle, multiplication operator, and wildcard import
    /// symbol: `*`.
    Star,
    /// Toggles emphasized text and indicates a subscript in a formula: `_`.
    Underscore,
    /// Starts and ends a math formula.
    Dollar,
    /// A forced line break: `\`.
    Backslash,
    /// The non-breaking space: `~`.
    Tilde,
    /// The soft hyphen: `-?`.
    HyphQuest,
    /// The en-dash: `--`.
    Hyph2,
    /// The em-dash: `---`.
    Hyph3,
    /// The ellipsis: `...`.
    Dot3,
    /// A smart quote: `'` or `"`.
    Quote { double: bool },
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
    /// The `wrap` keyword.
    Wrap,
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
    /// The `as` keyword.
    As,

    /// Markup of which all lines must have a minimal indentation.
    ///
    /// Notably, the number does not determine in which column the markup
    /// started, but to the right of which column all markup elements must be,
    /// so it is zero except inside indent-aware constructs like lists.
    Markup { min_indent: usize },
    /// Consecutive text without markup.
    Text(EcoString),
    /// A unicode escape sequence, written as a slash and the letter "u"
    /// followed by a hexadecimal unicode entity enclosed in curly braces:
    /// `\u{1F5FA}`.
    Escape(char),
    /// Strong content: `*Strong*`.
    Strong,
    /// Emphasized content: `_Emphasized_`.
    Emph,
    /// A hyperlink: `https://typst.org`.
    Link(EcoString),
    /// A raw block with optional syntax highlighting: `` `...` ``.
    Raw(Arc<RawNode>),
    /// A math formula: `$x$`, `$ x^2 $`.
    Math,
    /// An atom in a math formula: `x`, `+`, `12`.
    Atom(EcoString),
    /// A base with optional sub- and superscript in a math formula: `a_1^2`.
    Script,
    /// A fraction in a math formula: `x/2`.
    Frac,
    /// An alignment indicator in a math formula: `&`, `&&`.
    Align,
    /// A section heading: `= Introduction`.
    Heading,
    /// An item in an unordered list: `- ...`.
    ListItem,
    /// An item in an enumeration (ordered list): `+ ...` or `1. ...`.
    EnumItem,
    /// An explicit enumeration numbering: `23.`.
    EnumNumbering(usize),
    /// An item in a description list: `/ Term: Details.
    DescItem,
    /// A label: `<label>`.
    Label(EcoString),
    /// A reference: `@label`.
    Ref(EcoString),

    /// An identifier: `center`.
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
    GroupExpr,
    /// An array expression: `(1, "hi", 12cm)`.
    ArrayExpr,
    /// A dictionary expression: `(thickness: 3pt, pattern: dashed)`.
    DictExpr,
    /// A named pair: `thickness: 3pt`.
    Named,
    /// A keyed pair: `"spacy key": true`.
    Keyed,
    /// A unary operation: `-x`.
    UnaryExpr,
    /// A binary operation: `a + b`.
    BinaryExpr,
    /// A field access: `properties.age`.
    FieldAccess,
    /// An invocation of a function: `f(x, y)`.
    FuncCall,
    /// An invocation of a method: `array.push(v)`.
    MethodCall,
    /// A function call's argument list: `(x, y)`.
    CallArgs,
    /// Spreaded arguments or a argument sink: `..x`.
    Spread,
    /// A closure expression: `(x, y) => z`.
    ClosureExpr,
    /// A closure's parameters: `(x, y)`.
    ClosureParams,
    /// A let expression: `let x = 1`.
    LetExpr,
    /// A set expression: `set text(...)`.
    SetExpr,
    /// A show expression: `show node: heading as [*{nody.body}*]`.
    ShowExpr,
    /// A wrap expression: `wrap body in columns(2, body)`.
    WrapExpr,
    /// An if-else expression: `if x { y } else { z }`.
    IfExpr,
    /// A while loop expression: `while x { ... }`.
    WhileExpr,
    /// A for loop expression: `for x in y { ... }`.
    ForExpr,
    /// A for loop's destructuring pattern: `x` or `x, y`.
    ForPattern,
    /// An import expression: `import a, b, c from "utils.typ"`.
    ImportExpr,
    /// Items to import: `a, b, c`.
    ImportItems,
    /// An include expression: `include "chapter1.typ"`.
    IncludeExpr,
    /// A break expression: `break`.
    BreakExpr,
    /// A continue expression: `continue`.
    ContinueExpr,
    /// A return expression: `return x + 1`.
    ReturnExpr,

    /// An invalid sequence of characters.
    Error(SpanPos, EcoString),
}

impl NodeKind {
    /// Whether this is a kind of parenthesis.
    pub fn is_paren(&self) -> bool {
        matches!(self, Self::LeftParen | Self::RightParen)
    }

    /// Whether this is a space.
    pub fn is_space(&self) -> bool {
        matches!(self, Self::Space { .. })
    }

    /// Whether this is trivia.
    pub fn is_trivia(&self) -> bool {
        self.is_space() || matches!(self, Self::LineComment | Self::BlockComment)
    }

    /// Whether this is a kind of error.
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
            Self::Backslash => "linebreak",
            Self::Tilde => "non-breaking space",
            Self::HyphQuest => "soft hyphen",
            Self::Hyph2 => "en dash",
            Self::Hyph3 => "em dash",
            Self::Dot3 => "ellipsis",
            Self::Quote { double: false } => "single quote",
            Self::Quote { double: true } => "double quote",
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
            Self::Wrap => "keyword `wrap`",
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
            Self::As => "keyword `as`",
            Self::Markup { .. } => "markup",
            Self::Text(_) => "text",
            Self::Escape(_) => "escape sequence",
            Self::Strong => "strong content",
            Self::Emph => "emphasized content",
            Self::Link(_) => "link",
            Self::Raw(_) => "raw block",
            Self::Math => "math formula",
            Self::Atom(_) => "math atom",
            Self::Script => "script",
            Self::Frac => "fraction",
            Self::Align => "alignment indicator",
            Self::Heading => "heading",
            Self::ListItem => "list item",
            Self::EnumItem => "enumeration item",
            Self::EnumNumbering(_) => "enumeration item numbering",
            Self::DescItem => "description list item",
            Self::Label(_) => "label",
            Self::Ref(_) => "reference",
            Self::Ident(_) => "identifier",
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Numeric(_, _) => "numeric value",
            Self::Str(_) => "string",
            Self::CodeBlock => "code block",
            Self::ContentBlock => "content block",
            Self::GroupExpr => "group",
            Self::ArrayExpr => "array",
            Self::DictExpr => "dictionary",
            Self::Named => "named pair",
            Self::Keyed => "keyed pair",
            Self::UnaryExpr => "unary expression",
            Self::BinaryExpr => "binary expression",
            Self::FieldAccess => "field access",
            Self::FuncCall => "function call",
            Self::MethodCall => "method call",
            Self::CallArgs => "call arguments",
            Self::Spread => "spread",
            Self::ClosureExpr => "closure",
            Self::ClosureParams => "closure parameters",
            Self::LetExpr => "`let` expression",
            Self::SetExpr => "`set` expression",
            Self::ShowExpr => "`show` expression",
            Self::WrapExpr => "`wrap` expression",
            Self::IfExpr => "`if` expression",
            Self::WhileExpr => "while-loop expression",
            Self::ForExpr => "for-loop expression",
            Self::ForPattern => "for-loop destructuring pattern",
            Self::ImportExpr => "`import` expression",
            Self::ImportItems => "import items",
            Self::IncludeExpr => "`include` expression",
            Self::BreakExpr => "`break` expression",
            Self::ContinueExpr => "`continue` expression",
            Self::ReturnExpr => "`return` expression",
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
            Self::Backslash => {}
            Self::Tilde => {}
            Self::HyphQuest => {}
            Self::Hyph2 => {}
            Self::Hyph3 => {}
            Self::Dot3 => {}
            Self::Quote { double } => double.hash(state),
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
            Self::Wrap => {}
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
            Self::As => {}
            Self::Markup { min_indent } => min_indent.hash(state),
            Self::Text(s) => s.hash(state),
            Self::Escape(c) => c.hash(state),
            Self::Strong => {}
            Self::Emph => {}
            Self::Link(link) => link.hash(state),
            Self::Raw(raw) => raw.hash(state),
            Self::Math => {}
            Self::Atom(c) => c.hash(state),
            Self::Script => {}
            Self::Frac => {}
            Self::Align => {}
            Self::Heading => {}
            Self::ListItem => {}
            Self::EnumItem => {}
            Self::EnumNumbering(num) => num.hash(state),
            Self::DescItem => {}
            Self::Label(c) => c.hash(state),
            Self::Ref(c) => c.hash(state),
            Self::Ident(v) => v.hash(state),
            Self::Bool(v) => v.hash(state),
            Self::Int(v) => v.hash(state),
            Self::Float(v) => v.to_bits().hash(state),
            Self::Numeric(v, u) => (v.to_bits(), u).hash(state),
            Self::Str(v) => v.hash(state),
            Self::CodeBlock => {}
            Self::ContentBlock => {}
            Self::GroupExpr => {}
            Self::ArrayExpr => {}
            Self::DictExpr => {}
            Self::Named => {}
            Self::Keyed => {}
            Self::UnaryExpr => {}
            Self::BinaryExpr => {}
            Self::FieldAccess => {}
            Self::FuncCall => {}
            Self::MethodCall => {}
            Self::CallArgs => {}
            Self::Spread => {}
            Self::ClosureExpr => {}
            Self::ClosureParams => {}
            Self::LetExpr => {}
            Self::SetExpr => {}
            Self::ShowExpr => {}
            Self::WrapExpr => {}
            Self::IfExpr => {}
            Self::WhileExpr => {}
            Self::ForExpr => {}
            Self::ForPattern => {}
            Self::ImportExpr => {}
            Self::ImportItems => {}
            Self::IncludeExpr => {}
            Self::BreakExpr => {}
            Self::ContinueExpr => {}
            Self::ReturnExpr => {}
            Self::Error(pos, msg) => (pos, msg).hash(state),
        }
    }
}
