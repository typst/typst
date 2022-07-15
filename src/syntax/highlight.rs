use std::fmt::Write;
use std::ops::Range;
use std::sync::Arc;

use syntect::highlighting::{Color, FontStyle, Highlighter, Style, Theme};
use syntect::parsing::Scope;

use super::{InnerNode, NodeKind, SyntaxNode};
use crate::parse::TokenMode;

/// Provide highlighting categories for the descendants of a node that fall into
/// a range.
pub fn highlight_node<F>(root: &SyntaxNode, range: Range<usize>, mut f: F)
where
    F: FnMut(Range<usize>, Category),
{
    highlight_node_impl(0, root, range, &mut f)
}

/// Provide highlighting categories for the descendants of a node that fall into
/// a range.
pub fn highlight_node_impl<F>(
    mut offset: usize,
    node: &SyntaxNode,
    range: Range<usize>,
    f: &mut F,
) where
    F: FnMut(Range<usize>, Category),
{
    for (i, child) in node.children().enumerate() {
        let span = offset .. offset + child.len();
        if range.start <= span.end && range.end >= span.start {
            if let Some(category) = Category::determine(child, node, i) {
                f(span, category);
            }
            highlight_node_impl(offset, child, range.clone(), f);
        }
        offset += child.len();
    }
}

/// Highlight source text in a theme by calling `f` with each consecutive piece
/// and its style.
pub fn highlight_themed<F>(text: &str, mode: TokenMode, theme: &Theme, mut f: F)
where
    F: FnMut(&str, Style),
{
    let root = match mode {
        TokenMode::Markup => crate::parse::parse(text),
        TokenMode::Code => {
            let children = crate::parse::parse_code(text);
            SyntaxNode::Inner(Arc::new(InnerNode::with_children(
                NodeKind::CodeBlock,
                children,
            )))
        }
    };

    let highlighter = Highlighter::new(&theme);
    highlight_themed_impl(text, 0, &root, vec![], &highlighter, &mut f);
}

/// Recursive implementation for highlighting with a syntect theme.
fn highlight_themed_impl<F>(
    text: &str,
    mut offset: usize,
    node: &SyntaxNode,
    scopes: Vec<Scope>,
    highlighter: &Highlighter,
    f: &mut F,
) where
    F: FnMut(&str, Style),
{
    if node.children().len() == 0 {
        let piece = &text[offset .. offset + node.len()];
        let style = highlighter.style_for_stack(&scopes);
        f(piece, style);
        return;
    }

    for (i, child) in node.children().enumerate() {
        let mut scopes = scopes.clone();
        if let Some(category) = Category::determine(child, node, i) {
            scopes.push(Scope::new(category.tm_scope()).unwrap())
        }
        highlight_themed_impl(text, offset, child, scopes, highlighter, f);
        offset += child.len();
    }
}

/// Highlight source text into a standalone HTML document.
pub fn highlight_html(text: &str, mode: TokenMode, theme: &Theme) -> String {
    let mut buf = String::new();
    buf.push_str("<!DOCTYPE html>\n");
    buf.push_str("<html>\n");
    buf.push_str("<head>\n");
    buf.push_str("  <meta charset=\"utf-8\">\n");
    buf.push_str("</head>\n");
    buf.push_str("<body>\n");
    buf.push_str(&highlight_pre(text, mode, theme));
    buf.push_str("\n</body>\n");
    buf.push_str("</html>\n");
    buf
}

/// Highlight source text into an HTML pre element.
pub fn highlight_pre(text: &str, mode: TokenMode, theme: &Theme) -> String {
    let mut buf = String::new();
    buf.push_str("<pre>\n");

    highlight_themed(text, mode, theme, |piece, style| {
        let styled = style != Style::default();
        if styled {
            buf.push_str("<span style=\"");

            if style.foreground != Color::BLACK {
                let Color { r, g, b, a } = style.foreground;
                write!(buf, "color: #{r:02x}{g:02x}{b:02x}{a:02x};").unwrap();
            }

            if style.font_style.contains(FontStyle::BOLD) {
                buf.push_str("font-weight:bold;");
            }

            if style.font_style.contains(FontStyle::ITALIC) {
                buf.push_str("font-style:italic;");
            }

            if style.font_style.contains(FontStyle::UNDERLINE) {
                buf.push_str("text-decoration:underline;")
            }

            buf.push_str("\">");
        }

        buf.push_str(piece);

        if styled {
            buf.push_str("</span>");
        }
    });

    buf.push_str("\n</pre>");
    buf
}

/// The syntax highlighting category of a node.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Category {
    /// Any kind of bracket, parenthesis or brace.
    Bracket,
    /// Punctuation in code.
    Punctuation,
    /// A line or block comment.
    Comment,
    /// Strong text.
    Strong,
    /// Emphasized text.
    Emph,
    /// Raw text or code.
    Raw,
    /// A math formula.
    Math,
    /// A section heading.
    Heading,
    /// A list or enumeration.
    List,
    /// An easily typable shortcut to a unicode codepoint.
    Shortcut,
    /// An escape sequence.
    Escape,
    /// A keyword.
    Keyword,
    /// An operator symbol.
    Operator,
    /// The none literal.
    None,
    /// The auto literal.
    Auto,
    /// A boolean literal.
    Bool,
    /// A numeric literal.
    Number,
    /// A string literal.
    String,
    /// A function.
    Function,
    /// An interpolated variable in markup.
    Interpolated,
    /// An invalid node.
    Invalid,
}

impl Category {
    /// Determine the highlighting category of a node given its parent and its
    /// index in its siblings.
    pub fn determine(
        child: &SyntaxNode,
        parent: &SyntaxNode,
        i: usize,
    ) -> Option<Category> {
        match child.kind() {
            NodeKind::LeftBrace => Some(Category::Bracket),
            NodeKind::RightBrace => Some(Category::Bracket),
            NodeKind::LeftBracket => Some(Category::Bracket),
            NodeKind::RightBracket => Some(Category::Bracket),
            NodeKind::LeftParen => Some(Category::Bracket),
            NodeKind::RightParen => Some(Category::Bracket),
            NodeKind::Comma => Some(Category::Punctuation),
            NodeKind::Semicolon => Some(Category::Punctuation),
            NodeKind::Colon => Some(Category::Punctuation),
            NodeKind::Dot => Some(Category::Punctuation),
            NodeKind::LineComment => Some(Category::Comment),
            NodeKind::BlockComment => Some(Category::Comment),
            NodeKind::Strong => Some(Category::Strong),
            NodeKind::Emph => Some(Category::Emph),
            NodeKind::Raw(_) => Some(Category::Raw),
            NodeKind::Math(_) => Some(Category::Math),
            NodeKind::Heading => Some(Category::Heading),
            NodeKind::Minus => match parent.kind() {
                NodeKind::List => Some(Category::List),
                _ => Some(Category::Operator),
            },
            NodeKind::EnumNumbering(_) => Some(Category::List),
            NodeKind::Linebreak { .. } => Some(Category::Shortcut),
            NodeKind::NonBreakingSpace => Some(Category::Shortcut),
            NodeKind::Shy => Some(Category::Shortcut),
            NodeKind::EnDash => Some(Category::Shortcut),
            NodeKind::EmDash => Some(Category::Shortcut),
            NodeKind::Ellipsis => Some(Category::Shortcut),
            NodeKind::Escape(_) => Some(Category::Escape),
            NodeKind::Not => Some(Category::Keyword),
            NodeKind::And => Some(Category::Keyword),
            NodeKind::Or => Some(Category::Keyword),
            NodeKind::Let => Some(Category::Keyword),
            NodeKind::Set => Some(Category::Keyword),
            NodeKind::Show => Some(Category::Keyword),
            NodeKind::Wrap => Some(Category::Keyword),
            NodeKind::If => Some(Category::Keyword),
            NodeKind::Else => Some(Category::Keyword),
            NodeKind::While => Some(Category::Keyword),
            NodeKind::For => Some(Category::Keyword),
            NodeKind::In => Some(Category::Keyword),
            NodeKind::As => Some(Category::Keyword),
            NodeKind::Break => Some(Category::Keyword),
            NodeKind::Continue => Some(Category::Keyword),
            NodeKind::Return => Some(Category::Keyword),
            NodeKind::Import => Some(Category::Keyword),
            NodeKind::From => Some(Category::Keyword),
            NodeKind::Include => Some(Category::Keyword),
            NodeKind::Plus => Some(Category::Operator),
            NodeKind::Star => match parent.kind() {
                NodeKind::Strong => None,
                _ => Some(Category::Operator),
            },
            NodeKind::Slash => Some(Category::Operator),
            NodeKind::PlusEq => Some(Category::Operator),
            NodeKind::HyphEq => Some(Category::Operator),
            NodeKind::StarEq => Some(Category::Operator),
            NodeKind::SlashEq => Some(Category::Operator),
            NodeKind::Eq => match parent.kind() {
                NodeKind::Heading => None,
                _ => Some(Category::Operator),
            },
            NodeKind::EqEq => Some(Category::Operator),
            NodeKind::ExclEq => Some(Category::Operator),
            NodeKind::Lt => Some(Category::Operator),
            NodeKind::LtEq => Some(Category::Operator),
            NodeKind::Gt => Some(Category::Operator),
            NodeKind::GtEq => Some(Category::Operator),
            NodeKind::Dots => Some(Category::Operator),
            NodeKind::Arrow => Some(Category::Operator),
            NodeKind::None => Some(Category::None),
            NodeKind::Auto => Some(Category::Auto),
            NodeKind::Ident(_) => match parent.kind() {
                NodeKind::Markup { .. } => Some(Category::Interpolated),
                NodeKind::FuncCall => Some(Category::Function),
                NodeKind::MethodCall if i > 0 => Some(Category::Function),
                NodeKind::ClosureExpr if i == 0 => Some(Category::Function),
                NodeKind::SetExpr => Some(Category::Function),
                NodeKind::ShowExpr
                    if parent
                        .children()
                        .rev()
                        .skip_while(|child| child.kind() != &NodeKind::As)
                        .take_while(|child| child.kind() != &NodeKind::Colon)
                        .find(|c| matches!(c.kind(), NodeKind::Ident(_)))
                        .map_or(false, |ident| std::ptr::eq(ident, child)) =>
                {
                    Some(Category::Function)
                }
                _ => None,
            },
            NodeKind::Bool(_) => Some(Category::Bool),
            NodeKind::Int(_) => Some(Category::Number),
            NodeKind::Float(_) => Some(Category::Number),
            NodeKind::Numeric(_, _) => Some(Category::Number),
            NodeKind::Str(_) => Some(Category::String),
            NodeKind::Error(_, _) => Some(Category::Invalid),
            NodeKind::Unknown(_) => Some(Category::Invalid),
            NodeKind::Underscore => None,
            NodeKind::Markup { .. } => None,
            NodeKind::Space { .. } => None,
            NodeKind::Text(_) => None,
            NodeKind::Quote { .. } => None,
            NodeKind::List => None,
            NodeKind::Enum => None,
            NodeKind::CodeBlock => None,
            NodeKind::ContentBlock => None,
            NodeKind::GroupExpr => None,
            NodeKind::ArrayExpr => None,
            NodeKind::DictExpr => None,
            NodeKind::Named => None,
            NodeKind::Keyed => None,
            NodeKind::UnaryExpr => None,
            NodeKind::BinaryExpr => None,
            NodeKind::FieldAccess => None,
            NodeKind::FuncCall => None,
            NodeKind::MethodCall => None,
            NodeKind::CallArgs => None,
            NodeKind::Spread => None,
            NodeKind::ClosureExpr => None,
            NodeKind::ClosureParams => None,
            NodeKind::LetExpr => None,
            NodeKind::SetExpr => None,
            NodeKind::ShowExpr => None,
            NodeKind::WrapExpr => None,
            NodeKind::IfExpr => None,
            NodeKind::WhileExpr => None,
            NodeKind::ForExpr => None,
            NodeKind::ForPattern => None,
            NodeKind::ImportExpr => None,
            NodeKind::ImportItems => None,
            NodeKind::IncludeExpr => None,
            NodeKind::BreakExpr => None,
            NodeKind::ContinueExpr => None,
            NodeKind::ReturnExpr => None,
        }
    }

    /// Return the TextMate grammar scope for the given highlighting category.
    pub fn tm_scope(&self) -> &'static str {
        match self {
            Self::Bracket => "punctuation.definition.typst",
            Self::Punctuation => "punctuation.typst",
            Self::Comment => "comment.typst",
            Self::Strong => "markup.bold.typst",
            Self::Emph => "markup.italic.typst",
            Self::Raw => "markup.raw.typst",
            Self::Math => "string.other.math.typst",
            Self::Heading => "markup.heading.typst",
            Self::List => "markup.list.typst",
            Self::Shortcut => "punctuation.shortcut.typst",
            Self::Escape => "constant.character.escape.content.typst",
            Self::Keyword => "keyword.typst",
            Self::Operator => "keyword.operator.typst",
            Self::None => "constant.language.none.typst",
            Self::Auto => "constant.language.auto.typst",
            Self::Bool => "constant.language.boolean.typst",
            Self::Number => "constant.numeric.typst",
            Self::String => "string.quoted.double.typst",
            Self::Function => "entity.name.function.typst",
            Self::Interpolated => "entity.other.interpolated.typst",
            Self::Invalid => "invalid.typst",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceFile;

    #[test]
    fn test_highlighting() {
        use Category::*;

        #[track_caller]
        fn test(src: &str, goal: &[(Range<usize>, Category)]) {
            let mut vec = vec![];
            let source = SourceFile::detached(src);
            let full = 0 .. src.len();
            highlight_node(source.root(), full, &mut |range, category| {
                vec.push((range, category));
            });
            assert_eq!(vec, goal);
        }

        test("= *AB*", &[(0 .. 6, Heading), (2 .. 6, Strong)]);

        test("#f(x + 1)", &[
            (0 .. 2, Function),
            (2 .. 3, Bracket),
            (5 .. 6, Operator),
            (7 .. 8, Number),
            (8 .. 9, Bracket),
        ]);

        test("#let f(x) = x", &[
            (0 .. 4, Keyword),
            (5 .. 6, Function),
            (6 .. 7, Bracket),
            (8 .. 9, Bracket),
            (10 .. 11, Operator),
        ]);
    }
}
