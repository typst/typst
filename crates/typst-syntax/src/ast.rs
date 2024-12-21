//! A typed layer over the untyped syntax tree.
//!
//! The AST is rooted in the [`Markup`] node.

use std::num::NonZeroUsize;
use std::ops::Deref;

use ecow::EcoString;
use unscanny::Scanner;

use crate::{is_newline, Span, SyntaxKind, SyntaxNode};

/// A typed AST node.
pub trait AstNode<'a>: Sized {
    /// Convert a node into its typed variant.
    fn from_untyped(node: &'a SyntaxNode) -> Option<Self>;

    /// A reference to the underlying syntax node.
    fn to_untyped(self) -> &'a SyntaxNode;

    /// The source code location.
    fn span(self) -> Span {
        self.to_untyped().span()
    }
}

macro_rules! node {
    ($(#[$attr:meta])* $name:ident) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
        #[repr(transparent)]
        $(#[$attr])*
        pub struct $name<'a>(&'a SyntaxNode);

        impl<'a> AstNode<'a> for $name<'a> {
            #[inline]
            fn from_untyped(node: &'a SyntaxNode) -> Option<Self> {
                if node.kind() == SyntaxKind::$name {
                    Some(Self(node))
                } else {
                    Option::None
                }
            }

            #[inline]
            fn to_untyped(self) -> &'a SyntaxNode {
                self.0
            }
        }

        impl Default for $name<'_> {
            #[inline]
            fn default() -> Self {
                static PLACEHOLDER: SyntaxNode
                    = SyntaxNode::placeholder(SyntaxKind::$name);
                Self(&PLACEHOLDER)
            }
        }
    };
}

node! {
    /// The syntactical root capable of representing a full parsed document.
    Markup
}

impl<'a> Markup<'a> {
    /// The expressions.
    pub fn exprs(self) -> impl DoubleEndedIterator<Item = Expr<'a>> {
        let mut was_stmt = false;
        self.0
            .children()
            .filter(move |node| {
                // Ignore newline directly after statements without semicolons.
                let kind = node.kind();
                let keep = !was_stmt || node.kind() != SyntaxKind::Space;
                was_stmt = kind.is_stmt();
                keep
            })
            .filter_map(Expr::cast_with_space)
    }
}

/// An expression in markup, math or code.
#[derive(Debug, Copy, Clone, Hash)]
pub enum Expr<'a> {
    /// Plain text without markup.
    Text(Text<'a>),
    /// Whitespace in markup or math. Has at most one newline in markup, as more
    /// indicate a paragraph break.
    Space(Space<'a>),
    /// A forced line break: `\`.
    Linebreak(Linebreak<'a>),
    /// A paragraph break, indicated by one or multiple blank lines.
    Parbreak(Parbreak<'a>),
    /// An escape sequence: `\#`, `\u{1F5FA}`.
    Escape(Escape<'a>),
    /// A shorthand for a unicode codepoint. For example, `~` for non-breaking
    /// space or `-?` for a soft hyphen.
    Shorthand(Shorthand<'a>),
    /// A smart quote: `'` or `"`.
    SmartQuote(SmartQuote<'a>),
    /// Strong content: `*Strong*`.
    Strong(Strong<'a>),
    /// Emphasized content: `_Emphasized_`.
    Emph(Emph<'a>),
    /// Raw text with optional syntax highlighting: `` `...` ``.
    Raw(Raw<'a>),
    /// A hyperlink: `https://typst.org`.
    Link(Link<'a>),
    /// A label: `<intro>`.
    Label(Label<'a>),
    /// A reference: `@target`, `@target[..]`.
    Ref(Ref<'a>),
    /// A section heading: `= Introduction`.
    Heading(Heading<'a>),
    /// An item in a bullet list: `- ...`.
    List(ListItem<'a>),
    /// An item in an enumeration (numbered list): `+ ...` or `1. ...`.
    Enum(EnumItem<'a>),
    /// An item in a term list: `/ Term: Details`.
    Term(TermItem<'a>),
    /// A mathematical equation: `$x$`, `$ x^2 $`.
    Equation(Equation<'a>),
    /// The contents of a mathematical equation: `x^2 + 1`.
    Math(Math<'a>),
    /// An identifier in math: `pi`.
    MathIdent(MathIdent<'a>),
    /// A shorthand for a unicode codepoint in math: `a <= b`.
    MathShorthand(MathShorthand<'a>),
    /// An alignment point in math: `&`.
    MathAlignPoint(MathAlignPoint<'a>),
    /// Matched delimiters in math: `[x + y]`.
    MathDelimited(MathDelimited<'a>),
    /// A base with optional attachments in math: `a_1^2`.
    MathAttach(MathAttach<'a>),
    /// Grouped math primes
    MathPrimes(MathPrimes<'a>),
    /// A fraction in math: `x/2`.
    MathFrac(MathFrac<'a>),
    /// A root in math: `√x`, `∛x` or `∜x`.
    MathRoot(MathRoot<'a>),
    /// An identifier: `left`.
    Ident(Ident<'a>),
    /// The `none` literal.
    None(None<'a>),
    /// The `auto` literal.
    Auto(Auto<'a>),
    /// A boolean: `true`, `false`.
    Bool(Bool<'a>),
    /// An integer: `120`.
    Int(Int<'a>),
    /// A floating-point number: `1.2`, `10e-4`.
    Float(Float<'a>),
    /// A numeric value with a unit: `12pt`, `3cm`, `2em`, `90deg`, `50%`.
    Numeric(Numeric<'a>),
    /// A quoted string: `"..."`.
    Str(Str<'a>),
    /// A code block: `{ let x = 1; x + 2 }`.
    Code(CodeBlock<'a>),
    /// A content block: `[*Hi* there!]`.
    Content(ContentBlock<'a>),
    /// A grouped expression: `(1 + 2)`.
    Parenthesized(Parenthesized<'a>),
    /// An array: `(1, "hi", 12cm)`.
    Array(Array<'a>),
    /// A dictionary: `(thickness: 3pt, dash: "solid")`.
    Dict(Dict<'a>),
    /// A unary operation: `-x`.
    Unary(Unary<'a>),
    /// A binary operation: `a + b`.
    Binary(Binary<'a>),
    /// A field access: `properties.age`.
    FieldAccess(FieldAccess<'a>),
    /// An invocation of a function or method: `f(x, y)`.
    FuncCall(FuncCall<'a>),
    /// A closure: `(x, y) => z`.
    Closure(Closure<'a>),
    /// A let binding: `let x = 1`.
    Let(LetBinding<'a>),
    /// A destructuring assignment: `(x, y) = (1, 2)`.
    DestructAssign(DestructAssignment<'a>),
    /// A set rule: `set text(...)`.
    Set(SetRule<'a>),
    /// A show rule: `show heading: it => emph(it.body)`.
    Show(ShowRule<'a>),
    /// A contextual expression: `context text.lang`.
    Contextual(Contextual<'a>),
    /// An if-else conditional: `if x { y } else { z }`.
    Conditional(Conditional<'a>),
    /// A while loop: `while x { y }`.
    While(WhileLoop<'a>),
    /// A for loop: `for x in y { z }`.
    For(ForLoop<'a>),
    /// A module import: `import "utils.typ": a, b, c`.
    Import(ModuleImport<'a>),
    /// A module include: `include "chapter1.typ"`.
    Include(ModuleInclude<'a>),
    /// A break from a loop: `break`.
    Break(LoopBreak<'a>),
    /// A continue in a loop: `continue`.
    Continue(LoopContinue<'a>),
    /// A return from a function: `return`, `return x + 1`.
    Return(FuncReturn<'a>),
}

impl<'a> Expr<'a> {
    fn cast_with_space(node: &'a SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Space => node.cast().map(Self::Space),
            _ => Self::from_untyped(node),
        }
    }
}

impl<'a> AstNode<'a> for Expr<'a> {
    fn from_untyped(node: &'a SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Linebreak => node.cast().map(Self::Linebreak),
            SyntaxKind::Parbreak => node.cast().map(Self::Parbreak),
            SyntaxKind::Text => node.cast().map(Self::Text),
            SyntaxKind::Escape => node.cast().map(Self::Escape),
            SyntaxKind::Shorthand => node.cast().map(Self::Shorthand),
            SyntaxKind::SmartQuote => node.cast().map(Self::SmartQuote),
            SyntaxKind::Strong => node.cast().map(Self::Strong),
            SyntaxKind::Emph => node.cast().map(Self::Emph),
            SyntaxKind::Raw => node.cast().map(Self::Raw),
            SyntaxKind::Link => node.cast().map(Self::Link),
            SyntaxKind::Label => node.cast().map(Self::Label),
            SyntaxKind::Ref => node.cast().map(Self::Ref),
            SyntaxKind::Heading => node.cast().map(Self::Heading),
            SyntaxKind::ListItem => node.cast().map(Self::List),
            SyntaxKind::EnumItem => node.cast().map(Self::Enum),
            SyntaxKind::TermItem => node.cast().map(Self::Term),
            SyntaxKind::Equation => node.cast().map(Self::Equation),
            SyntaxKind::Math => node.cast().map(Self::Math),
            SyntaxKind::MathIdent => node.cast().map(Self::MathIdent),
            SyntaxKind::MathShorthand => node.cast().map(Self::MathShorthand),
            SyntaxKind::MathAlignPoint => node.cast().map(Self::MathAlignPoint),
            SyntaxKind::MathDelimited => node.cast().map(Self::MathDelimited),
            SyntaxKind::MathAttach => node.cast().map(Self::MathAttach),
            SyntaxKind::MathPrimes => node.cast().map(Self::MathPrimes),
            SyntaxKind::MathFrac => node.cast().map(Self::MathFrac),
            SyntaxKind::MathRoot => node.cast().map(Self::MathRoot),
            SyntaxKind::Ident => node.cast().map(Self::Ident),
            SyntaxKind::None => node.cast().map(Self::None),
            SyntaxKind::Auto => node.cast().map(Self::Auto),
            SyntaxKind::Bool => node.cast().map(Self::Bool),
            SyntaxKind::Int => node.cast().map(Self::Int),
            SyntaxKind::Float => node.cast().map(Self::Float),
            SyntaxKind::Numeric => node.cast().map(Self::Numeric),
            SyntaxKind::Str => node.cast().map(Self::Str),
            SyntaxKind::CodeBlock => node.cast().map(Self::Code),
            SyntaxKind::ContentBlock => node.cast().map(Self::Content),
            SyntaxKind::Parenthesized => node.cast().map(Self::Parenthesized),
            SyntaxKind::Array => node.cast().map(Self::Array),
            SyntaxKind::Dict => node.cast().map(Self::Dict),
            SyntaxKind::Unary => node.cast().map(Self::Unary),
            SyntaxKind::Binary => node.cast().map(Self::Binary),
            SyntaxKind::FieldAccess => node.cast().map(Self::FieldAccess),
            SyntaxKind::FuncCall => node.cast().map(Self::FuncCall),
            SyntaxKind::Closure => node.cast().map(Self::Closure),
            SyntaxKind::LetBinding => node.cast().map(Self::Let),
            SyntaxKind::DestructAssignment => node.cast().map(Self::DestructAssign),
            SyntaxKind::SetRule => node.cast().map(Self::Set),
            SyntaxKind::ShowRule => node.cast().map(Self::Show),
            SyntaxKind::Contextual => node.cast().map(Self::Contextual),
            SyntaxKind::Conditional => node.cast().map(Self::Conditional),
            SyntaxKind::WhileLoop => node.cast().map(Self::While),
            SyntaxKind::ForLoop => node.cast().map(Self::For),
            SyntaxKind::ModuleImport => node.cast().map(Self::Import),
            SyntaxKind::ModuleInclude => node.cast().map(Self::Include),
            SyntaxKind::LoopBreak => node.cast().map(Self::Break),
            SyntaxKind::LoopContinue => node.cast().map(Self::Continue),
            SyntaxKind::FuncReturn => node.cast().map(Self::Return),
            _ => Option::None,
        }
    }

    fn to_untyped(self) -> &'a SyntaxNode {
        match self {
            Self::Text(v) => v.to_untyped(),
            Self::Space(v) => v.to_untyped(),
            Self::Linebreak(v) => v.to_untyped(),
            Self::Parbreak(v) => v.to_untyped(),
            Self::Escape(v) => v.to_untyped(),
            Self::Shorthand(v) => v.to_untyped(),
            Self::SmartQuote(v) => v.to_untyped(),
            Self::Strong(v) => v.to_untyped(),
            Self::Emph(v) => v.to_untyped(),
            Self::Raw(v) => v.to_untyped(),
            Self::Link(v) => v.to_untyped(),
            Self::Label(v) => v.to_untyped(),
            Self::Ref(v) => v.to_untyped(),
            Self::Heading(v) => v.to_untyped(),
            Self::List(v) => v.to_untyped(),
            Self::Enum(v) => v.to_untyped(),
            Self::Term(v) => v.to_untyped(),
            Self::Equation(v) => v.to_untyped(),
            Self::Math(v) => v.to_untyped(),
            Self::MathIdent(v) => v.to_untyped(),
            Self::MathShorthand(v) => v.to_untyped(),
            Self::MathAlignPoint(v) => v.to_untyped(),
            Self::MathDelimited(v) => v.to_untyped(),
            Self::MathAttach(v) => v.to_untyped(),
            Self::MathPrimes(v) => v.to_untyped(),
            Self::MathFrac(v) => v.to_untyped(),
            Self::MathRoot(v) => v.to_untyped(),
            Self::Ident(v) => v.to_untyped(),
            Self::None(v) => v.to_untyped(),
            Self::Auto(v) => v.to_untyped(),
            Self::Bool(v) => v.to_untyped(),
            Self::Int(v) => v.to_untyped(),
            Self::Float(v) => v.to_untyped(),
            Self::Numeric(v) => v.to_untyped(),
            Self::Str(v) => v.to_untyped(),
            Self::Code(v) => v.to_untyped(),
            Self::Content(v) => v.to_untyped(),
            Self::Array(v) => v.to_untyped(),
            Self::Dict(v) => v.to_untyped(),
            Self::Parenthesized(v) => v.to_untyped(),
            Self::Unary(v) => v.to_untyped(),
            Self::Binary(v) => v.to_untyped(),
            Self::FieldAccess(v) => v.to_untyped(),
            Self::FuncCall(v) => v.to_untyped(),
            Self::Closure(v) => v.to_untyped(),
            Self::Let(v) => v.to_untyped(),
            Self::DestructAssign(v) => v.to_untyped(),
            Self::Set(v) => v.to_untyped(),
            Self::Show(v) => v.to_untyped(),
            Self::Contextual(v) => v.to_untyped(),
            Self::Conditional(v) => v.to_untyped(),
            Self::While(v) => v.to_untyped(),
            Self::For(v) => v.to_untyped(),
            Self::Import(v) => v.to_untyped(),
            Self::Include(v) => v.to_untyped(),
            Self::Break(v) => v.to_untyped(),
            Self::Continue(v) => v.to_untyped(),
            Self::Return(v) => v.to_untyped(),
        }
    }
}

impl Expr<'_> {
    /// Can this expression be embedded into markup with a hash?
    pub fn hash(self) -> bool {
        matches!(
            self,
            Self::Ident(_)
                | Self::None(_)
                | Self::Auto(_)
                | Self::Bool(_)
                | Self::Int(_)
                | Self::Float(_)
                | Self::Numeric(_)
                | Self::Str(_)
                | Self::Code(_)
                | Self::Content(_)
                | Self::Array(_)
                | Self::Dict(_)
                | Self::Parenthesized(_)
                | Self::FieldAccess(_)
                | Self::FuncCall(_)
                | Self::Let(_)
                | Self::Set(_)
                | Self::Show(_)
                | Self::Contextual(_)
                | Self::Conditional(_)
                | Self::While(_)
                | Self::For(_)
                | Self::Import(_)
                | Self::Include(_)
                | Self::Break(_)
                | Self::Continue(_)
                | Self::Return(_)
        )
    }

    /// Is this a literal?
    pub fn is_literal(self) -> bool {
        matches!(
            self,
            Self::None(_)
                | Self::Auto(_)
                | Self::Bool(_)
                | Self::Int(_)
                | Self::Float(_)
                | Self::Numeric(_)
                | Self::Str(_)
        )
    }
}

impl Default for Expr<'_> {
    fn default() -> Self {
        Expr::None(None::default())
    }
}

node! {
    /// Plain text without markup.
    Text
}

impl<'a> Text<'a> {
    /// Get the text.
    pub fn get(self) -> &'a EcoString {
        self.0.text()
    }
}

node! {
    /// Whitespace in markup or math. Has at most one newline in markup, as more
    /// indicate a paragraph break.
    Space
}

node! {
    /// A forced line break: `\`.
    Linebreak
}

node! {
    /// A paragraph break, indicated by one or multiple blank lines.
    Parbreak
}

node! {
    /// An escape sequence: `\#`, `\u{1F5FA}`.
    Escape
}

impl Escape<'_> {
    /// Get the escaped character.
    pub fn get(self) -> char {
        let mut s = Scanner::new(self.0.text());
        s.expect('\\');
        if s.eat_if("u{") {
            let hex = s.eat_while(char::is_ascii_hexdigit);
            u32::from_str_radix(hex, 16)
                .ok()
                .and_then(std::char::from_u32)
                .unwrap_or_default()
        } else {
            s.eat().unwrap_or_default()
        }
    }
}

node! {
    /// A shorthand for a unicode codepoint. For example, `~` for a non-breaking
    /// space or `-?` for a soft hyphen.
    Shorthand
}

impl Shorthand<'_> {
    /// A list of all shorthands in markup mode.
    pub const LIST: &'static [(&'static str, char)] = &[
        ("...", '…'),
        ("~", '\u{00A0}'),
        ("-", '\u{2212}'), // Only before a digit
        ("--", '\u{2013}'),
        ("---", '\u{2014}'),
        ("-?", '\u{00AD}'),
    ];

    /// Get the shorthanded character.
    pub fn get(self) -> char {
        let text = self.0.text();
        Self::LIST
            .iter()
            .find(|&&(s, _)| s == text)
            .map_or_else(char::default, |&(_, c)| c)
    }
}

node! {
    /// A smart quote: `'` or `"`.
    SmartQuote
}

impl SmartQuote<'_> {
    /// Whether this is a double quote.
    pub fn double(self) -> bool {
        self.0.text() == "\""
    }
}

node! {
    /// Strong content: `*Strong*`.
    Strong
}

impl<'a> Strong<'a> {
    /// The contents of the strong node.
    pub fn body(self) -> Markup<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// Emphasized content: `_Emphasized_`.
    Emph
}

impl<'a> Emph<'a> {
    /// The contents of the emphasis node.
    pub fn body(self) -> Markup<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// Raw text with optional syntax highlighting: `` `...` ``.
    Raw
}

impl<'a> Raw<'a> {
    /// The lines in the raw block.
    pub fn lines(self) -> impl DoubleEndedIterator<Item = Text<'a>> {
        self.0.children().filter_map(SyntaxNode::cast)
    }

    /// An optional identifier specifying the language to syntax-highlight in.
    pub fn lang(self) -> Option<RawLang<'a>> {
        // Only blocky literals are supposed to contain a language.
        let delim: RawDelim = self.0.cast_first_match()?;
        if delim.0.len() < 3 {
            return Option::None;
        }

        self.0.cast_first_match()
    }

    /// Whether the raw text should be displayed in a separate block.
    pub fn block(self) -> bool {
        self.0
            .cast_first_match()
            .is_some_and(|delim: RawDelim| delim.0.len() >= 3)
            && self.0.children().any(|e| {
                e.kind() == SyntaxKind::RawTrimmed && e.text().chars().any(is_newline)
            })
    }
}

node! {
    /// A language tag at the start of raw element: ``typ ``.
    RawLang
}

impl<'a> RawLang<'a> {
    /// Get the language tag.
    pub fn get(self) -> &'a EcoString {
        self.0.text()
    }
}

node! {
    /// A raw delimiter in single or 3+ backticks: `` ` ``.
    RawDelim
}

node! {
    /// A hyperlink: `https://typst.org`.
    Link
}

impl<'a> Link<'a> {
    /// Get the URL.
    pub fn get(self) -> &'a EcoString {
        self.0.text()
    }
}

node! {
    /// A label: `<intro>`.
    Label
}

impl<'a> Label<'a> {
    /// Get the label's text.
    pub fn get(self) -> &'a str {
        self.0.text().trim_start_matches('<').trim_end_matches('>')
    }
}

node! {
    /// A reference: `@target`, `@target[..]`.
    Ref
}

impl<'a> Ref<'a> {
    /// Get the target.
    pub fn target(self) -> &'a str {
        self.0
            .children()
            .find(|node| node.kind() == SyntaxKind::RefMarker)
            .map(|node| node.text().trim_start_matches('@'))
            .unwrap_or_default()
    }

    /// Get the supplement.
    pub fn supplement(self) -> Option<ContentBlock<'a>> {
        self.0.cast_last_match()
    }
}

node! {
    /// A section heading: `= Introduction`.
    Heading
}

impl<'a> Heading<'a> {
    /// The contents of the heading.
    pub fn body(self) -> Markup<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The section depth (number of equals signs).
    pub fn depth(self) -> NonZeroUsize {
        self.0
            .children()
            .find(|node| node.kind() == SyntaxKind::HeadingMarker)
            .and_then(|node| node.len().try_into().ok())
            .unwrap_or(NonZeroUsize::new(1).unwrap())
    }
}

node! {
    /// An item in a bullet list: `- ...`.
    ListItem
}

impl<'a> ListItem<'a> {
    /// The contents of the list item.
    pub fn body(self) -> Markup<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// An item in an enumeration (numbered list): `+ ...` or `1. ...`.
    EnumItem
}

impl<'a> EnumItem<'a> {
    /// The explicit numbering, if any: `23.`.
    pub fn number(self) -> Option<usize> {
        self.0.children().find_map(|node| match node.kind() {
            SyntaxKind::EnumMarker => node.text().trim_end_matches('.').parse().ok(),
            _ => Option::None,
        })
    }

    /// The contents of the list item.
    pub fn body(self) -> Markup<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// An item in a term list: `/ Term: Details`.
    TermItem
}

impl<'a> TermItem<'a> {
    /// The term described by the item.
    pub fn term(self) -> Markup<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The description of the term.
    pub fn description(self) -> Markup<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A mathematical equation: `$x$`, `$ x^2 $`.
    Equation
}

impl<'a> Equation<'a> {
    /// The contained math.
    pub fn body(self) -> Math<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// Whether the equation should be displayed as a separate block.
    pub fn block(self) -> bool {
        let is_space = |node: Option<&SyntaxNode>| {
            node.map(SyntaxNode::kind) == Some(SyntaxKind::Space)
        };
        is_space(self.0.children().nth(1)) && is_space(self.0.children().nth_back(1))
    }
}

node! {
    /// The contents of a mathematical equation: `x^2 + 1`.
    Math
}

impl<'a> Math<'a> {
    /// The expressions the mathematical content consists of.
    pub fn exprs(self) -> impl DoubleEndedIterator<Item = Expr<'a>> {
        self.0.children().filter_map(Expr::cast_with_space)
    }
}

node! {
    /// An identifier in math: `pi`.
    MathIdent
}

impl<'a> MathIdent<'a> {
    /// Get the identifier.
    pub fn get(self) -> &'a EcoString {
        self.0.text()
    }

    /// Get the identifier as a string slice.
    pub fn as_str(self) -> &'a str {
        self.get()
    }
}

impl Deref for MathIdent<'_> {
    type Target = str;

    /// Dereference to a string. Note that this shortens the lifetime, so you
    /// may need to use [`get()`](Self::get) instead in some situations.
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

node! {
    /// A shorthand for a unicode codepoint in math: `a <= b`.
    MathShorthand
}

impl MathShorthand<'_> {
    /// A list of all shorthands in math mode.
    pub const LIST: &'static [(&'static str, char)] = &[
        ("...", '…'),
        ("-", '−'),
        ("*", '∗'),
        ("~", '∼'),
        ("!=", '≠'),
        (":=", '≔'),
        ("::=", '⩴'),
        ("=:", '≕'),
        ("<<", '≪'),
        ("<<<", '⋘'),
        (">>", '≫'),
        (">>>", '⋙'),
        ("<=", '≤'),
        (">=", '≥'),
        ("->", '→'),
        ("-->", '⟶'),
        ("|->", '↦'),
        (">->", '↣'),
        ("->>", '↠'),
        ("<-", '←'),
        ("<--", '⟵'),
        ("<-<", '↢'),
        ("<<-", '↞'),
        ("<->", '↔'),
        ("<-->", '⟷'),
        ("~>", '⇝'),
        ("~~>", '⟿'),
        ("<~", '⇜'),
        ("<~~", '⬳'),
        ("=>", '⇒'),
        ("|=>", '⤇'),
        ("==>", '⟹'),
        ("<==", '⟸'),
        ("<=>", '⇔'),
        ("<==>", '⟺'),
        ("[|", '⟦'),
        ("|]", '⟧'),
        ("||", '‖'),
    ];

    /// Get the shorthanded character.
    pub fn get(self) -> char {
        let text = self.0.text();
        Self::LIST
            .iter()
            .find(|&&(s, _)| s == text)
            .map_or_else(char::default, |&(_, c)| c)
    }
}

node! {
    /// An alignment point in math: `&`.
    MathAlignPoint
}

node! {
    /// Matched delimiters in math: `[x + y]`.
    MathDelimited
}

impl<'a> MathDelimited<'a> {
    /// The opening delimiter.
    pub fn open(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The contents, including the delimiters.
    pub fn body(self) -> Math<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The closing delimiter.
    pub fn close(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A base with optional attachments in math: `a_1^2`.
    MathAttach
}

impl<'a> MathAttach<'a> {
    /// The base, to which things are attached.
    pub fn base(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The bottom attachment.
    pub fn bottom(self) -> Option<Expr<'a>> {
        self.0
            .children()
            .skip_while(|node| !matches!(node.kind(), SyntaxKind::Underscore))
            .find_map(SyntaxNode::cast)
    }

    /// The top attachment.
    pub fn top(self) -> Option<Expr<'a>> {
        self.0
            .children()
            .skip_while(|node| !matches!(node.kind(), SyntaxKind::Hat))
            .find_map(SyntaxNode::cast)
    }

    /// Extract attached primes if present.
    pub fn primes(self) -> Option<MathPrimes<'a>> {
        self.0
            .children()
            .skip_while(|node| node.cast::<Expr<'_>>().is_none())
            .nth(1)
            .and_then(|n| n.cast())
    }
}

node! {
    /// Grouped primes in math: `a'''`.
    MathPrimes
}

impl MathPrimes<'_> {
    /// The number of grouped primes.
    pub fn count(self) -> usize {
        self.0
            .children()
            .filter(|node| matches!(node.kind(), SyntaxKind::Prime))
            .count()
    }
}

node! {
    /// A fraction in math: `x/2`
    MathFrac
}

impl<'a> MathFrac<'a> {
    /// The numerator.
    pub fn num(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The denominator.
    pub fn denom(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A root in math: `√x`, `∛x` or `∜x`.
    MathRoot
}

impl<'a> MathRoot<'a> {
    /// The index of the root.
    pub fn index(self) -> Option<usize> {
        match self.0.children().next().map(|node| node.text().as_str()) {
            Some("∜") => Some(4),
            Some("∛") => Some(3),
            Some("√") => Option::None,
            _ => Option::None,
        }
    }

    /// The radicand.
    pub fn radicand(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// An identifier: `it`.
    Ident
}

impl<'a> Ident<'a> {
    /// Get the identifier.
    pub fn get(self) -> &'a EcoString {
        self.0.text()
    }

    /// Get the identifier as a string slice.
    pub fn as_str(self) -> &'a str {
        self.get()
    }
}

impl Deref for Ident<'_> {
    type Target = str;

    /// Dereference to a string. Note that this shortens the lifetime, so you
    /// may need to use [`get()`](Self::get) instead in some situations.
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

node! {
    /// The `none` literal.
    None
}

node! {
    /// The `auto` literal.
    Auto
}

node! {
    /// A boolean: `true`, `false`.
    Bool
}

impl Bool<'_> {
    /// Get the boolean value.
    pub fn get(self) -> bool {
        self.0.text() == "true"
    }
}

node! {
    /// An integer: `120`.
    Int
}

impl Int<'_> {
    /// Get the integer value.
    pub fn get(self) -> i64 {
        let text = self.0.text();
        if let Some(rest) = text.strip_prefix("0x") {
            i64::from_str_radix(rest, 16)
        } else if let Some(rest) = text.strip_prefix("0o") {
            i64::from_str_radix(rest, 8)
        } else if let Some(rest) = text.strip_prefix("0b") {
            i64::from_str_radix(rest, 2)
        } else {
            text.parse()
        }
        .unwrap_or_default()
    }
}

node! {
    /// A floating-point number: `1.2`, `10e-4`.
    Float
}

impl Float<'_> {
    /// Get the floating-point value.
    pub fn get(self) -> f64 {
        self.0.text().parse().unwrap_or_default()
    }
}

node! {
    /// A numeric value with a unit: `12pt`, `3cm`, `2em`, `90deg`, `50%`.
    Numeric
}

impl Numeric<'_> {
    /// Get the numeric value and unit.
    pub fn get(self) -> (f64, Unit) {
        let text = self.0.text();
        let count = text
            .chars()
            .rev()
            .take_while(|c| matches!(c, 'a'..='z' | '%'))
            .count();

        let split = text.len() - count;
        let value = text[..split].parse().unwrap_or_default();
        let unit = match &text[split..] {
            "pt" => Unit::Pt,
            "mm" => Unit::Mm,
            "cm" => Unit::Cm,
            "in" => Unit::In,
            "deg" => Unit::Deg,
            "rad" => Unit::Rad,
            "em" => Unit::Em,
            "fr" => Unit::Fr,
            "%" => Unit::Percent,
            _ => Unit::Percent,
        };

        (value, unit)
    }
}

/// Unit of a numeric value.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Unit {
    /// Points.
    Pt,
    /// Millimeters.
    Mm,
    /// Centimeters.
    Cm,
    /// Inches.
    In,
    /// Radians.
    Rad,
    /// Degrees.
    Deg,
    /// Font-relative: `1em` is the same as the font size.
    Em,
    /// Fractions: `fr`.
    Fr,
    /// Percentage: `%`.
    Percent,
}

node! {
    /// A quoted string: `"..."`.
    Str
}

impl Str<'_> {
    /// Get the string value with resolved escape sequences.
    pub fn get(self) -> EcoString {
        let text = self.0.text();
        let unquoted = &text[1..text.len() - 1];
        if !unquoted.contains('\\') {
            return unquoted.into();
        }

        let mut out = EcoString::with_capacity(unquoted.len());
        let mut s = Scanner::new(unquoted);

        while let Some(c) = s.eat() {
            if c != '\\' {
                out.push(c);
                continue;
            }

            let start = s.locate(-1);
            match s.eat() {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('u') if s.eat_if('{') => {
                    let sequence = s.eat_while(char::is_ascii_hexdigit);
                    s.eat_if('}');

                    match u32::from_str_radix(sequence, 16)
                        .ok()
                        .and_then(std::char::from_u32)
                    {
                        Some(c) => out.push(c),
                        Option::None => out.push_str(s.from(start)),
                    }
                }
                _ => out.push_str(s.from(start)),
            }
        }

        out
    }
}

node! {
    /// A code block: `{ let x = 1; x + 2 }`.
    CodeBlock
}

impl<'a> CodeBlock<'a> {
    /// The contained code.
    pub fn body(self) -> Code<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// The body of a code block.
    Code
}

impl<'a> Code<'a> {
    /// The list of expressions contained in the code.
    pub fn exprs(self) -> impl DoubleEndedIterator<Item = Expr<'a>> {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

node! {
    /// A content block: `[*Hi* there!]`.
    ContentBlock
}

impl<'a> ContentBlock<'a> {
    /// The contained markup.
    pub fn body(self) -> Markup<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// A grouped expression: `(1 + 2)`.
    Parenthesized
}

impl<'a> Parenthesized<'a> {
    /// The wrapped expression.
    ///
    /// Should only be accessed if this is contained in an `Expr`.
    pub fn expr(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The wrapped pattern.
    ///
    /// Should only be accessed if this is contained in a `Pattern`.
    pub fn pattern(self) -> Pattern<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// An array: `(1, "hi", 12cm)`.
    Array
}

impl<'a> Array<'a> {
    /// The array's items.
    pub fn items(self) -> impl DoubleEndedIterator<Item = ArrayItem<'a>> {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

/// An item in an array.
#[derive(Debug, Copy, Clone, Hash)]
pub enum ArrayItem<'a> {
    /// A bare expression: `12`.
    Pos(Expr<'a>),
    /// A spread expression: `..things`.
    Spread(Spread<'a>),
}

impl<'a> AstNode<'a> for ArrayItem<'a> {
    fn from_untyped(node: &'a SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Spread => node.cast().map(Self::Spread),
            _ => node.cast().map(Self::Pos),
        }
    }

    fn to_untyped(self) -> &'a SyntaxNode {
        match self {
            Self::Pos(v) => v.to_untyped(),
            Self::Spread(v) => v.to_untyped(),
        }
    }
}

node! {
    /// A dictionary: `(thickness: 3pt, dash: "solid")`.
    Dict
}

impl<'a> Dict<'a> {
    /// The dictionary's items.
    pub fn items(self) -> impl DoubleEndedIterator<Item = DictItem<'a>> {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

/// An item in an dictionary expression.
#[derive(Debug, Copy, Clone, Hash)]
pub enum DictItem<'a> {
    /// A named pair: `thickness: 3pt`.
    Named(Named<'a>),
    /// A keyed pair: `"spacy key": true`.
    Keyed(Keyed<'a>),
    /// A spread expression: `..things`.
    Spread(Spread<'a>),
}

impl<'a> AstNode<'a> for DictItem<'a> {
    fn from_untyped(node: &'a SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Named => node.cast().map(Self::Named),
            SyntaxKind::Keyed => node.cast().map(Self::Keyed),
            SyntaxKind::Spread => node.cast().map(Self::Spread),
            _ => Option::None,
        }
    }

    fn to_untyped(self) -> &'a SyntaxNode {
        match self {
            Self::Named(v) => v.to_untyped(),
            Self::Keyed(v) => v.to_untyped(),
            Self::Spread(v) => v.to_untyped(),
        }
    }
}

node! {
    /// A named pair: `thickness: 3pt`.
    Named
}

impl<'a> Named<'a> {
    /// The name: `thickness`.
    pub fn name(self) -> Ident<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The right-hand side of the pair: `3pt`.
    ///
    /// This should only be accessed if this `Named` is contained in a
    /// `DictItem`, `Arg`, or `Param`.
    pub fn expr(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }

    /// The right-hand side of the pair as a pattern.
    ///
    /// This should only be accessed if this `Named` is contained in a
    /// `Destructuring`.
    pub fn pattern(self) -> Pattern<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A keyed pair: `"spacy key": true`.
    Keyed
}

impl<'a> Keyed<'a> {
    /// The key: `"spacy key"`.
    pub fn key(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The right-hand side of the pair: `true`.
    ///
    /// This should only be accessed if this `Keyed` is contained in a
    /// `DictItem`.
    pub fn expr(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A spread: `..x` or `..x.at(0)`.
    Spread
}

impl<'a> Spread<'a> {
    /// The spread expression.
    ///
    /// This should only be accessed if this `Spread` is contained in an
    /// `ArrayItem`, `DictItem`, or `Arg`.
    pub fn expr(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The sink identifier, if present.
    ///
    /// This should only be accessed if this `Spread` is contained in a
    /// `Param` or binding `DestructuringItem`.
    pub fn sink_ident(self) -> Option<Ident<'a>> {
        self.0.cast_first_match()
    }

    /// The sink expressions, if present.
    ///
    /// This should only be accessed if this `Spread` is contained in a
    /// `DestructuringItem`.
    pub fn sink_expr(self) -> Option<Expr<'a>> {
        self.0.cast_first_match()
    }
}

node! {
    /// A unary operation: `-x`.
    Unary
}

impl<'a> Unary<'a> {
    /// The operator: `-`.
    pub fn op(self) -> UnOp {
        self.0
            .children()
            .find_map(|node| UnOp::from_kind(node.kind()))
            .unwrap_or(UnOp::Pos)
    }

    /// The expression to operate on: `x`.
    pub fn expr(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

/// A unary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum UnOp {
    /// The plus operator: `+`.
    Pos,
    /// The negation operator: `-`.
    Neg,
    /// The boolean `not`.
    Not,
}

impl UnOp {
    /// Try to convert the token into a unary operation.
    pub fn from_kind(token: SyntaxKind) -> Option<Self> {
        Some(match token {
            SyntaxKind::Plus => Self::Pos,
            SyntaxKind::Minus => Self::Neg,
            SyntaxKind::Not => Self::Not,
            _ => return Option::None,
        })
    }

    /// The precedence of this operator.
    pub fn precedence(self) -> usize {
        match self {
            Self::Pos | Self::Neg => 7,
            Self::Not => 4,
        }
    }

    /// The string representation of this operation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pos => "+",
            Self::Neg => "-",
            Self::Not => "not",
        }
    }
}

node! {
    /// A binary operation: `a + b`.
    Binary
}

impl<'a> Binary<'a> {
    /// The binary operator: `+`.
    pub fn op(self) -> BinOp {
        let mut not = false;
        self.0
            .children()
            .find_map(|node| match node.kind() {
                SyntaxKind::Not => {
                    not = true;
                    Option::None
                }
                SyntaxKind::In if not => Some(BinOp::NotIn),
                _ => BinOp::from_kind(node.kind()),
            })
            .unwrap_or(BinOp::Add)
    }

    /// The left-hand side of the operation: `a`.
    pub fn lhs(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The right-hand side of the operation: `b`.
    pub fn rhs(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

/// A binary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BinOp {
    /// The addition operator: `+`.
    Add,
    /// The subtraction operator: `-`.
    Sub,
    /// The multiplication operator: `*`.
    Mul,
    /// The division operator: `/`.
    Div,
    /// The short-circuiting boolean `and`.
    And,
    /// The short-circuiting boolean `or`.
    Or,
    /// The equality operator: `==`.
    Eq,
    /// The inequality operator: `!=`.
    Neq,
    /// The less-than operator: `<`.
    Lt,
    /// The less-than or equal operator: `<=`.
    Leq,
    /// The greater-than operator: `>`.
    Gt,
    /// The greater-than or equal operator: `>=`.
    Geq,
    /// The assignment operator: `=`.
    Assign,
    /// The containment operator: `in`.
    In,
    /// The inverse containment operator: `not in`.
    NotIn,
    /// The add-assign operator: `+=`.
    AddAssign,
    /// The subtract-assign operator: `-=`.
    SubAssign,
    /// The multiply-assign operator: `*=`.
    MulAssign,
    /// The divide-assign operator: `/=`.
    DivAssign,
}

impl BinOp {
    /// Try to convert the token into a binary operation.
    pub fn from_kind(token: SyntaxKind) -> Option<Self> {
        Some(match token {
            SyntaxKind::Plus => Self::Add,
            SyntaxKind::Minus => Self::Sub,
            SyntaxKind::Star => Self::Mul,
            SyntaxKind::Slash => Self::Div,
            SyntaxKind::And => Self::And,
            SyntaxKind::Or => Self::Or,
            SyntaxKind::EqEq => Self::Eq,
            SyntaxKind::ExclEq => Self::Neq,
            SyntaxKind::Lt => Self::Lt,
            SyntaxKind::LtEq => Self::Leq,
            SyntaxKind::Gt => Self::Gt,
            SyntaxKind::GtEq => Self::Geq,
            SyntaxKind::Eq => Self::Assign,
            SyntaxKind::In => Self::In,
            SyntaxKind::PlusEq => Self::AddAssign,
            SyntaxKind::HyphEq => Self::SubAssign,
            SyntaxKind::StarEq => Self::MulAssign,
            SyntaxKind::SlashEq => Self::DivAssign,
            _ => return Option::None,
        })
    }

    /// The precedence of this operator.
    pub fn precedence(self) -> usize {
        match self {
            Self::Mul => 6,
            Self::Div => 6,
            Self::Add => 5,
            Self::Sub => 5,
            Self::Eq => 4,
            Self::Neq => 4,
            Self::Lt => 4,
            Self::Leq => 4,
            Self::Gt => 4,
            Self::Geq => 4,
            Self::In => 4,
            Self::NotIn => 4,
            Self::And => 3,
            Self::Or => 2,
            Self::Assign => 1,
            Self::AddAssign => 1,
            Self::SubAssign => 1,
            Self::MulAssign => 1,
            Self::DivAssign => 1,
        }
    }

    /// The associativity of this operator.
    pub fn assoc(self) -> Assoc {
        match self {
            Self::Add => Assoc::Left,
            Self::Sub => Assoc::Left,
            Self::Mul => Assoc::Left,
            Self::Div => Assoc::Left,
            Self::And => Assoc::Left,
            Self::Or => Assoc::Left,
            Self::Eq => Assoc::Left,
            Self::Neq => Assoc::Left,
            Self::Lt => Assoc::Left,
            Self::Leq => Assoc::Left,
            Self::Gt => Assoc::Left,
            Self::Geq => Assoc::Left,
            Self::In => Assoc::Left,
            Self::NotIn => Assoc::Left,
            Self::Assign => Assoc::Right,
            Self::AddAssign => Assoc::Right,
            Self::SubAssign => Assoc::Right,
            Self::MulAssign => Assoc::Right,
            Self::DivAssign => Assoc::Right,
        }
    }

    /// The string representation of this operation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::And => "and",
            Self::Or => "or",
            Self::Eq => "==",
            Self::Neq => "!=",
            Self::Lt => "<",
            Self::Leq => "<=",
            Self::Gt => ">",
            Self::Geq => ">=",
            Self::In => "in",
            Self::NotIn => "not in",
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::MulAssign => "*=",
            Self::DivAssign => "/=",
        }
    }
}

/// The associativity of a binary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Assoc {
    /// Left-associative: `a + b + c` is equivalent to `(a + b) + c`.
    Left,
    /// Right-associative: `a = b = c` is equivalent to `a = (b = c)`.
    Right,
}

node! {
    /// A field access: `properties.age`.
    FieldAccess
}

impl<'a> FieldAccess<'a> {
    /// The expression to access the field on.
    pub fn target(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The name of the field.
    pub fn field(self) -> Ident<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// An invocation of a function or method: `f(x, y)`.
    FuncCall
}

impl<'a> FuncCall<'a> {
    /// The function to call.
    pub fn callee(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The arguments to the function.
    pub fn args(self) -> Args<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A function call's argument list: `(12pt, y)`.
    Args
}

impl<'a> Args<'a> {
    /// The positional and named arguments.
    pub fn items(self) -> impl DoubleEndedIterator<Item = Arg<'a>> {
        self.0.children().filter_map(SyntaxNode::cast)
    }

    /// Whether there is a comma at the end.
    pub fn trailing_comma(self) -> bool {
        self.0
            .children()
            .rev()
            .skip(1)
            .find(|n| !n.kind().is_trivia())
            .is_some_and(|n| n.kind() == SyntaxKind::Comma)
    }
}

/// An argument to a function call.
#[derive(Debug, Copy, Clone, Hash)]
pub enum Arg<'a> {
    /// A positional argument: `12`.
    Pos(Expr<'a>),
    /// A named argument: `draw: false`.
    Named(Named<'a>),
    /// A spread argument: `..things`.
    Spread(Spread<'a>),
}

impl<'a> AstNode<'a> for Arg<'a> {
    fn from_untyped(node: &'a SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Named => node.cast().map(Self::Named),
            SyntaxKind::Spread => node.cast().map(Self::Spread),
            _ => node.cast().map(Self::Pos),
        }
    }

    fn to_untyped(self) -> &'a SyntaxNode {
        match self {
            Self::Pos(v) => v.to_untyped(),
            Self::Named(v) => v.to_untyped(),
            Self::Spread(v) => v.to_untyped(),
        }
    }
}

node! {
    /// A closure: `(x, y) => z`.
    Closure
}

impl<'a> Closure<'a> {
    /// The name of the closure.
    ///
    /// This only exists if you use the function syntax sugar: `let f(x) = y`.
    pub fn name(self) -> Option<Ident<'a>> {
        self.0.children().next()?.cast()
    }

    /// The parameter bindings.
    pub fn params(self) -> Params<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The body of the closure.
    pub fn body(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A closure's parameters: `(x, y)`.
    Params
}

impl<'a> Params<'a> {
    /// The parameter bindings.
    pub fn children(self) -> impl DoubleEndedIterator<Item = Param<'a>> {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

/// A parameter to a closure.
#[derive(Debug, Copy, Clone, Hash)]
pub enum Param<'a> {
    /// A positional parameter: `x`.
    Pos(Pattern<'a>),
    /// A named parameter with a default value: `draw: false`.
    Named(Named<'a>),
    /// An argument sink: `..args` or `..`.
    Spread(Spread<'a>),
}

impl<'a> AstNode<'a> for Param<'a> {
    fn from_untyped(node: &'a SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Named => node.cast().map(Self::Named),
            SyntaxKind::Spread => node.cast().map(Self::Spread),
            _ => node.cast().map(Self::Pos),
        }
    }

    fn to_untyped(self) -> &'a SyntaxNode {
        match self {
            Self::Pos(v) => v.to_untyped(),
            Self::Named(v) => v.to_untyped(),
            Self::Spread(v) => v.to_untyped(),
        }
    }
}

/// The kind of a pattern.
#[derive(Debug, Copy, Clone, Hash)]
pub enum Pattern<'a> {
    /// A single expression: `x`.
    Normal(Expr<'a>),
    /// A placeholder: `_`.
    Placeholder(Underscore<'a>),
    /// A parenthesized pattern.
    Parenthesized(Parenthesized<'a>),
    /// A destructuring pattern: `(x, _, ..y)`.
    Destructuring(Destructuring<'a>),
}

impl<'a> AstNode<'a> for Pattern<'a> {
    fn from_untyped(node: &'a SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Underscore => node.cast().map(Self::Placeholder),
            SyntaxKind::Parenthesized => node.cast().map(Self::Parenthesized),
            SyntaxKind::Destructuring => node.cast().map(Self::Destructuring),
            _ => node.cast().map(Self::Normal),
        }
    }

    fn to_untyped(self) -> &'a SyntaxNode {
        match self {
            Self::Normal(v) => v.to_untyped(),
            Self::Placeholder(v) => v.to_untyped(),
            Self::Parenthesized(v) => v.to_untyped(),
            Self::Destructuring(v) => v.to_untyped(),
        }
    }
}

impl<'a> Pattern<'a> {
    /// Returns a list of all new bindings introduced by the pattern.
    pub fn bindings(self) -> Vec<Ident<'a>> {
        match self {
            Self::Normal(Expr::Ident(ident)) => vec![ident],
            Self::Parenthesized(v) => v.pattern().bindings(),
            Self::Destructuring(v) => v.bindings(),
            _ => vec![],
        }
    }
}

impl Default for Pattern<'_> {
    fn default() -> Self {
        Self::Normal(Expr::default())
    }
}

node! {
    /// An underscore: `_`
    Underscore
}

node! {
    /// A destructuring pattern: `x` or `(x, _, ..y)`.
    Destructuring
}

impl<'a> Destructuring<'a> {
    /// The items of the destructuring.
    pub fn items(self) -> impl DoubleEndedIterator<Item = DestructuringItem<'a>> {
        self.0.children().filter_map(SyntaxNode::cast)
    }

    /// Returns a list of all new bindings introduced by the destructuring.
    pub fn bindings(self) -> Vec<Ident<'a>> {
        self.items()
            .flat_map(|binding| match binding {
                DestructuringItem::Pattern(pattern) => pattern.bindings(),
                DestructuringItem::Named(named) => named.pattern().bindings(),
                DestructuringItem::Spread(spread) => {
                    spread.sink_ident().into_iter().collect()
                }
            })
            .collect()
    }
}

/// The kind of an element in a destructuring pattern.
#[derive(Debug, Copy, Clone, Hash)]
pub enum DestructuringItem<'a> {
    /// A sub-pattern: `x`.
    Pattern(Pattern<'a>),
    /// A renamed destructuring: `x: y`.
    Named(Named<'a>),
    /// A destructuring sink: `..y` or `..`.
    Spread(Spread<'a>),
}

impl<'a> AstNode<'a> for DestructuringItem<'a> {
    fn from_untyped(node: &'a SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Named => node.cast().map(Self::Named),
            SyntaxKind::Spread => node.cast().map(Self::Spread),
            _ => node.cast().map(Self::Pattern),
        }
    }

    fn to_untyped(self) -> &'a SyntaxNode {
        match self {
            Self::Pattern(v) => v.to_untyped(),
            Self::Named(v) => v.to_untyped(),
            Self::Spread(v) => v.to_untyped(),
        }
    }
}

node! {
    /// A let binding: `let x = 1`.
    LetBinding
}

/// The kind of a let binding, either a normal one or a closure.
#[derive(Debug)]
pub enum LetBindingKind<'a> {
    /// A normal binding: `let x = 1`.
    Normal(Pattern<'a>),
    /// A closure binding: `let f(x) = 1`.
    Closure(Ident<'a>),
}

impl<'a> LetBindingKind<'a> {
    /// Returns a list of all new bindings introduced by the let binding.
    pub fn bindings(self) -> Vec<Ident<'a>> {
        match self {
            LetBindingKind::Normal(pattern) => pattern.bindings(),
            LetBindingKind::Closure(ident) => vec![ident],
        }
    }
}

impl<'a> LetBinding<'a> {
    /// The kind of the let binding.
    pub fn kind(self) -> LetBindingKind<'a> {
        match self.0.cast_first_match::<Pattern>() {
            Some(Pattern::Normal(Expr::Closure(closure))) => {
                LetBindingKind::Closure(closure.name().unwrap_or_default())
            }
            pattern => LetBindingKind::Normal(pattern.unwrap_or_default()),
        }
    }

    /// The expression the binding is initialized with.
    pub fn init(self) -> Option<Expr<'a>> {
        match self.kind() {
            LetBindingKind::Normal(Pattern::Normal(_) | Pattern::Parenthesized(_)) => {
                self.0.children().filter_map(SyntaxNode::cast).nth(1)
            }
            LetBindingKind::Normal(_) => self.0.cast_first_match(),
            LetBindingKind::Closure(_) => self.0.cast_first_match(),
        }
    }
}

node! {
    /// An assignment expression `(x, y) = (1, 2)`.
    DestructAssignment
}

impl<'a> DestructAssignment<'a> {
    /// The pattern of the assignment.
    pub fn pattern(self) -> Pattern<'a> {
        self.0.cast_first_match::<Pattern>().unwrap_or_default()
    }

    /// The expression that is assigned.
    pub fn value(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A set rule: `set text(...)`.
    SetRule
}

impl<'a> SetRule<'a> {
    /// The function to set style properties for.
    pub fn target(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The style properties to set.
    pub fn args(self) -> Args<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }

    /// A condition under which the set rule applies.
    pub fn condition(self) -> Option<Expr<'a>> {
        self.0
            .children()
            .skip_while(|child| child.kind() != SyntaxKind::If)
            .find_map(SyntaxNode::cast)
    }
}

node! {
    /// A show rule: `show heading: it => emph(it.body)`.
    ShowRule
}

impl<'a> ShowRule<'a> {
    /// Defines which nodes the show rule applies to.
    pub fn selector(self) -> Option<Expr<'a>> {
        self.0
            .children()
            .rev()
            .skip_while(|child| child.kind() != SyntaxKind::Colon)
            .find_map(SyntaxNode::cast)
    }

    /// The transformation recipe.
    pub fn transform(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A contextual expression: `context text.lang`.
    Contextual
}

impl<'a> Contextual<'a> {
    /// The expression which depends on the context.
    pub fn body(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// An if-else conditional: `if x { y } else { z }`.
    Conditional
}

impl<'a> Conditional<'a> {
    /// The condition which selects the body to evaluate.
    pub fn condition(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The expression to evaluate if the condition is true.
    pub fn if_body(self) -> Expr<'a> {
        self.0
            .children()
            .filter_map(SyntaxNode::cast)
            .nth(1)
            .unwrap_or_default()
    }

    /// The expression to evaluate if the condition is false.
    pub fn else_body(self) -> Option<Expr<'a>> {
        self.0.children().filter_map(SyntaxNode::cast).nth(2)
    }
}

node! {
    /// A while loop: `while x { y }`.
    WhileLoop
}

impl<'a> WhileLoop<'a> {
    /// The condition which selects whether to evaluate the body.
    pub fn condition(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The expression to evaluate while the condition is true.
    pub fn body(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A for loop: `for x in y { z }`.
    ForLoop
}

impl<'a> ForLoop<'a> {
    /// The pattern to assign to.
    pub fn pattern(self) -> Pattern<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The expression to iterate over.
    pub fn iterable(self) -> Expr<'a> {
        self.0
            .children()
            .skip_while(|&c| c.kind() != SyntaxKind::In)
            .find_map(SyntaxNode::cast)
            .unwrap_or_default()
    }

    /// The expression to evaluate for each iteration.
    pub fn body(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A module import: `import "utils.typ": a, b, c`.
    ModuleImport
}

impl<'a> ModuleImport<'a> {
    /// The module or path from which the items should be imported.
    pub fn source(self) -> Expr<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The items to be imported.
    pub fn imports(self) -> Option<Imports<'a>> {
        self.0.children().find_map(|node| match node.kind() {
            SyntaxKind::Star => Some(Imports::Wildcard),
            SyntaxKind::ImportItems => node.cast().map(Imports::Items),
            _ => Option::None,
        })
    }

    /// The name this module was assigned to, if it was renamed with `as`
    /// (`renamed` in `import "..." as renamed`).
    pub fn new_name(self) -> Option<Ident<'a>> {
        self.0
            .children()
            .skip_while(|child| child.kind() != SyntaxKind::As)
            .find_map(SyntaxNode::cast)
    }
}

/// The items that ought to be imported from a file.
#[derive(Debug, Copy, Clone, Hash)]
pub enum Imports<'a> {
    /// All items in the scope of the file should be imported.
    Wildcard,
    /// The specified items from the file should be imported.
    Items(ImportItems<'a>),
}

node! {
    /// Items to import from a module: `a, b, c`.
    ImportItems
}

impl<'a> ImportItems<'a> {
    /// Returns an iterator over the items to import from the module.
    pub fn iter(self) -> impl DoubleEndedIterator<Item = ImportItem<'a>> {
        self.0.children().filter_map(|child| match child.kind() {
            SyntaxKind::RenamedImportItem => child.cast().map(ImportItem::Renamed),
            SyntaxKind::ImportItemPath => child.cast().map(ImportItem::Simple),
            _ => Option::None,
        })
    }
}

node! {
    /// A path to a submodule's imported name: `a.b.c`.
    ImportItemPath
}

impl<'a> ImportItemPath<'a> {
    /// An iterator over the path's components.
    pub fn iter(self) -> impl DoubleEndedIterator<Item = Ident<'a>> {
        self.0.children().filter_map(SyntaxNode::cast)
    }

    /// The name of the imported item. This is the last segment in the path.
    pub fn name(self) -> Ident<'a> {
        self.iter().last().unwrap_or_default()
    }
}

/// An imported item, potentially renamed to another identifier.
#[derive(Debug, Copy, Clone, Hash)]
pub enum ImportItem<'a> {
    /// A non-renamed import (the item's name in the scope is the same as its
    /// name).
    Simple(ImportItemPath<'a>),
    /// A renamed import (the item was bound to a different name in the scope
    /// than the one it was defined as).
    Renamed(RenamedImportItem<'a>),
}

impl<'a> ImportItem<'a> {
    /// The path to the imported item.
    pub fn path(self) -> ImportItemPath<'a> {
        match self {
            Self::Simple(path) => path,
            Self::Renamed(renamed_item) => renamed_item.path(),
        }
    }

    /// The original name of the imported item, at its source. This will be the
    /// equal to the bound name if the item wasn't renamed with 'as'.
    pub fn original_name(self) -> Ident<'a> {
        match self {
            Self::Simple(path) => path.name(),
            Self::Renamed(renamed_item) => renamed_item.original_name(),
        }
    }

    /// The name which this import item was bound to. Corresponds to the new
    /// name, if it was renamed; otherwise, it's just its original name.
    pub fn bound_name(self) -> Ident<'a> {
        match self {
            Self::Simple(path) => path.name(),
            Self::Renamed(renamed_item) => renamed_item.new_name(),
        }
    }
}

node! {
    /// A renamed import item: `a as d`
    RenamedImportItem
}

impl<'a> RenamedImportItem<'a> {
    /// The path to the imported item.
    pub fn path(self) -> ImportItemPath<'a> {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The original name of the imported item (`a` in `a as d` or `c.b.a as d`).
    pub fn original_name(self) -> Ident<'a> {
        self.path().name()
    }

    /// The new name of the imported item (`d` in `a as d`).
    pub fn new_name(self) -> Ident<'a> {
        self.0
            .children()
            .filter_map(SyntaxNode::cast)
            .last()
            .unwrap_or_default()
    }
}

node! {
    /// A module include: `include "chapter1.typ"`.
    ModuleInclude
}

impl<'a> ModuleInclude<'a> {
    /// The module or path from which the content should be included.
    pub fn source(self) -> Expr<'a> {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A break from a loop: `break`.
    LoopBreak
}

node! {
    /// A continue in a loop: `continue`.
    LoopContinue
}

node! {
    /// A return from a function: `return`, `return x + 1`.
    FuncReturn
}

impl<'a> FuncReturn<'a> {
    /// The expression to return.
    pub fn body(self) -> Option<Expr<'a>> {
        self.0.cast_last_match()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_default() {
        assert!(Expr::default().to_untyped().cast::<Expr>().is_some());
    }
}
