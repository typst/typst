use crate::{ast, LinkedNode, SyntaxKind, SyntaxNode};

/// A syntax highlighting tag.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Tag {
    /// A line or block comment.
    Comment,
    /// Punctuation in code.
    Punctuation,
    /// An escape sequence or shorthand.
    Escape,
    /// Strong markup.
    Strong,
    /// Emphasized markup.
    Emph,
    /// A hyperlink.
    Link,
    /// Raw text.
    Raw,
    /// A label.
    Label,
    /// A reference to a label.
    Ref,
    /// A section heading.
    Heading,
    /// A marker of a list, enumeration, or term list.
    ListMarker,
    /// A term in a term list.
    ListTerm,
    /// The delimiters of an equation.
    MathDelimiter,
    /// An operator with special meaning in an equation.
    MathOperator,
    /// A keyword.
    Keyword,
    /// An operator in code.
    Operator,
    /// A numeric literal.
    Number,
    /// A string literal.
    String,
    /// A function or method name.
    Function,
    /// An interpolated variable in markup or math.
    Interpolated,
    /// A syntax error.
    Error,
}

impl Tag {
    /// The list of all tags, in the same order as thy are defined.
    ///
    /// Can be used as the counter-part to `tag as usize`.
    pub const LIST: &'static [Tag] = &[
        Self::Comment,
        Self::Punctuation,
        Self::Escape,
        Self::Strong,
        Self::Emph,
        Self::Link,
        Self::Raw,
        Self::Label,
        Self::Ref,
        Self::Heading,
        Self::ListMarker,
        Self::ListTerm,
        Self::MathDelimiter,
        Self::MathOperator,
        Self::Keyword,
        Self::Operator,
        Self::Number,
        Self::String,
        Self::Function,
        Self::Interpolated,
        Self::Error,
    ];

    /// Return the recommended TextMate grammar scope for the given highlighting
    /// tag.
    pub fn tm_scope(&self) -> &'static str {
        match self {
            Self::Comment => "comment.typst",
            Self::Punctuation => "punctuation.typst",
            Self::Escape => "constant.character.escape.typst",
            Self::Strong => "markup.bold.typst",
            Self::Emph => "markup.italic.typst",
            Self::Link => "markup.underline.link.typst",
            Self::Raw => "markup.raw.typst",
            Self::MathDelimiter => "punctuation.definition.math.typst",
            Self::MathOperator => "keyword.operator.math.typst",
            Self::Heading => "markup.heading.typst",
            Self::ListMarker => "punctuation.definition.list.typst",
            Self::ListTerm => "markup.list.term.typst",
            Self::Label => "entity.name.label.typst",
            Self::Ref => "markup.other.reference.typst",
            Self::Keyword => "keyword.typst",
            Self::Operator => "keyword.operator.typst",
            Self::Number => "constant.numeric.typst",
            Self::String => "string.quoted.double.typst",
            Self::Function => "entity.name.function.typst",
            Self::Interpolated => "meta.interpolation.typst",
            Self::Error => "invalid.typst",
        }
    }

    /// The recommended CSS class for the highlighting tag.
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Comment => "typ-comment",
            Self::Punctuation => "typ-punct",
            Self::Escape => "typ-escape",
            Self::Strong => "typ-strong",
            Self::Emph => "typ-emph",
            Self::Link => "typ-link",
            Self::Raw => "typ-raw",
            Self::Label => "typ-label",
            Self::Ref => "typ-ref",
            Self::Heading => "typ-heading",
            Self::ListMarker => "typ-marker",
            Self::ListTerm => "typ-term",
            Self::MathDelimiter => "typ-math-delim",
            Self::MathOperator => "typ-math-op",
            Self::Keyword => "typ-key",
            Self::Operator => "typ-op",
            Self::Number => "typ-num",
            Self::String => "typ-str",
            Self::Function => "typ-func",
            Self::Interpolated => "typ-pol",
            Self::Error => "typ-error",
        }
    }
}

/// Determine the highlight tag of a linked syntax node.
///
/// Returns `None` if the node should not be highlighted.
pub fn highlight(node: &LinkedNode) -> Option<Tag> {
    match node.kind() {
        SyntaxKind::Markup
            if node.parent_kind() == Some(SyntaxKind::TermItem)
                && node.next_sibling_kind() == Some(SyntaxKind::Colon) =>
        {
            Some(Tag::ListTerm)
        }
        SyntaxKind::Markup => None,
        SyntaxKind::Text => None,
        SyntaxKind::Space => None,
        SyntaxKind::Linebreak => Some(Tag::Escape),
        SyntaxKind::Parbreak => None,
        SyntaxKind::Escape => Some(Tag::Escape),
        SyntaxKind::Shorthand => Some(Tag::Escape),
        SyntaxKind::SmartQuote => None,
        SyntaxKind::Strong => Some(Tag::Strong),
        SyntaxKind::Emph => Some(Tag::Emph),
        SyntaxKind::Raw => Some(Tag::Raw),
        SyntaxKind::RawLang => None,
        SyntaxKind::RawTrimmed => None,
        SyntaxKind::RawDelim => None,
        SyntaxKind::Link => Some(Tag::Link),
        SyntaxKind::Label => Some(Tag::Label),
        SyntaxKind::Ref => Some(Tag::Ref),
        SyntaxKind::RefMarker => None,
        SyntaxKind::Heading => Some(Tag::Heading),
        SyntaxKind::HeadingMarker => None,
        SyntaxKind::ListItem => None,
        SyntaxKind::ListMarker => Some(Tag::ListMarker),
        SyntaxKind::EnumItem => None,
        SyntaxKind::EnumMarker => Some(Tag::ListMarker),
        SyntaxKind::TermItem => None,
        SyntaxKind::TermMarker => Some(Tag::ListMarker),
        SyntaxKind::Equation => None,

        SyntaxKind::Math => None,
        SyntaxKind::MathIdent => highlight_ident(node),
        SyntaxKind::MathShorthand => Some(Tag::Escape),
        SyntaxKind::MathAlignPoint => Some(Tag::MathOperator),
        SyntaxKind::MathDelimited => None,
        SyntaxKind::MathAttach => None,
        SyntaxKind::MathFrac => None,
        SyntaxKind::MathRoot => None,
        SyntaxKind::MathPrimes => None,

        SyntaxKind::Hash => highlight_hash(node),
        SyntaxKind::LeftBrace => Some(Tag::Punctuation),
        SyntaxKind::RightBrace => Some(Tag::Punctuation),
        SyntaxKind::LeftBracket => Some(Tag::Punctuation),
        SyntaxKind::RightBracket => Some(Tag::Punctuation),
        SyntaxKind::LeftParen => Some(Tag::Punctuation),
        SyntaxKind::RightParen => Some(Tag::Punctuation),
        SyntaxKind::Comma => Some(Tag::Punctuation),
        SyntaxKind::Semicolon => Some(Tag::Punctuation),
        SyntaxKind::Colon => Some(Tag::Punctuation),
        SyntaxKind::Star => match node.parent_kind() {
            Some(SyntaxKind::Strong) => None,
            _ => Some(Tag::Operator),
        },
        SyntaxKind::Underscore => match node.parent_kind() {
            Some(SyntaxKind::MathAttach) => Some(Tag::MathOperator),
            _ => None,
        },
        SyntaxKind::Dollar => Some(Tag::MathDelimiter),
        SyntaxKind::Plus => Some(Tag::Operator),
        SyntaxKind::Minus => Some(Tag::Operator),
        SyntaxKind::Slash => Some(match node.parent_kind() {
            Some(SyntaxKind::MathFrac) => Tag::MathOperator,
            _ => Tag::Operator,
        }),
        SyntaxKind::Hat => Some(Tag::MathOperator),
        SyntaxKind::Prime => Some(Tag::MathOperator),
        SyntaxKind::Dot => Some(Tag::Punctuation),
        SyntaxKind::Eq => match node.parent_kind() {
            Some(SyntaxKind::Heading) => None,
            _ => Some(Tag::Operator),
        },
        SyntaxKind::EqEq => Some(Tag::Operator),
        SyntaxKind::ExclEq => Some(Tag::Operator),
        SyntaxKind::Lt => Some(Tag::Operator),
        SyntaxKind::LtEq => Some(Tag::Operator),
        SyntaxKind::Gt => Some(Tag::Operator),
        SyntaxKind::GtEq => Some(Tag::Operator),
        SyntaxKind::PlusEq => Some(Tag::Operator),
        SyntaxKind::HyphEq => Some(Tag::Operator),
        SyntaxKind::StarEq => Some(Tag::Operator),
        SyntaxKind::SlashEq => Some(Tag::Operator),
        SyntaxKind::Dots => Some(Tag::Operator),
        SyntaxKind::Arrow => Some(Tag::Operator),
        SyntaxKind::Root => Some(Tag::MathOperator),

        SyntaxKind::Not => Some(Tag::Keyword),
        SyntaxKind::And => Some(Tag::Keyword),
        SyntaxKind::Or => Some(Tag::Keyword),
        SyntaxKind::None => Some(Tag::Keyword),
        SyntaxKind::Auto => Some(Tag::Keyword),
        SyntaxKind::Let => Some(Tag::Keyword),
        SyntaxKind::Set => Some(Tag::Keyword),
        SyntaxKind::Show => Some(Tag::Keyword),
        SyntaxKind::Context => Some(Tag::Keyword),
        SyntaxKind::If => Some(Tag::Keyword),
        SyntaxKind::Else => Some(Tag::Keyword),
        SyntaxKind::For => Some(Tag::Keyword),
        SyntaxKind::In => Some(Tag::Keyword),
        SyntaxKind::While => Some(Tag::Keyword),
        SyntaxKind::Break => Some(Tag::Keyword),
        SyntaxKind::Continue => Some(Tag::Keyword),
        SyntaxKind::Return => Some(Tag::Keyword),
        SyntaxKind::Import => Some(Tag::Keyword),
        SyntaxKind::Include => Some(Tag::Keyword),
        SyntaxKind::As => Some(Tag::Keyword),

        SyntaxKind::Code => None,
        SyntaxKind::Ident => highlight_ident(node),
        SyntaxKind::Bool => Some(Tag::Keyword),
        SyntaxKind::Int => Some(Tag::Number),
        SyntaxKind::Float => Some(Tag::Number),
        SyntaxKind::Numeric => Some(Tag::Number),
        SyntaxKind::Str => Some(Tag::String),
        SyntaxKind::CodeBlock => None,
        SyntaxKind::ContentBlock => None,
        SyntaxKind::Parenthesized => None,
        SyntaxKind::Array => None,
        SyntaxKind::Dict => None,
        SyntaxKind::Named => None,
        SyntaxKind::Keyed => None,
        SyntaxKind::Unary => None,
        SyntaxKind::Binary => None,
        SyntaxKind::FieldAccess => None,
        SyntaxKind::FuncCall => None,
        SyntaxKind::Args => None,
        SyntaxKind::Spread => None,
        SyntaxKind::Closure => None,
        SyntaxKind::Params => None,
        SyntaxKind::LetBinding => None,
        SyntaxKind::SetRule => None,
        SyntaxKind::ShowRule => None,
        SyntaxKind::Contextual => None,
        SyntaxKind::Conditional => None,
        SyntaxKind::WhileLoop => None,
        SyntaxKind::ForLoop => None,
        SyntaxKind::ModuleImport => None,
        SyntaxKind::ImportItems => None,
        SyntaxKind::ImportItemPath => None,
        SyntaxKind::RenamedImportItem => None,
        SyntaxKind::ModuleInclude => None,
        SyntaxKind::LoopBreak => None,
        SyntaxKind::LoopContinue => None,
        SyntaxKind::FuncReturn => None,
        SyntaxKind::Destructuring => None,
        SyntaxKind::DestructAssignment => None,

        SyntaxKind::LineComment => Some(Tag::Comment),
        SyntaxKind::BlockComment => Some(Tag::Comment),
        SyntaxKind::Error => Some(Tag::Error),
        SyntaxKind::End => None,
    }
}

/// Highlight an identifier based on context.
fn highlight_ident(node: &LinkedNode) -> Option<Tag> {
    // Are we directly before an argument list?
    let next_leaf = node.next_leaf();
    if let Some(next) = &next_leaf {
        if node.range().end == next.offset()
            && ((next.kind() == SyntaxKind::LeftParen
                && matches!(
                    next.parent_kind(),
                    Some(SyntaxKind::Args | SyntaxKind::Params)
                ))
                || (next.kind() == SyntaxKind::LeftBracket
                    && next.parent_kind() == Some(SyntaxKind::ContentBlock)))
        {
            return Some(Tag::Function);
        }
    }

    // Are we in math?
    if node.kind() == SyntaxKind::MathIdent {
        return Some(Tag::Interpolated);
    }

    // Find the first non-field access ancestor.
    let mut ancestor = node;
    while ancestor.parent_kind() == Some(SyntaxKind::FieldAccess) {
        ancestor = ancestor.parent()?;
    }

    // Are we directly before or behind a show rule colon?
    if ancestor.parent_kind() == Some(SyntaxKind::ShowRule)
        && (next_leaf.map(|leaf| leaf.kind()) == Some(SyntaxKind::Colon)
            || node.prev_leaf().map(|leaf| leaf.kind()) == Some(SyntaxKind::Colon))
    {
        return Some(Tag::Function);
    }

    // Are we (or an ancestor field access) directly after a hash.
    if ancestor.prev_leaf().map(|leaf| leaf.kind()) == Some(SyntaxKind::Hash) {
        return Some(Tag::Interpolated);
    }

    // Are we behind a dot, that is behind another identifier?
    let prev = node.prev_leaf()?;
    if prev.kind() == SyntaxKind::Dot {
        let prev_prev = prev.prev_leaf()?;
        if is_ident(&prev_prev) {
            return highlight_ident(&prev_prev);
        }
    }

    None
}

/// Highlight a hash based on context.
fn highlight_hash(node: &LinkedNode) -> Option<Tag> {
    let next = node.next_sibling()?;
    let expr = next.cast::<ast::Expr>()?;
    if !expr.hash() {
        return None;
    }
    highlight(&next.leftmost_leaf()?)
}

/// Whether the node is one of the two identifier nodes.
fn is_ident(node: &LinkedNode) -> bool {
    matches!(node.kind(), SyntaxKind::Ident | SyntaxKind::MathIdent)
}

/// Highlight a node to an HTML `code` element.
///
/// This uses these [CSS classes for categories](Tag::css_class).
pub fn highlight_html(root: &SyntaxNode) -> String {
    let mut buf = String::from("<code>");
    let node = LinkedNode::new(root);
    highlight_html_impl(&mut buf, &node);
    buf.push_str("</code>");
    buf
}

/// Highlight one source node, emitting HTML.
fn highlight_html_impl(html: &mut String, node: &LinkedNode) {
    let mut span = false;
    if let Some(tag) = highlight(node) {
        if tag != Tag::Error {
            span = true;
            html.push_str("<span class=\"");
            html.push_str(tag.css_class());
            html.push_str("\">");
        }
    }

    let text = node.text();
    if !text.is_empty() {
        for c in text.chars() {
            match c {
                '<' => html.push_str("&lt;"),
                '>' => html.push_str("&gt;"),
                '&' => html.push_str("&amp;"),
                '\'' => html.push_str("&#39;"),
                '"' => html.push_str("&quot;"),
                _ => html.push(c),
            }
        }
    } else {
        for child in node.children() {
            highlight_html_impl(html, &child);
        }
    }

    if span {
        html.push_str("</span>");
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::*;

    #[test]
    fn test_highlighting() {
        use Tag::*;

        #[track_caller]
        fn test(text: &str, goal: &[(Range<usize>, Tag)]) {
            let mut vec = vec![];
            let root = crate::parse(text);
            highlight_tree(&mut vec, &LinkedNode::new(&root));
            assert_eq!(vec, goal);
        }

        fn highlight_tree(tags: &mut Vec<(Range<usize>, Tag)>, node: &LinkedNode) {
            if let Some(tag) = highlight(node) {
                tags.push((node.range(), tag));
            }

            for child in node.children() {
                highlight_tree(tags, &child);
            }
        }

        test("= *AB*", &[(0..6, Heading), (2..6, Strong)]);

        test(
            "#f(x + 1)",
            &[
                (0..1, Function),
                (1..2, Function),
                (2..3, Punctuation),
                (5..6, Operator),
                (7..8, Number),
                (8..9, Punctuation),
            ],
        );

        test(
            "#let f(x) = x",
            &[
                (0..1, Keyword),
                (1..4, Keyword),
                (5..6, Function),
                (6..7, Punctuation),
                (8..9, Punctuation),
                (10..11, Operator),
            ],
        );
    }
}
