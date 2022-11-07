//! Syntax highlighting for Typst source code.

use std::fmt::Write;
use std::ops::Range;

use syntect::highlighting::{Color, FontStyle, Highlighter, Style, Theme};
use syntect::parsing::Scope;

use super::{parse, NodeKind, SyntaxNode};

/// Highlight source text into a standalone HTML document.
pub fn highlight_html(text: &str, theme: &Theme) -> String {
    let mut buf = String::new();
    buf.push_str("<!DOCTYPE html>\n");
    buf.push_str("<html>\n");
    buf.push_str("<head>\n");
    buf.push_str("  <meta charset=\"utf-8\">\n");
    buf.push_str("</head>\n");
    buf.push_str("<body>\n");
    buf.push_str(&highlight_pre(text, theme));
    buf.push_str("\n</body>\n");
    buf.push_str("</html>\n");
    buf
}

/// Highlight source text into an HTML pre element.
pub fn highlight_pre(text: &str, theme: &Theme) -> String {
    let mut buf = String::new();
    buf.push_str("<pre>\n");

    let root = parse(text);
    highlight_themed(&root, theme, |range, style| {
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

        buf.push_str(&text[range]);

        if styled {
            buf.push_str("</span>");
        }
    });

    buf.push_str("\n</pre>");
    buf
}

/// Highlight a syntax node in a theme by calling `f` with ranges and their
/// styles.
pub fn highlight_themed<F>(root: &SyntaxNode, theme: &Theme, mut f: F)
where
    F: FnMut(Range<usize>, Style),
{
    fn process<F>(
        mut offset: usize,
        node: &SyntaxNode,
        scopes: Vec<Scope>,
        highlighter: &Highlighter,
        f: &mut F,
    ) where
        F: FnMut(Range<usize>, Style),
    {
        if node.children().len() == 0 {
            let range = offset..offset + node.len();
            let style = highlighter.style_for_stack(&scopes);
            f(range, style);
            return;
        }

        for (i, child) in node.children().enumerate() {
            let mut scopes = scopes.clone();
            if let Some(category) = Category::determine(child, node, i) {
                scopes.push(Scope::new(category.tm_scope()).unwrap())
            }
            process(offset, child, scopes, highlighter, f);
            offset += child.len();
        }
    }

    let highlighter = Highlighter::new(theme);
    process(0, root, vec![], &highlighter, &mut f);
}

/// Highlight a syntax node by calling `f` with ranges overlapping `within` and
/// their categories.
pub fn highlight_categories<F>(root: &SyntaxNode, within: Range<usize>, mut f: F)
where
    F: FnMut(Range<usize>, Category),
{
    fn process<F>(mut offset: usize, node: &SyntaxNode, range: Range<usize>, f: &mut F)
    where
        F: FnMut(Range<usize>, Category),
    {
        for (i, child) in node.children().enumerate() {
            let span = offset..offset + child.len();
            if range.start <= span.end && range.end >= span.start {
                if let Some(category) = Category::determine(child, node, i) {
                    f(span, category);
                }
                process(offset, child, range.clone(), f);
            }
            offset += child.len();
        }
    }

    process(0, root, within, &mut f)
}

/// The syntax highlighting category of a node.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Category {
    /// A line or block comment.
    Comment,
    /// A square bracket, parenthesis or brace.
    Bracket,
    /// Punctuation in code.
    Punctuation,
    /// An escape sequence.
    Escape,
    /// An easily typable shortcut to a unicode codepoint.
    Shorthand,
    /// A smart quote.
    SmartQuote,
    /// Strong markup.
    Strong,
    /// Emphasized markup.
    Emph,
    /// A hyperlink.
    Link,
    /// Raw text or code.
    Raw,
    /// A label.
    Label,
    /// A reference.
    Ref,
    /// A section heading.
    Heading,
    /// A full item of a list, enumeration or description list.
    ListItem,
    /// A marker of a list, enumeration, or description list.
    ListMarker,
    /// A term in a description list.
    ListTerm,
    /// A full math formula.
    Math,
    /// The delimiters of a math formula.
    MathDelimiter,
    /// An operator with special meaning in a math formula.
    MathOperator,
    /// A keyword.
    Keyword,
    /// A literal defined by a keyword like `none`, `auto` or a boolean.
    KeywordLiteral,
    /// An operator symbol.
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

impl Category {
    /// Determine the highlighting category of a node given its parent and its
    /// index in its siblings.
    pub fn determine(
        child: &SyntaxNode,
        parent: &SyntaxNode,
        i: usize,
    ) -> Option<Category> {
        match child.kind() {
            NodeKind::LineComment => Some(Category::Comment),
            NodeKind::BlockComment => Some(Category::Comment),
            NodeKind::Space { .. } => None,

            NodeKind::LeftBrace => Some(Category::Bracket),
            NodeKind::RightBrace => Some(Category::Bracket),
            NodeKind::LeftBracket => Some(Category::Bracket),
            NodeKind::RightBracket => Some(Category::Bracket),
            NodeKind::LeftParen => Some(Category::Bracket),
            NodeKind::RightParen => Some(Category::Bracket),
            NodeKind::Comma => Some(Category::Punctuation),
            NodeKind::Semicolon => Some(Category::Punctuation),
            NodeKind::Colon => Some(Category::Punctuation),
            NodeKind::Star => match parent.kind() {
                NodeKind::Strong => None,
                _ => Some(Category::Operator),
            },
            NodeKind::Underscore => match parent.kind() {
                NodeKind::Script => Some(Category::MathOperator),
                _ => None,
            },
            NodeKind::Dollar => Some(Category::MathDelimiter),
            NodeKind::Plus => Some(match parent.kind() {
                NodeKind::EnumItem => Category::ListMarker,
                _ => Category::Operator,
            }),
            NodeKind::Minus => Some(match parent.kind() {
                NodeKind::ListItem => Category::ListMarker,
                _ => Category::Operator,
            }),
            NodeKind::Slash => Some(match parent.kind() {
                NodeKind::DescItem => Category::ListMarker,
                NodeKind::Frac => Category::MathOperator,
                _ => Category::Operator,
            }),
            NodeKind::Hat => Some(Category::MathOperator),
            NodeKind::Amp => Some(Category::MathOperator),
            NodeKind::Dot => Some(Category::Punctuation),
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
            NodeKind::PlusEq => Some(Category::Operator),
            NodeKind::HyphEq => Some(Category::Operator),
            NodeKind::StarEq => Some(Category::Operator),
            NodeKind::SlashEq => Some(Category::Operator),
            NodeKind::Dots => Some(Category::Operator),
            NodeKind::Arrow => Some(Category::Operator),

            NodeKind::Not => Some(Category::Keyword),
            NodeKind::And => Some(Category::Keyword),
            NodeKind::Or => Some(Category::Keyword),
            NodeKind::None => Some(Category::KeywordLiteral),
            NodeKind::Auto => Some(Category::KeywordLiteral),
            NodeKind::Let => Some(Category::Keyword),
            NodeKind::Set => Some(Category::Keyword),
            NodeKind::Show => Some(Category::Keyword),
            NodeKind::If => Some(Category::Keyword),
            NodeKind::Else => Some(Category::Keyword),
            NodeKind::For => Some(Category::Keyword),
            NodeKind::In => Some(Category::Keyword),
            NodeKind::While => Some(Category::Keyword),
            NodeKind::Break => Some(Category::Keyword),
            NodeKind::Continue => Some(Category::Keyword),
            NodeKind::Return => Some(Category::Keyword),
            NodeKind::Import => Some(Category::Keyword),
            NodeKind::Include => Some(Category::Keyword),
            NodeKind::From => Some(Category::Keyword),

            NodeKind::Markup { .. } => match parent.kind() {
                NodeKind::DescItem
                    if parent
                        .children()
                        .take_while(|child| child.kind() != &NodeKind::Colon)
                        .find(|c| matches!(c.kind(), NodeKind::Markup { .. }))
                        .map_or(false, |ident| std::ptr::eq(ident, child)) =>
                {
                    Some(Category::ListTerm)
                }
                _ => None,
            },
            NodeKind::Text(_) => None,
            NodeKind::Linebreak => Some(Category::Escape),
            NodeKind::Escape(_) => Some(Category::Escape),
            NodeKind::Shorthand(_) => Some(Category::Shorthand),
            NodeKind::SmartQuote { .. } => Some(Category::SmartQuote),
            NodeKind::Strong => Some(Category::Strong),
            NodeKind::Emph => Some(Category::Emph),
            NodeKind::Raw(_) => Some(Category::Raw),
            NodeKind::Link(_) => Some(Category::Link),
            NodeKind::Label(_) => Some(Category::Label),
            NodeKind::Ref(_) => Some(Category::Ref),
            NodeKind::Heading => Some(Category::Heading),
            NodeKind::ListItem => Some(Category::ListItem),
            NodeKind::EnumItem => Some(Category::ListItem),
            NodeKind::EnumNumbering(_) => Some(Category::ListMarker),
            NodeKind::DescItem => Some(Category::ListItem),
            NodeKind::Math => Some(Category::Math),
            NodeKind::Atom(_) => None,
            NodeKind::Script => None,
            NodeKind::Frac => None,
            NodeKind::Align => None,

            NodeKind::Ident(_) => match parent.kind() {
                NodeKind::Markup { .. } => Some(Category::Interpolated),
                NodeKind::Math => Some(Category::Interpolated),
                NodeKind::FuncCall => Some(Category::Function),
                NodeKind::MethodCall if i > 0 => Some(Category::Function),
                NodeKind::Closure if i == 0 => Some(Category::Function),
                NodeKind::SetRule => Some(Category::Function),
                NodeKind::ShowRule
                    if parent
                        .children()
                        .rev()
                        .skip_while(|child| child.kind() != &NodeKind::Colon)
                        .find(|c| matches!(c.kind(), NodeKind::Ident(_)))
                        .map_or(false, |ident| std::ptr::eq(ident, child)) =>
                {
                    Some(Category::Function)
                }
                _ => None,
            },
            NodeKind::Bool(_) => Some(Category::KeywordLiteral),
            NodeKind::Int(_) => Some(Category::Number),
            NodeKind::Float(_) => Some(Category::Number),
            NodeKind::Numeric(_, _) => Some(Category::Number),
            NodeKind::Str(_) => Some(Category::String),
            NodeKind::CodeBlock => None,
            NodeKind::ContentBlock => None,
            NodeKind::Parenthesized => None,
            NodeKind::Array => None,
            NodeKind::Dict => None,
            NodeKind::Named => None,
            NodeKind::Keyed => None,
            NodeKind::Unary => None,
            NodeKind::Binary => None,
            NodeKind::FieldAccess => None,
            NodeKind::FuncCall => None,
            NodeKind::MethodCall => None,
            NodeKind::Args => None,
            NodeKind::Spread => None,
            NodeKind::Closure => None,
            NodeKind::Params => None,
            NodeKind::LetBinding => None,
            NodeKind::SetRule => None,
            NodeKind::ShowRule => None,
            NodeKind::Conditional => None,
            NodeKind::WhileLoop => None,
            NodeKind::ForLoop => None,
            NodeKind::ForPattern => None,
            NodeKind::ModuleImport => None,
            NodeKind::ImportItems => None,
            NodeKind::ModuleInclude => None,
            NodeKind::LoopBreak => None,
            NodeKind::LoopContinue => None,
            NodeKind::FuncReturn => None,

            NodeKind::Error(_, _) => Some(Category::Error),
        }
    }

    /// Return the TextMate grammar scope for the given highlighting category.
    pub fn tm_scope(&self) -> &'static str {
        match self {
            Self::Comment => "comment.typst",
            Self::Bracket => "punctuation.definition.bracket.typst",
            Self::Punctuation => "punctuation.typst",
            Self::Escape => "constant.character.escape.typst",
            Self::Shorthand => "constant.character.shorthand.typst",
            Self::SmartQuote => "constant.character.quote.typst",
            Self::Strong => "markup.bold.typst",
            Self::Emph => "markup.italic.typst",
            Self::Link => "markup.underline.link.typst",
            Self::Raw => "markup.raw.typst",
            Self::Math => "string.other.math.typst",
            Self::MathDelimiter => "punctuation.definition.math.typst",
            Self::MathOperator => "keyword.operator.math.typst",
            Self::Heading => "markup.heading.typst",
            Self::ListItem => "markup.list.typst",
            Self::ListMarker => "punctuation.definition.list.typst",
            Self::ListTerm => "markup.list.term.typst",
            Self::Label => "entity.name.label.typst",
            Self::Ref => "markup.other.reference.typst",
            Self::Keyword => "keyword.typst",
            Self::Operator => "keyword.operator.typst",
            Self::KeywordLiteral => "constant.language.typst",
            Self::Number => "constant.numeric.typst",
            Self::String => "string.quoted.double.typst",
            Self::Function => "entity.name.function.typst",
            Self::Interpolated => "meta.interpolation.typst",
            Self::Error => "invalid.typst",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Source;
    use super::*;

    #[test]
    fn test_highlighting() {
        use Category::*;

        #[track_caller]
        fn test(text: &str, goal: &[(Range<usize>, Category)]) {
            let mut vec = vec![];
            let source = Source::detached(text);
            let full = 0..text.len();
            highlight_categories(source.root(), full, &mut |range, category| {
                vec.push((range, category));
            });
            assert_eq!(vec, goal);
        }

        test("= *AB*", &[(0..6, Heading), (2..6, Strong)]);

        test(
            "#f(x + 1)",
            &[
                (0..2, Function),
                (2..3, Bracket),
                (5..6, Operator),
                (7..8, Number),
                (8..9, Bracket),
            ],
        );

        test(
            "#let f(x) = x",
            &[
                (0..4, Keyword),
                (5..6, Function),
                (6..7, Bracket),
                (8..9, Bracket),
                (10..11, Operator),
            ],
        );
    }
}
