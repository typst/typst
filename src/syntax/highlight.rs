//! Syntax highlighting for Typst source code.

use std::fmt::Write;
use std::ops::Range;

use syntect::highlighting::{Color, FontStyle, Highlighter, Style, Theme};
use syntect::parsing::Scope;

use super::{parse, SyntaxKind, SyntaxNode};

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
    /// Raw text.
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
            SyntaxKind::LineComment => Some(Category::Comment),
            SyntaxKind::BlockComment => Some(Category::Comment),
            SyntaxKind::Space { .. } => None,

            SyntaxKind::LeftBrace => Some(Category::Bracket),
            SyntaxKind::RightBrace => Some(Category::Bracket),
            SyntaxKind::LeftBracket => Some(Category::Bracket),
            SyntaxKind::RightBracket => Some(Category::Bracket),
            SyntaxKind::LeftParen => Some(Category::Bracket),
            SyntaxKind::RightParen => Some(Category::Bracket),
            SyntaxKind::Comma => Some(Category::Punctuation),
            SyntaxKind::Semicolon => Some(Category::Punctuation),
            SyntaxKind::Colon => Some(Category::Punctuation),
            SyntaxKind::Star => match parent.kind() {
                SyntaxKind::Strong => None,
                _ => Some(Category::Operator),
            },
            SyntaxKind::Underscore => match parent.kind() {
                SyntaxKind::Script => Some(Category::MathOperator),
                _ => None,
            },
            SyntaxKind::Dollar => Some(Category::MathDelimiter),
            SyntaxKind::Plus => Some(match parent.kind() {
                SyntaxKind::EnumItem => Category::ListMarker,
                _ => Category::Operator,
            }),
            SyntaxKind::Minus => Some(match parent.kind() {
                SyntaxKind::ListItem => Category::ListMarker,
                _ => Category::Operator,
            }),
            SyntaxKind::Slash => Some(match parent.kind() {
                SyntaxKind::DescItem => Category::ListMarker,
                SyntaxKind::Frac => Category::MathOperator,
                _ => Category::Operator,
            }),
            SyntaxKind::Hat => Some(Category::MathOperator),
            SyntaxKind::Amp => Some(Category::MathOperator),
            SyntaxKind::Dot => Some(Category::Punctuation),
            SyntaxKind::Eq => match parent.kind() {
                SyntaxKind::Heading => None,
                _ => Some(Category::Operator),
            },
            SyntaxKind::EqEq => Some(Category::Operator),
            SyntaxKind::ExclEq => Some(Category::Operator),
            SyntaxKind::Lt => Some(Category::Operator),
            SyntaxKind::LtEq => Some(Category::Operator),
            SyntaxKind::Gt => Some(Category::Operator),
            SyntaxKind::GtEq => Some(Category::Operator),
            SyntaxKind::PlusEq => Some(Category::Operator),
            SyntaxKind::HyphEq => Some(Category::Operator),
            SyntaxKind::StarEq => Some(Category::Operator),
            SyntaxKind::SlashEq => Some(Category::Operator),
            SyntaxKind::Dots => Some(Category::Operator),
            SyntaxKind::Arrow => Some(Category::Operator),

            SyntaxKind::Not => Some(Category::Keyword),
            SyntaxKind::And => Some(Category::Keyword),
            SyntaxKind::Or => Some(Category::Keyword),
            SyntaxKind::None => Some(Category::KeywordLiteral),
            SyntaxKind::Auto => Some(Category::KeywordLiteral),
            SyntaxKind::Let => Some(Category::Keyword),
            SyntaxKind::Set => Some(Category::Keyword),
            SyntaxKind::Show => Some(Category::Keyword),
            SyntaxKind::If => Some(Category::Keyword),
            SyntaxKind::Else => Some(Category::Keyword),
            SyntaxKind::For => Some(Category::Keyword),
            SyntaxKind::In => Some(Category::Keyword),
            SyntaxKind::While => Some(Category::Keyword),
            SyntaxKind::Break => Some(Category::Keyword),
            SyntaxKind::Continue => Some(Category::Keyword),
            SyntaxKind::Return => Some(Category::Keyword),
            SyntaxKind::Import => Some(Category::Keyword),
            SyntaxKind::Include => Some(Category::Keyword),
            SyntaxKind::From => Some(Category::Keyword),

            SyntaxKind::Markup { .. } => match parent.kind() {
                SyntaxKind::DescItem
                    if parent
                        .children()
                        .take_while(|child| child.kind() != &SyntaxKind::Colon)
                        .find(|c| matches!(c.kind(), SyntaxKind::Markup { .. }))
                        .map_or(false, |ident| std::ptr::eq(ident, child)) =>
                {
                    Some(Category::ListTerm)
                }
                _ => None,
            },
            SyntaxKind::Text(_) => None,
            SyntaxKind::Linebreak => Some(Category::Escape),
            SyntaxKind::Escape(_) => Some(Category::Escape),
            SyntaxKind::Shorthand(_) => Some(Category::Shorthand),
            SyntaxKind::SmartQuote { .. } => Some(Category::SmartQuote),
            SyntaxKind::Strong => Some(Category::Strong),
            SyntaxKind::Emph => Some(Category::Emph),
            SyntaxKind::Raw(_) => Some(Category::Raw),
            SyntaxKind::Link(_) => Some(Category::Link),
            SyntaxKind::Label(_) => Some(Category::Label),
            SyntaxKind::Ref(_) => Some(Category::Ref),
            SyntaxKind::Heading => Some(Category::Heading),
            SyntaxKind::ListItem => Some(Category::ListItem),
            SyntaxKind::EnumItem => Some(Category::ListItem),
            SyntaxKind::EnumNumbering(_) => Some(Category::ListMarker),
            SyntaxKind::DescItem => Some(Category::ListItem),
            SyntaxKind::Math => Some(Category::Math),
            SyntaxKind::Atom(_) => None,
            SyntaxKind::Script => None,
            SyntaxKind::Frac => None,
            SyntaxKind::Align => None,

            SyntaxKind::Ident(_) => match parent.kind() {
                SyntaxKind::Markup { .. } => Some(Category::Interpolated),
                SyntaxKind::Math => Some(Category::Interpolated),
                SyntaxKind::FuncCall => Some(Category::Function),
                SyntaxKind::MethodCall if i > 0 => Some(Category::Function),
                SyntaxKind::Closure if i == 0 => Some(Category::Function),
                SyntaxKind::SetRule => Some(Category::Function),
                SyntaxKind::ShowRule
                    if parent
                        .children()
                        .rev()
                        .skip_while(|child| child.kind() != &SyntaxKind::Colon)
                        .find(|c| matches!(c.kind(), SyntaxKind::Ident(_)))
                        .map_or(false, |ident| std::ptr::eq(ident, child)) =>
                {
                    Some(Category::Function)
                }
                _ => None,
            },
            SyntaxKind::Bool(_) => Some(Category::KeywordLiteral),
            SyntaxKind::Int(_) => Some(Category::Number),
            SyntaxKind::Float(_) => Some(Category::Number),
            SyntaxKind::Numeric(_, _) => Some(Category::Number),
            SyntaxKind::Str(_) => Some(Category::String),
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
            SyntaxKind::MethodCall => None,
            SyntaxKind::Args => None,
            SyntaxKind::Spread => None,
            SyntaxKind::Closure => None,
            SyntaxKind::Params => None,
            SyntaxKind::LetBinding => None,
            SyntaxKind::SetRule => None,
            SyntaxKind::ShowRule => None,
            SyntaxKind::Conditional => None,
            SyntaxKind::WhileLoop => None,
            SyntaxKind::ForLoop => None,
            SyntaxKind::ForPattern => None,
            SyntaxKind::ModuleImport => None,
            SyntaxKind::ImportItems => None,
            SyntaxKind::ModuleInclude => None,
            SyntaxKind::LoopBreak => None,
            SyntaxKind::LoopContinue => None,
            SyntaxKind::FuncReturn => None,

            SyntaxKind::Error(_, _) => Some(Category::Error),
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
