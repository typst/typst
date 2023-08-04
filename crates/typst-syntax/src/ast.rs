//! A typed layer over the untyped syntax tree.
//!
//! The AST is rooted in the [`Markup`] node.

use std::num::NonZeroUsize;
use std::ops::Deref;

use ecow::EcoString;
use unscanny::Scanner;

use super::{
    is_id_continue, is_id_start, is_newline, split_newlines, Span, SyntaxKind, SyntaxNode,
};

/// A typed AST node.
pub trait AstNode: Sized {
    /// Convert a node into its typed variant.
    fn from_untyped(node: &SyntaxNode) -> Option<Self>;

    /// A reference to the underlying syntax node.
    fn as_untyped(&self) -> &SyntaxNode;

    /// The source code location.
    fn span(&self) -> Span {
        self.as_untyped().span()
    }
}

macro_rules! node {
    ($(#[$attr:meta])* $name:ident) => {
        #[derive(Debug, Default, Clone, Hash)]
        #[repr(transparent)]
        $(#[$attr])*
        pub struct $name(SyntaxNode);

        impl AstNode for $name {
            fn from_untyped(node: &SyntaxNode) -> Option<Self> {
                if matches!(node.kind(), SyntaxKind::$name) {
                    Some(Self(node.clone()))
                } else {
                    Option::None
                }
            }

            fn as_untyped(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

node! {
    /// The syntactical root capable of representing a full parsed document.
    Markup
}

impl Markup {
    /// The expressions.
    pub fn exprs(&self) -> impl DoubleEndedIterator<Item = Expr> + '_ {
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
#[derive(Debug, Clone, Hash)]
pub enum Expr {
    /// Plain text without markup.
    Text(Text),
    /// Whitespace in markup or math. Has at most one newline in markup, as more
    /// indicate a paragraph break.
    Space(Space),
    /// A forced line break: `\`.
    Linebreak(Linebreak),
    /// A paragraph break, indicated by one or multiple blank lines.
    Parbreak(Parbreak),
    /// An escape sequence: `\#`, `\u{1F5FA}`.
    Escape(Escape),
    /// A shorthand for a unicode codepoint. For example, `~` for non-breaking
    /// space or `-?` for a soft hyphen.
    Shorthand(Shorthand),
    /// A smart quote: `'` or `"`.
    SmartQuote(SmartQuote),
    /// Strong content: `*Strong*`.
    Strong(Strong),
    /// Emphasized content: `_Emphasized_`.
    Emph(Emph),
    /// Raw text with optional syntax highlighting: `` `...` ``.
    Raw(Raw),
    /// A hyperlink: `https://typst.org`.
    Link(Link),
    /// A label: `<intro>`.
    Label(Label),
    /// A reference: `@target`, `@target[..]`.
    Ref(Ref),
    /// A section heading: `= Introduction`.
    Heading(Heading),
    /// An item in a bullet list: `- ...`.
    List(ListItem),
    /// An item in an enumeration (numbered list): `+ ...` or `1. ...`.
    Enum(EnumItem),
    /// An item in a term list: `/ Term: Details`.
    Term(TermItem),
    /// A mathematical equation: `$x$`, `$ x^2 $`.
    Equation(Equation),
    /// The contents of a mathematical equation: `x^2 + 1`.
    Math(Math),
    /// An identifier in math: `pi`.
    MathIdent(MathIdent),
    /// An alignment point in math: `&`.
    MathAlignPoint(MathAlignPoint),
    /// Matched delimiters in math: `[x + y]`.
    MathDelimited(MathDelimited),
    /// A base with optional attachments in math: `a_1^2`.
    MathAttach(MathAttach),
    /// Grouped math primes
    MathPrimes(MathPrimes),
    /// A fraction in math: `x/2`.
    MathFrac(MathFrac),
    /// A root in math: `√x`, `∛x` or `∜x`.
    MathRoot(MathRoot),
    /// An identifier: `left`.
    Ident(Ident),
    /// The `none` literal.
    None(None),
    /// The `auto` literal.
    Auto(Auto),
    /// A boolean: `true`, `false`.
    Bool(Bool),
    /// An integer: `120`.
    Int(Int),
    /// A floating-point number: `1.2`, `10e-4`.
    Float(Float),
    /// A numeric value with a unit: `12pt`, `3cm`, `2em`, `90deg`, `50%`.
    Numeric(Numeric),
    /// A quoted string: `"..."`.
    Str(Str),
    /// A code block: `{ let x = 1; x + 2 }`.
    Code(CodeBlock),
    /// A content block: `[*Hi* there!]`.
    Content(ContentBlock),
    /// A grouped expression: `(1 + 2)`.
    Parenthesized(Parenthesized),
    /// An array: `(1, "hi", 12cm)`.
    Array(Array),
    /// A dictionary: `(thickness: 3pt, pattern: dashed)`.
    Dict(Dict),
    /// A unary operation: `-x`.
    Unary(Unary),
    /// A binary operation: `a + b`.
    Binary(Binary),
    /// A field access: `properties.age`.
    FieldAccess(FieldAccess),
    /// An invocation of a function or method: `f(x, y)`.
    FuncCall(FuncCall),
    /// A closure: `(x, y) => z`.
    Closure(Closure),
    /// A let binding: `let x = 1`.
    Let(LetBinding),
    //// A destructuring assignment: `(x, y) = (1, 2)`.
    DestructAssign(DestructAssignment),
    /// A set rule: `set text(...)`.
    Set(SetRule),
    /// A show rule: `show heading: it => emph(it.body)`.
    Show(ShowRule),
    /// An if-else conditional: `if x { y } else { z }`.
    Conditional(Conditional),
    /// A while loop: `while x { y }`.
    While(WhileLoop),
    /// A for loop: `for x in y { z }`.
    For(ForLoop),
    /// A module import: `import "utils.typ": a, b, c`.
    Import(ModuleImport),
    /// A module include: `include "chapter1.typ"`.
    Include(ModuleInclude),
    /// A break from a loop: `break`.
    Break(LoopBreak),
    /// A continue in a loop: `continue`.
    Continue(LoopContinue),
    /// A return from a function: `return`, `return x + 1`.
    Return(FuncReturn),
}

impl Expr {
    fn cast_with_space(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Space => node.cast().map(Self::Space),
            _ => Self::from_untyped(node),
        }
    }
}

impl AstNode for Expr {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
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

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Text(v) => v.as_untyped(),
            Self::Space(v) => v.as_untyped(),
            Self::Linebreak(v) => v.as_untyped(),
            Self::Parbreak(v) => v.as_untyped(),
            Self::Escape(v) => v.as_untyped(),
            Self::Shorthand(v) => v.as_untyped(),
            Self::SmartQuote(v) => v.as_untyped(),
            Self::Strong(v) => v.as_untyped(),
            Self::Emph(v) => v.as_untyped(),
            Self::Raw(v) => v.as_untyped(),
            Self::Link(v) => v.as_untyped(),
            Self::Label(v) => v.as_untyped(),
            Self::Ref(v) => v.as_untyped(),
            Self::Heading(v) => v.as_untyped(),
            Self::List(v) => v.as_untyped(),
            Self::Enum(v) => v.as_untyped(),
            Self::Term(v) => v.as_untyped(),
            Self::Equation(v) => v.as_untyped(),
            Self::Math(v) => v.as_untyped(),
            Self::MathIdent(v) => v.as_untyped(),
            Self::MathAlignPoint(v) => v.as_untyped(),
            Self::MathDelimited(v) => v.as_untyped(),
            Self::MathAttach(v) => v.as_untyped(),
            Self::MathPrimes(v) => v.as_untyped(),
            Self::MathFrac(v) => v.as_untyped(),
            Self::MathRoot(v) => v.as_untyped(),
            Self::Ident(v) => v.as_untyped(),
            Self::None(v) => v.as_untyped(),
            Self::Auto(v) => v.as_untyped(),
            Self::Bool(v) => v.as_untyped(),
            Self::Int(v) => v.as_untyped(),
            Self::Float(v) => v.as_untyped(),
            Self::Numeric(v) => v.as_untyped(),
            Self::Str(v) => v.as_untyped(),
            Self::Code(v) => v.as_untyped(),
            Self::Content(v) => v.as_untyped(),
            Self::Array(v) => v.as_untyped(),
            Self::Dict(v) => v.as_untyped(),
            Self::Parenthesized(v) => v.as_untyped(),
            Self::Unary(v) => v.as_untyped(),
            Self::Binary(v) => v.as_untyped(),
            Self::FieldAccess(v) => v.as_untyped(),
            Self::FuncCall(v) => v.as_untyped(),
            Self::Closure(v) => v.as_untyped(),
            Self::Let(v) => v.as_untyped(),
            Self::DestructAssign(v) => v.as_untyped(),
            Self::Set(v) => v.as_untyped(),
            Self::Show(v) => v.as_untyped(),
            Self::Conditional(v) => v.as_untyped(),
            Self::While(v) => v.as_untyped(),
            Self::For(v) => v.as_untyped(),
            Self::Import(v) => v.as_untyped(),
            Self::Include(v) => v.as_untyped(),
            Self::Break(v) => v.as_untyped(),
            Self::Continue(v) => v.as_untyped(),
            Self::Return(v) => v.as_untyped(),
        }
    }
}

impl Expr {
    /// Can this expression be embedded into markup with a hashtag?
    pub fn hashtag(&self) -> bool {
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
    pub fn is_literal(&self) -> bool {
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

impl Default for Expr {
    fn default() -> Self {
        Expr::Space(Space::default())
    }
}

node! {
    /// Plain text without markup.
    Text
}

impl Text {
    /// Get the text.
    pub fn get(&self) -> &EcoString {
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

impl Escape {
    /// Get the escaped character.
    pub fn get(&self) -> char {
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

impl Shorthand {
    /// A list of all shorthands in markup mode.
    pub const MARKUP_LIST: &[(&'static str, char)] = &[
        ("...", '…'),
        ("~", '\u{00A0}'),
        ("--", '\u{2013}'),
        ("---", '\u{2014}'),
        ("-?", '\u{00AD}'),
    ];

    /// A list of all shorthands in math mode.
    pub const MATH_LIST: &[(&'static str, char)] = &[
        ("...", '…'),
        ("-", '\u{2212}'),
        ("'", '′'),
        ("*", '∗'),
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
    pub fn get(&self) -> char {
        let text = self.0.text();
        (Self::MARKUP_LIST.iter().chain(Self::MATH_LIST))
            .find(|&&(s, _)| s == text)
            .map_or_else(char::default, |&(_, c)| c)
    }
}

node! {
    /// A smart quote: `'` or `"`.
    SmartQuote
}

impl SmartQuote {
    /// Whether this is a double quote.
    pub fn double(&self) -> bool {
        self.0.text() == "\""
    }
}

node! {
    /// Strong content: `*Strong*`.
    Strong
}

impl Strong {
    /// The contents of the strong node.
    pub fn body(&self) -> Markup {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// Emphasized content: `_Emphasized_`.
    Emph
}

impl Emph {
    /// The contents of the emphasis node.
    pub fn body(&self) -> Markup {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// Raw text with optional syntax highlighting: `` `...` ``.
    Raw
}

impl Raw {
    /// The trimmed raw text.
    pub fn text(&self) -> EcoString {
        let mut text = self.0.text().as_str();
        let blocky = text.starts_with("```");
        text = text.trim_matches('`');

        // Trim tag, one space at the start, and one space at the end if the
        // last non-whitespace char is a backtick.
        if blocky {
            let mut s = Scanner::new(text);
            if s.eat_if(is_id_start) {
                s.eat_while(is_id_continue);
            }
            text = s.after();
            text = text.strip_prefix(' ').unwrap_or(text);
            if text.trim_end().ends_with('`') {
                text = text.strip_suffix(' ').unwrap_or(text);
            }
        }

        // Split into lines.
        let mut lines = split_newlines(text);

        if blocky {
            let dedent = lines
                .iter()
                .skip(1)
                .filter(|line| !line.chars().all(char::is_whitespace))
                // The line with the closing ``` is always taken into account
                .chain(lines.last())
                .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
                .min()
                .unwrap_or(0);

            // Dedent based on column, but not for the first line.
            for line in lines.iter_mut().skip(1) {
                let offset = line.chars().take(dedent).map(char::len_utf8).sum();
                *line = &line[offset..];
            }

            let is_whitespace = |line: &&str| line.chars().all(char::is_whitespace);

            // Trims a sequence of whitespace followed by a newline at the start.
            if lines.first().map_or(false, is_whitespace) {
                lines.remove(0);
            }

            // Trims a newline followed by a sequence of whitespace at the end.
            if lines.last().map_or(false, is_whitespace) {
                lines.pop();
            }
        }

        lines.join("\n").into()
    }

    /// An optional identifier specifying the language to syntax-highlight in.
    pub fn lang(&self) -> Option<&str> {
        let text = self.0.text();

        // Only blocky literals are supposed to contain a language.
        if !text.starts_with("```") {
            return Option::None;
        }

        let inner = text.trim_start_matches('`');
        let mut s = Scanner::new(inner);
        s.eat_if(is_id_start).then(|| {
            s.eat_while(is_id_continue);
            s.before()
        })
    }

    /// Whether the raw text should be displayed in a separate block.
    pub fn block(&self) -> bool {
        let text = self.0.text();
        text.starts_with("```") && text.chars().any(is_newline)
    }
}

node! {
    /// A hyperlink: `https://typst.org`.
    Link
}

impl Link {
    /// Get the URL.
    pub fn get(&self) -> &EcoString {
        self.0.text()
    }
}

node! {
    /// A label: `<intro>`.
    Label
}

impl Label {
    /// Get the label's text.
    pub fn get(&self) -> &str {
        self.0.text().trim_start_matches('<').trim_end_matches('>')
    }
}

node! {
    /// A reference: `@target`, `@target[..]`.
    Ref
}

impl Ref {
    /// Get the target.
    pub fn target(&self) -> &str {
        self.0
            .children()
            .find(|node| node.kind() == SyntaxKind::RefMarker)
            .map(|node| node.text().trim_start_matches('@'))
            .unwrap_or_default()
    }

    /// Get the supplement.
    pub fn supplement(&self) -> Option<ContentBlock> {
        self.0.cast_last_match()
    }
}

node! {
    /// A section heading: `= Introduction`.
    Heading
}

impl Heading {
    /// The contents of the heading.
    pub fn body(&self) -> Markup {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The section depth (number of equals signs).
    pub fn level(&self) -> NonZeroUsize {
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

impl ListItem {
    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// An item in an enumeration (numbered list): `+ ...` or `1. ...`.
    EnumItem
}

impl EnumItem {
    /// The explicit numbering, if any: `23.`.
    pub fn number(&self) -> Option<usize> {
        self.0.children().find_map(|node| match node.kind() {
            SyntaxKind::EnumMarker => node.text().trim_end_matches('.').parse().ok(),
            _ => Option::None,
        })
    }

    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// An item in a term list: `/ Term: Details`.
    TermItem
}

impl TermItem {
    /// The term described by the item.
    pub fn term(&self) -> Markup {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The description of the term.
    pub fn description(&self) -> Markup {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A mathemathical equation: `$x$`, `$ x^2 $`.
    Equation
}

impl Equation {
    /// The contained math.
    pub fn body(&self) -> Math {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// Whether the equation should be displayed as a separate block.
    pub fn block(&self) -> bool {
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

impl Math {
    /// The expressions the mathematical content consists of.
    pub fn exprs(&self) -> impl DoubleEndedIterator<Item = Expr> + '_ {
        self.0.children().filter_map(Expr::cast_with_space)
    }
}

node! {
    /// An identifier in math: `pi`.
    MathIdent
}

impl MathIdent {
    /// Get the identifier.
    pub fn get(&self) -> &EcoString {
        self.0.text()
    }

    /// Take out the contained identifier.
    pub fn take(self) -> EcoString {
        self.0.into_text()
    }

    /// Get the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        self.get()
    }
}

impl Deref for MathIdent {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
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

impl MathDelimited {
    /// The opening delimiter.
    pub fn open(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The contents, including the delimiters.
    pub fn body(&self) -> Math {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The closing delimiter.
    pub fn close(&self) -> Expr {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A base with optional attachments in math: `a_1^2`.
    MathAttach
}

impl MathAttach {
    /// The base, to which things are attached.
    pub fn base(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The bottom attachment.
    pub fn bottom(&self) -> Option<Expr> {
        self.0
            .children()
            .skip_while(|node| !matches!(node.kind(), SyntaxKind::Underscore))
            .find_map(SyntaxNode::cast)
    }

    /// The top attachment.
    pub fn top(&self) -> Option<Expr> {
        self.0
            .children()
            .skip_while(|node| !matches!(node.kind(), SyntaxKind::Hat))
            .find_map(SyntaxNode::cast)
    }

    /// Extract primes if present.
    pub fn primes(&self) -> Option<MathPrimes> {
        self.0.cast_first_match()
    }
}

node! {
    /// Grouped primes in math: `a'''`.
    MathPrimes
}

impl MathPrimes {
    pub fn count(&self) -> usize {
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

impl MathFrac {
    /// The numerator.
    pub fn num(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The denominator.
    pub fn denom(&self) -> Expr {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A root in math: `√x`, `∛x` or `∜x`.
    MathRoot
}

impl MathRoot {
    /// The index of the root.
    pub fn index(&self) -> Option<usize> {
        match self.0.children().next().map(|node| node.text().as_str()) {
            Some("∜") => Some(4),
            Some("∛") => Some(3),
            Some("√") => Option::None,
            _ => Option::None,
        }
    }

    /// The radicand.
    pub fn radicand(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// An identifier: `it`.
    Ident
}

impl Ident {
    /// Get the identifier.
    pub fn get(&self) -> &EcoString {
        self.0.text()
    }

    /// Take out the contained identifier.
    pub fn take(self) -> EcoString {
        self.0.into_text()
    }

    /// Get the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        self.get()
    }
}

impl Deref for Ident {
    type Target = str;

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

impl Bool {
    /// Get the boolean value.
    pub fn get(&self) -> bool {
        self.0.text() == "true"
    }
}

node! {
    /// An integer: `120`.
    Int
}

impl Int {
    /// Get the integer value.
    pub fn get(&self) -> i64 {
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

impl Float {
    /// Get the floating-point value.
    pub fn get(&self) -> f64 {
        self.0.text().parse().unwrap_or_default()
    }
}

node! {
    /// A numeric value with a unit: `12pt`, `3cm`, `2em`, `90deg`, `50%`.
    Numeric
}

impl Numeric {
    /// Get the numeric value and unit.
    pub fn get(&self) -> (f64, Unit) {
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

impl Str {
    /// Get the string value with resolved escape sequences.
    pub fn get(&self) -> EcoString {
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

impl CodeBlock {
    /// The contained code.
    pub fn body(&self) -> Code {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// Code.
    Code
}

impl Code {
    /// The list of expressions contained in the code.
    pub fn exprs(&self) -> impl DoubleEndedIterator<Item = Expr> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

node! {
    /// A content block: `[*Hi* there!]`.
    ContentBlock
}

impl ContentBlock {
    /// The contained markup.
    pub fn body(&self) -> Markup {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// A grouped expression: `(1 + 2)`.
    Parenthesized
}

impl Parenthesized {
    /// The wrapped expression.
    pub fn expr(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }
}

node! {
    /// An array: `(1, "hi", 12cm)`.
    Array
}

impl Array {
    /// The array's items.
    pub fn items(&self) -> impl DoubleEndedIterator<Item = ArrayItem> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

/// An item in an array.
#[derive(Debug, Clone, Hash)]
pub enum ArrayItem {
    /// A bare expression: `12`.
    Pos(Expr),
    /// A spread expression: `..things`.
    Spread(Expr),
}

impl AstNode for ArrayItem {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Spread => node.cast_first_match().map(Self::Spread),
            _ => node.cast().map(Self::Pos),
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Pos(v) => v.as_untyped(),
            Self::Spread(v) => v.as_untyped(),
        }
    }
}

node! {
    /// A dictionary: `(thickness: 3pt, pattern: dashed)`.
    Dict
}

impl Dict {
    /// The dictionary's items.
    pub fn items(&self) -> impl DoubleEndedIterator<Item = DictItem> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

/// An item in an dictionary expression.
#[derive(Debug, Clone, Hash)]
pub enum DictItem {
    /// A named pair: `thickness: 3pt`.
    Named(Named),
    /// A keyed pair: `"spacy key": true`.
    Keyed(Keyed),
    /// A spread expression: `..things`.
    Spread(Expr),
}

impl AstNode for DictItem {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Named => node.cast().map(Self::Named),
            SyntaxKind::Keyed => node.cast().map(Self::Keyed),
            SyntaxKind::Spread => node.cast_first_match().map(Self::Spread),
            _ => Option::None,
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Named(v) => v.as_untyped(),
            Self::Keyed(v) => v.as_untyped(),
            Self::Spread(v) => v.as_untyped(),
        }
    }
}

node! {
    /// A named pair: `thickness: 3pt`.
    Named
}

impl Named {
    /// The name: `thickness`.
    pub fn name(&self) -> Ident {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The right-hand side of the pair: `3pt`.
    pub fn expr(&self) -> Expr {
        self.0.cast_last_match().unwrap_or_default()
    }

    /// The right-hand side of the pair as an identifier.
    pub fn expr_ident(&self) -> Option<Ident> {
        self.0.cast_last_match()
    }
}

node! {
    /// A keyed pair: `"spacy key": true`.
    Keyed
}

impl Keyed {
    /// The key: `"spacy key"`.
    pub fn key(&self) -> Str {
        self.0
            .children()
            .find_map(|node| node.cast::<Str>())
            .unwrap_or_default()
    }

    /// The right-hand side of the pair: `true`.
    pub fn expr(&self) -> Expr {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A unary operation: `-x`.
    Unary
}

impl Unary {
    /// The operator: `-`.
    pub fn op(&self) -> UnOp {
        self.0
            .children()
            .find_map(|node| UnOp::from_kind(node.kind()))
            .unwrap_or(UnOp::Pos)
    }

    /// The expression to operate on: `x`.
    pub fn expr(&self) -> Expr {
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

impl Binary {
    /// The binary operator: `+`.
    pub fn op(&self) -> BinOp {
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
    pub fn lhs(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The right-hand side of the operation: `b`.
    pub fn rhs(&self) -> Expr {
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
    /// The inversed containment operator: `not in`.
    NotIn,
    /// The add-assign operator: `+=`.
    AddAssign,
    /// The subtract-assign oeprator: `-=`.
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

impl FieldAccess {
    /// The expression to access the field on.
    pub fn target(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The name of the field.
    pub fn field(&self) -> Ident {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// An invocation of a function or method: `f(x, y)`.
    FuncCall
}

impl FuncCall {
    /// The function to call.
    pub fn callee(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The arguments to the function.
    pub fn args(&self) -> Args {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A function call's argument list: `(12pt, y)`.
    Args
}

impl Args {
    /// The positional and named arguments.
    pub fn items(&self) -> impl DoubleEndedIterator<Item = Arg> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

/// An argument to a function call.
#[derive(Debug, Clone, Hash)]
pub enum Arg {
    /// A positional argument: `12`.
    Pos(Expr),
    /// A named argument: `draw: false`.
    Named(Named),
    /// A spread argument: `..things`.
    Spread(Expr),
}

impl AstNode for Arg {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Named => node.cast().map(Self::Named),
            SyntaxKind::Spread => node.cast_first_match().map(Self::Spread),
            _ => node.cast().map(Self::Pos),
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Pos(v) => v.as_untyped(),
            Self::Named(v) => v.as_untyped(),
            Self::Spread(v) => v.as_untyped(),
        }
    }
}

node! {
    /// A closure: `(x, y) => z`.
    Closure
}

impl Closure {
    /// The name of the closure.
    ///
    /// This only exists if you use the function syntax sugar: `let f(x) = y`.
    pub fn name(&self) -> Option<Ident> {
        self.0.children().next()?.cast()
    }

    /// The parameter bindings.
    pub fn params(&self) -> Params {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The body of the closure.
    pub fn body(&self) -> Expr {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A closure's parameters: `(x, y)`.
    Params
}

impl Params {
    /// The parameter bindings.
    pub fn children(&self) -> impl DoubleEndedIterator<Item = Param> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

node! {
    /// A spread: `..x` or `..x.at(0)`.
    Spread
}

impl Spread {
    /// Try to get an identifier.
    pub fn name(&self) -> Option<Ident> {
        self.0.cast_first_match()
    }

    /// Try to get an expression.
    pub fn expr(&self) -> Option<Expr> {
        self.0.cast_first_match()
    }
}

node! {
    /// An underscore: `_`
    Underscore
}

/// A parameter to a closure.
#[derive(Debug, Clone, Hash)]
pub enum Param {
    /// A positional parameter: `x`.
    Pos(Pattern),
    /// A named parameter with a default value: `draw: false`.
    Named(Named),
    /// An argument sink: `..args`.
    Sink(Spread),
}

impl AstNode for Param {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Named => node.cast().map(Self::Named),
            SyntaxKind::Spread => node.cast().map(Self::Sink),
            _ => node.cast().map(Self::Pos),
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Pos(v) => v.as_untyped(),
            Self::Named(v) => v.as_untyped(),
            Self::Sink(v) => v.as_untyped(),
        }
    }
}

node! {
    /// A destructuring pattern: `x` or `(x, _, ..y)`.
    Destructuring
}

impl Destructuring {
    /// The bindings of the destructuring.
    pub fn bindings(&self) -> impl Iterator<Item = DestructuringKind> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }

    // Returns a list of all identifiers in the pattern.
    pub fn idents(&self) -> impl Iterator<Item = Ident> + '_ {
        self.bindings().filter_map(|binding| match binding {
            DestructuringKind::Normal(Expr::Ident(ident)) => Some(ident),
            DestructuringKind::Sink(spread) => spread.name(),
            DestructuringKind::Named(named) => named.expr_ident(),
            _ => Option::None,
        })
    }
}

/// The kind of an element in a destructuring pattern.
#[derive(Debug, Clone, Hash)]
pub enum DestructuringKind {
    /// An expression: `x`.
    Normal(Expr),
    /// An argument sink: `..y`.
    Sink(Spread),
    /// Named arguments: `x: 1`.
    Named(Named),
    /// A placeholder: `_`.
    Placeholder(Underscore),
}

impl AstNode for DestructuringKind {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Named => node.cast().map(Self::Named),
            SyntaxKind::Spread => node.cast().map(Self::Sink),
            SyntaxKind::Underscore => node.cast().map(Self::Placeholder),
            _ => node.cast().map(Self::Normal),
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Normal(v) => v.as_untyped(),
            Self::Named(v) => v.as_untyped(),
            Self::Sink(v) => v.as_untyped(),
            Self::Placeholder(v) => v.as_untyped(),
        }
    }
}

/// The kind of a pattern.
#[derive(Debug, Clone, Hash)]
pub enum Pattern {
    /// A single expression: `x`.
    Normal(Expr),
    /// A placeholder: `_`.
    Placeholder(Underscore),
    /// A destructuring pattern: `(x, _, ..y)`.
    Destructuring(Destructuring),
}

impl AstNode for Pattern {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Destructuring => node.cast().map(Self::Destructuring),
            SyntaxKind::Underscore => node.cast().map(Self::Placeholder),
            _ => node.cast().map(Self::Normal),
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Normal(v) => v.as_untyped(),
            Self::Destructuring(v) => v.as_untyped(),
            Self::Placeholder(v) => v.as_untyped(),
        }
    }
}

impl Pattern {
    // Returns a list of all identifiers in the pattern.
    pub fn idents(&self) -> Vec<Ident> {
        match self {
            Pattern::Normal(Expr::Ident(ident)) => vec![ident.clone()],
            Pattern::Destructuring(destruct) => destruct.idents().collect(),
            _ => vec![],
        }
    }
}

impl Default for Pattern {
    fn default() -> Self {
        Self::Normal(Expr::default())
    }
}

node! {
    /// A let binding: `let x = 1`.
    LetBinding
}

#[derive(Debug)]
pub enum LetBindingKind {
    /// A normal binding: `let x = 1`.
    Normal(Pattern),
    /// A closure binding: `let f(x) = 1`.
    Closure(Ident),
}

impl LetBindingKind {
    // Returns a list of all identifiers in the pattern.
    pub fn idents(&self) -> Vec<Ident> {
        match self {
            LetBindingKind::Normal(pattern) => pattern.idents(),
            LetBindingKind::Closure(ident) => {
                vec![ident.clone()]
            }
        }
    }
}

impl LetBinding {
    /// The kind of the let binding.
    pub fn kind(&self) -> LetBindingKind {
        match self.0.cast_first_match::<Pattern>() {
            Some(Pattern::Normal(Expr::Closure(closure))) => {
                LetBindingKind::Closure(closure.name().unwrap_or_default())
            }
            pattern => LetBindingKind::Normal(pattern.unwrap_or_default()),
        }
    }

    /// The expression the binding is initialized with.
    pub fn init(&self) -> Option<Expr> {
        match self.kind() {
            LetBindingKind::Normal(Pattern::Normal(_)) => {
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

impl DestructAssignment {
    /// The pattern of the assignment.
    pub fn pattern(&self) -> Pattern {
        self.0.cast_first_match::<Pattern>().unwrap_or_default()
    }

    /// The expression that is assigned.
    pub fn value(&self) -> Expr {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A set rule: `set text(...)`.
    SetRule
}

impl SetRule {
    /// The function to set style properties for.
    pub fn target(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The style properties to set.
    pub fn args(&self) -> Args {
        self.0.cast_last_match().unwrap_or_default()
    }

    /// A condition under which the set rule applies.
    pub fn condition(&self) -> Option<Expr> {
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

impl ShowRule {
    /// Defines which nodes the show rule applies to.
    pub fn selector(&self) -> Option<Expr> {
        self.0
            .children()
            .rev()
            .skip_while(|child| child.kind() != SyntaxKind::Colon)
            .find_map(SyntaxNode::cast)
    }

    /// The transformation recipe.
    pub fn transform(&self) -> Expr {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// An if-else conditional: `if x { y } else { z }`.
    Conditional
}

impl Conditional {
    /// The condition which selects the body to evaluate.
    pub fn condition(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The expression to evaluate if the condition is true.
    pub fn if_body(&self) -> Expr {
        self.0
            .children()
            .filter_map(SyntaxNode::cast)
            .nth(1)
            .unwrap_or_default()
    }

    /// The expression to evaluate if the condition is false.
    pub fn else_body(&self) -> Option<Expr> {
        self.0.children().filter_map(SyntaxNode::cast).nth(2)
    }
}

node! {
    /// A while loop: `while x { y }`.
    WhileLoop
}

impl WhileLoop {
    /// The condition which selects whether to evaluate the body.
    pub fn condition(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The expression to evaluate while the condition is true.
    pub fn body(&self) -> Expr {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A for loop: `for x in y { z }`.
    ForLoop
}

impl ForLoop {
    /// The pattern to assign to.
    pub fn pattern(&self) -> Pattern {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The expression to iterate over.
    pub fn iter(&self) -> Expr {
        self.0
            .children()
            .skip_while(|&c| c.kind() != SyntaxKind::In)
            .find_map(SyntaxNode::cast)
            .unwrap_or_default()
    }

    /// The expression to evaluate for each iteration.
    pub fn body(&self) -> Expr {
        self.0.cast_last_match().unwrap_or_default()
    }
}

node! {
    /// A module import: `import "utils.typ": a, b, c`.
    ModuleImport
}

impl ModuleImport {
    /// The module or path from which the items should be imported.
    pub fn source(&self) -> Expr {
        self.0.cast_first_match().unwrap_or_default()
    }

    /// The items to be imported.
    pub fn imports(&self) -> Option<Imports> {
        self.0.children().find_map(|node| match node.kind() {
            SyntaxKind::Star => Some(Imports::Wildcard),
            SyntaxKind::ImportItems => {
                let items = node.children().filter_map(SyntaxNode::cast).collect();
                Some(Imports::Items(items))
            }
            _ => Option::None,
        })
    }
}

/// The items that ought to be imported from a file.
#[derive(Debug, Clone, Hash)]
pub enum Imports {
    /// All items in the scope of the file should be imported.
    Wildcard,
    /// The specified items from the file should be imported.
    Items(Vec<Ident>),
}

node! {
    /// A module include: `include "chapter1.typ"`.
    ModuleInclude
}

impl ModuleInclude {
    /// The module or path from which the content should be included.
    pub fn source(&self) -> Expr {
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

impl FuncReturn {
    /// The expression to return.
    pub fn body(&self) -> Option<Expr> {
        self.0.cast_last_match()
    }
}
