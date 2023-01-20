//! A typed layer over the untyped syntax tree.
//!
//! The AST is rooted in the [`Markup`] node.

use std::num::NonZeroUsize;
use std::ops::Deref;

use unscanny::Scanner;

use super::{
    is_id_continue, is_id_start, is_newline, split_newlines, Span, SyntaxKind, SyntaxNode,
};
use crate::geom::{AbsUnit, AngleUnit};
use crate::util::EcoString;

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
        #[derive(Debug, Clone, PartialEq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Hash)]
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
    /// Symbol notation: `:arrow:l:`.
    Symbol(Symbol),
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
    /// A reference: `@target`.
    Ref(Ref),
    /// A section heading: `= Introduction`.
    Heading(Heading),
    /// An item in a bullet list: `- ...`.
    List(ListItem),
    /// An item in an enumeration (numbered list): `+ ...` or `1. ...`.
    Enum(EnumItem),
    /// An item in a term list: `/ Term: Details`.
    Term(TermItem),
    /// A math formula: `$x$`, `$ x^2 $`.
    Math(Math),
    /// An atom in a math formula: `x`, `+`, `12`.
    Atom(Atom),
    /// A base with optional sub- and superscripts in a math formula: `a_1^2`.
    Script(Script),
    /// A fraction in a math formula: `x/2`.
    Frac(Frac),
    /// An alignment point in a math formula: `&`.
    AlignPoint(AlignPoint),
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
    /// An invocation of a function: `f(x, y)`.
    FuncCall(FuncCall),
    /// An invocation of a method: `array.push(v)`.
    MethodCall(MethodCall),
    /// A closure: `(x, y) => z`.
    Closure(Closure),
    /// A let binding: `let x = 1`.
    Let(LetBinding),
    /// A set rule: `set text(...)`.
    Set(SetRule),
    /// A show rule: `show heading: it => [*{it.body}*]`.
    Show(ShowRule),
    /// An if-else conditional: `if x { y } else { z }`.
    Conditional(Conditional),
    /// A while loop: `while x { y }`.
    While(WhileLoop),
    /// A for loop: `for x in y { z }`.
    For(ForLoop),
    /// A module import: `import a, b, c from "utils.typ"`.
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
            SyntaxKind::Symbol => node.cast().map(Self::Symbol),
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
            SyntaxKind::Math => node.cast().map(Self::Math),
            SyntaxKind::Atom => node.cast().map(Self::Atom),
            SyntaxKind::Script => node.cast().map(Self::Script),
            SyntaxKind::Frac => node.cast().map(Self::Frac),
            SyntaxKind::AlignPoint => node.cast().map(Self::AlignPoint),
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
            SyntaxKind::MethodCall => node.cast().map(Self::MethodCall),
            SyntaxKind::Closure => node.cast().map(Self::Closure),
            SyntaxKind::LetBinding => node.cast().map(Self::Let),
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
            Self::Symbol(v) => v.as_untyped(),
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
            Self::Math(v) => v.as_untyped(),
            Self::Atom(v) => v.as_untyped(),
            Self::Script(v) => v.as_untyped(),
            Self::Frac(v) => v.as_untyped(),
            Self::AlignPoint(v) => v.as_untyped(),
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
            Self::MethodCall(v) => v.as_untyped(),
            Self::Closure(v) => v.as_untyped(),
            Self::Let(v) => v.as_untyped(),
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
                .expect("unicode escape is invalid")
        } else {
            s.eat().expect("escape is missing escaped character")
        }
    }
}

node! {
    /// A shorthand for a unicode codepoint. For example, `~` for a non-breaking
    /// space or `-?` for a soft hyphen.
    Shorthand
}

impl Shorthand {
    /// Get the shorthanded character.
    pub fn get(&self) -> char {
        match self.0.text().as_str() {
            "~" => '\u{00A0}',
            "..." => '\u{2026}',
            "--" => '\u{2013}',
            "---" => '\u{2014}',
            "-?" => '\u{00AD}',
            "!=" => '≠',
            "<=" => '≤',
            ">=" => '≥',
            "<-" => '←',
            "->" => '→',
            "=>" => '⇒',
            ":=" => '≔',
            "[|" => '⟦',
            "|]" => '⟧',
            "||" => '‖',
            "|->" => '↦',
            "<->" => '↔',
            "<=>" => '⇔',
            _ => panic!("shorthand is invalid"),
        }
    }
}

node! {
    /// Symbol notation: `:arrow:l:`.
    Symbol
}

impl Symbol {
    /// Get the symbol's notation.
    pub fn get(&self) -> &str {
        self.0.text().trim_matches(':')
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
        self.0.cast_first_match().expect("strong emphasis is missing body")
    }
}

node! {
    /// Emphasized content: `_Emphasized_`.
    Emph
}

impl Emph {
    /// The contents of the emphasis node.
    pub fn body(&self) -> Markup {
        self.0.cast_first_match().expect("emphasis is missing body")
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
        let inner = self.0.text().trim_start_matches('`');
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
    /// A reference: `@target`.
    Ref
}

impl Ref {
    /// Get the target.
    pub fn get(&self) -> &str {
        self.0.text().trim_start_matches('@')
    }
}

node! {
    /// A section heading: `= Introduction`.
    Heading
}

impl Heading {
    /// The contents of the heading.
    pub fn body(&self) -> Markup {
        self.0.cast_first_match().expect("heading is missing markup body")
    }

    /// The section depth (numer of equals signs).
    pub fn level(&self) -> NonZeroUsize {
        self.0
            .children()
            .find(|node| node.kind() == SyntaxKind::HeadingMarker)
            .and_then(|node| node.len().try_into().ok())
            .expect("heading is missing marker")
    }
}

node! {
    /// An item in a bullet list: `- ...`.
    ListItem
}

impl ListItem {
    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_match().expect("list item is missing body")
    }
}

node! {
    /// An item in an enumeration (numbered list): `+ ...` or `1. ...`.
    EnumItem
}

impl EnumItem {
    /// The explicit numbering, if any: `23.`.
    pub fn number(&self) -> Option<NonZeroUsize> {
        self.0.children().find_map(|node| match node.kind() {
            SyntaxKind::EnumMarker => node.text().trim_end_matches('.').parse().ok(),
            _ => Option::None,
        })
    }

    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_match().expect("enum item is missing body")
    }
}

node! {
    /// An item in a term list: `/ Term: Details`.
    TermItem
}

impl TermItem {
    /// The term described by the item.
    pub fn term(&self) -> Markup {
        self.0.cast_first_match().expect("term list item is missing term")
    }

    /// The description of the term.
    pub fn description(&self) -> Markup {
        self.0
            .cast_last_match()
            .expect("term list item is missing description")
    }
}

node! {
    /// A math formula: `$x$`, `$ x^2 $`.
    Math
}

impl Math {
    /// The expressions the formula consists of.
    pub fn exprs(&self) -> impl DoubleEndedIterator<Item = Expr> + '_ {
        self.0.children().filter_map(Expr::cast_with_space)
    }

    /// Whether the formula should be displayed as a separate block.
    pub fn block(&self) -> bool {
        matches!(self.exprs().next(), Some(Expr::Space(_)))
            && matches!(self.exprs().last(), Some(Expr::Space(_)))
    }
}

node! {
    /// A atom in a formula: `x`, `+`, `12`.
    Atom
}

impl Atom {
    /// Get the atom's text.
    pub fn get(&self) -> &EcoString {
        self.0.text()
    }
}

node! {
    /// A base with an optional sub- and superscript in a formula: `a_1^2`.
    Script
}

impl Script {
    /// The base of the script.
    pub fn base(&self) -> Expr {
        self.0.cast_first_match().expect("script node is missing base")
    }

    /// The subscript.
    pub fn sub(&self) -> Option<Expr> {
        self.0
            .children()
            .skip_while(|node| !matches!(node.kind(), SyntaxKind::Underscore))
            .nth(1)
            .map(|node| node.cast().expect("script node has invalid subscript"))
    }

    /// The superscript.
    pub fn sup(&self) -> Option<Expr> {
        self.0
            .children()
            .skip_while(|node| !matches!(node.kind(), SyntaxKind::Hat))
            .nth(1)
            .map(|node| node.cast().expect("script node has invalid superscript"))
    }
}

node! {
    /// A fraction in a formula: `x/2`
    Frac
}

impl Frac {
    /// The numerator.
    pub fn num(&self) -> Expr {
        self.0.cast_first_match().expect("fraction is missing numerator")
    }

    /// The denominator.
    pub fn denom(&self) -> Expr {
        self.0.cast_last_match().expect("fraction is missing denominator")
    }
}

node! {
    /// An alignment point in a formula: `&`.
    AlignPoint
}

node! {
    /// An identifier: `it`.
    Ident
}

impl Ident {
    /// Get the identifier.
    pub fn get(&self) -> &str {
        self.0.text().trim_start_matches('#')
    }

    /// Take out the contained identifier.
    pub fn take(self) -> EcoString {
        let text = self.0.into_text();
        match text.strip_prefix('#') {
            Some(text) => text.into(),
            Option::None => text,
        }
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
        self.0.text().parse().expect("integer is invalid")
    }
}

node! {
    /// A floating-point number: `1.2`, `10e-4`.
    Float
}

impl Float {
    /// Get the floating-point value.
    pub fn get(&self) -> f64 {
        self.0.text().parse().expect("float is invalid")
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
        let value = text[..split].parse().expect("number is invalid");
        let unit = match &text[split..] {
            "pt" => Unit::Length(AbsUnit::Pt),
            "mm" => Unit::Length(AbsUnit::Mm),
            "cm" => Unit::Length(AbsUnit::Cm),
            "in" => Unit::Length(AbsUnit::In),
            "deg" => Unit::Angle(AngleUnit::Deg),
            "rad" => Unit::Angle(AngleUnit::Rad),
            "em" => Unit::Em,
            "fr" => Unit::Fr,
            "%" => Unit::Percent,
            _ => panic!("number has invalid suffix"),
        };

        (value, unit)
    }
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
    /// The list of expressions contained in the block.
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
        self.0.cast_first_match().expect("content block is missing body")
    }
}

node! {
    /// A grouped expression: `(1 + 2)`.
    Parenthesized
}

impl Parenthesized {
    /// The wrapped expression.
    pub fn expr(&self) -> Expr {
        self.0
            .cast_first_match()
            .expect("parenthesized expression is missing expression")
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
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ArrayItem {
    /// A bare expression: `12`.
    Pos(Expr),
    /// A spreaded expression: `..things`.
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

/// An item in an dictionary expresssion.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum DictItem {
    /// A named pair: `thickness: 3pt`.
    Named(Named),
    /// A keyed pair: `"spacy key": true`.
    Keyed(Keyed),
    /// A spreaded expression: `..things`.
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
        self.0.cast_first_match().expect("named pair is missing name")
    }

    /// The right-hand side of the pair: `3pt`.
    pub fn expr(&self) -> Expr {
        self.0.cast_last_match().expect("named pair is missing expression")
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
            .expect("keyed pair is missing key")
    }

    /// The right-hand side of the pair: `true`.
    pub fn expr(&self) -> Expr {
        self.0.cast_last_match().expect("keyed pair is missing expression")
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
            .expect("unary operation is missing operator")
    }

    /// The expression to operate on: `x`.
    pub fn expr(&self) -> Expr {
        self.0.cast_last_match().expect("unary operation is missing child")
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
            .expect("binary operation is missing operator")
    }

    /// The left-hand side of the operation: `a`.
    pub fn lhs(&self) -> Expr {
        self.0
            .cast_first_match()
            .expect("binary operation is missing left-hand side")
    }

    /// The right-hand side of the operation: `b`.
    pub fn rhs(&self) -> Expr {
        self.0
            .cast_last_match()
            .expect("binary operation is missing right-hand side")
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
        self.0.cast_first_match().expect("field access is missing object")
    }

    /// The name of the field.
    pub fn field(&self) -> Ident {
        self.0.cast_last_match().expect("field access is missing name")
    }
}

node! {
    /// An invocation of a function: `f(x, y)`.
    FuncCall
}

impl FuncCall {
    /// The function to call.
    pub fn callee(&self) -> Expr {
        self.0.cast_first_match().expect("function call is missing callee")
    }

    /// The arguments to the function.
    pub fn args(&self) -> Args {
        self.0
            .cast_last_match()
            .expect("function call is missing argument list")
    }
}

node! {
    /// An invocation of a method: `array.push(v)`.
    MethodCall
}

impl MethodCall {
    /// The expression to call the method on.
    pub fn target(&self) -> Expr {
        self.0.cast_first_match().expect("method call is missing target")
    }

    /// The name of the method.
    pub fn method(&self) -> Ident {
        self.0.cast_last_match().expect("method call is missing name")
    }

    /// The arguments to the method.
    pub fn args(&self) -> Args {
        self.0
            .cast_last_match()
            .expect("method call is missing argument list")
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
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Arg {
    /// A positional argument: `12`.
    Pos(Expr),
    /// A named argument: `draw: false`.
    Named(Named),
    /// A spreaded argument: `..things`.
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
    pub fn params(&self) -> impl DoubleEndedIterator<Item = Param> + '_ {
        self.0
            .children()
            .find(|x| x.kind() == SyntaxKind::Params)
            .expect("closure is missing parameter list")
            .children()
            .filter_map(SyntaxNode::cast)
    }

    /// The body of the closure.
    pub fn body(&self) -> Expr {
        self.0.cast_last_match().expect("closure is missing body")
    }
}

/// A parameter to a closure.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Param {
    /// A positional parameter: `x`.
    Pos(Ident),
    /// A named parameter with a default value: `draw: false`.
    Named(Named),
    /// An argument sink: `..args`.
    Sink(Ident),
}

impl AstNode for Param {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Ident => node.cast().map(Self::Pos),
            SyntaxKind::Named => node.cast().map(Self::Named),
            SyntaxKind::Spread => node.cast_first_match().map(Self::Sink),
            _ => Option::None,
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
    /// A let binding: `let x = 1`.
    LetBinding
}

impl LetBinding {
    /// The binding to assign to.
    pub fn binding(&self) -> Ident {
        match self.0.cast_first_match() {
            Some(Expr::Ident(binding)) => binding,
            Some(Expr::Closure(closure)) => {
                closure.name().expect("let-bound closure is missing name")
            }
            _ => panic!("let is missing binding"),
        }
    }

    /// The expression the binding is initialized with.
    pub fn init(&self) -> Option<Expr> {
        if self.0.cast_first_match::<Ident>().is_some() {
            // This is a normal binding like `let x = 1`.
            self.0.children().filter_map(SyntaxNode::cast).nth(1)
        } else {
            // This is a closure binding like `let f(x) = 1`.
            self.0.cast_first_match()
        }
    }
}

node! {
    /// A set rule: `set text(...)`.
    SetRule
}

impl SetRule {
    /// The function to set style properties for.
    pub fn target(&self) -> Ident {
        self.0.cast_first_match().expect("set rule is missing target")
    }

    /// The style properties to set.
    pub fn args(&self) -> Args {
        self.0.cast_last_match().expect("set rule is missing argument list")
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
    /// A show rule: `show heading: it => [*{it.body}*]`.
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
        self.0.cast_last_match().expect("show rule is missing transform")
    }
}

node! {
    /// An if-else conditional: `if x { y } else { z }`.
    Conditional
}

impl Conditional {
    /// The condition which selects the body to evaluate.
    pub fn condition(&self) -> Expr {
        self.0.cast_first_match().expect("conditional is missing condition")
    }

    /// The expression to evaluate if the condition is true.
    pub fn if_body(&self) -> Expr {
        self.0
            .children()
            .filter_map(SyntaxNode::cast)
            .nth(1)
            .expect("conditional is missing body")
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
        self.0.cast_first_match().expect("while loop is missing condition")
    }

    /// The expression to evaluate while the condition is true.
    pub fn body(&self) -> Expr {
        self.0.cast_last_match().expect("while loop is missing body")
    }
}

node! {
    /// A for loop: `for x in y { z }`.
    ForLoop
}

impl ForLoop {
    /// The pattern to assign to.
    pub fn pattern(&self) -> ForPattern {
        self.0.cast_first_match().expect("for loop is missing pattern")
    }

    /// The expression to iterate over.
    pub fn iter(&self) -> Expr {
        self.0.cast_first_match().expect("for loop is missing iterable")
    }

    /// The expression to evaluate for each iteration.
    pub fn body(&self) -> Expr {
        self.0.cast_last_match().expect("for loop is missing body")
    }
}

node! {
    /// A for loop's destructuring pattern: `x` or `x, y`.
    ForPattern
}

impl ForPattern {
    /// The key part of the pattern: index for arrays, name for dictionaries.
    pub fn key(&self) -> Option<Ident> {
        let mut children = self.0.children().filter_map(SyntaxNode::cast);
        let key = children.next();
        if children.next().is_some() {
            key
        } else {
            Option::None
        }
    }

    /// The value part of the pattern.
    pub fn value(&self) -> Ident {
        self.0.cast_last_match().expect("for loop pattern is missing value")
    }
}

node! {
    /// A module import: `import "utils.typ": a, b, c`.
    ModuleImport
}

impl ModuleImport {
    /// The module or path from which the items should be imported.
    pub fn source(&self) -> Expr {
        self.0.cast_last_match().expect("module import is missing source")
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
#[derive(Debug, Clone, PartialEq, Hash)]
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
        self.0.cast_last_match().expect("module include is missing path")
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
