use std::ops::Range;

use super::{NodeKind, RedRef};

/// Provide highlighting categories for the children of a node that fall into a
/// range.
pub fn highlight<F>(node: RedRef, range: Range<usize>, f: &mut F)
where
    F: FnMut(Range<usize>, Category),
{
    for child in node.children() {
        let span = child.span();
        if range.start <= span.end && range.end >= span.start {
            if let Some(category) = Category::determine(child, node) {
                f(span.to_range(), category);
            }
            highlight(child, range.clone(), f);
        }
    }
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
    /// A variable.
    Variable,
    /// An invalid node.
    Invalid,
}

impl Category {
    /// Determine the highlighting category of a node given its parent.
    pub fn determine(child: RedRef, parent: RedRef) -> Option<Category> {
        match child.kind() {
            NodeKind::LeftBracket => Some(Category::Bracket),
            NodeKind::RightBracket => Some(Category::Bracket),
            NodeKind::LeftBrace => Some(Category::Bracket),
            NodeKind::RightBrace => Some(Category::Bracket),
            NodeKind::LeftParen => Some(Category::Bracket),
            NodeKind::RightParen => Some(Category::Bracket),
            NodeKind::Comma => Some(Category::Punctuation),
            NodeKind::Semicolon => Some(Category::Punctuation),
            NodeKind::Colon => Some(Category::Punctuation),
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
            NodeKind::Linebreak => Some(Category::Shortcut),
            NodeKind::NonBreakingSpace => Some(Category::Shortcut),
            NodeKind::EnDash => Some(Category::Shortcut),
            NodeKind::EmDash => Some(Category::Shortcut),
            NodeKind::Escape(_) => Some(Category::Escape),
            NodeKind::Let => Some(Category::Keyword),
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
            NodeKind::Not => Some(Category::Keyword),
            NodeKind::And => Some(Category::Keyword),
            NodeKind::Or => Some(Category::Keyword),
            NodeKind::With => Some(Category::Keyword),
            NodeKind::Plus => Some(Category::Operator),
            NodeKind::Star => Some(Category::Operator),
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
                NodeKind::Named => None,
                NodeKind::Closure if child.span().start == parent.span().start => {
                    Some(Category::Function)
                }
                NodeKind::WithExpr => Some(Category::Function),
                NodeKind::Call => Some(Category::Function),
                _ => Some(Category::Variable),
            },
            NodeKind::Bool(_) => Some(Category::Bool),
            NodeKind::Int(_) => Some(Category::Number),
            NodeKind::Float(_) => Some(Category::Number),
            NodeKind::Length(_, _) => Some(Category::Number),
            NodeKind::Angle(_, _) => Some(Category::Number),
            NodeKind::Percentage(_) => Some(Category::Number),
            NodeKind::Fraction(_) => Some(Category::Number),
            NodeKind::Str(_) => Some(Category::String),
            NodeKind::Error(_, _) => Some(Category::Invalid),
            NodeKind::Unknown(_) => Some(Category::Invalid),
            NodeKind::Markup => None,
            NodeKind::Space(_) => None,
            NodeKind::Parbreak => None,
            NodeKind::Text(_) => None,
            NodeKind::List => None,
            NodeKind::Enum => None,
            NodeKind::Array => None,
            NodeKind::Dict => None,
            NodeKind::Named => None,
            NodeKind::Group => None,
            NodeKind::Unary => None,
            NodeKind::Binary => None,
            NodeKind::Call => None,
            NodeKind::CallArgs => None,
            NodeKind::Closure => None,
            NodeKind::ClosureParams => None,
            NodeKind::Spread => None,
            NodeKind::Template => None,
            NodeKind::Block => None,
            NodeKind::ForExpr => None,
            NodeKind::WhileExpr => None,
            NodeKind::IfExpr => None,
            NodeKind::LetExpr => None,
            NodeKind::WithExpr => None,
            NodeKind::ForPattern => None,
            NodeKind::ImportExpr => None,
            NodeKind::ImportItems => None,
            NodeKind::IncludeExpr => None,
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
            source.highlight(0 .. src.len(), |range, category| {
                vec.push((range, category));
            });
            assert_eq!(vec, goal);
        }

        test("= *AB*", &[
            (0 .. 6, Heading),
            (2 .. 3, Strong),
            (5 .. 6, Strong),
        ]);

        test("#f(x + 1)", &[
            (0 .. 2, Function),
            (2 .. 3, Bracket),
            (3 .. 4, Variable),
            (5 .. 6, Operator),
            (7 .. 8, Number),
            (8 .. 9, Bracket),
        ]);

        test("#let f(x) = x", &[
            (0 .. 4, Keyword),
            (5 .. 6, Function),
            (6 .. 7, Bracket),
            (7 .. 8, Variable),
            (8 .. 9, Bracket),
            (10 .. 11, Operator),
            (12 .. 13, Variable),
        ]);
    }
}
