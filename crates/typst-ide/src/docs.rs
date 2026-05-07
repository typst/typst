use std::ops::Deref;

use ecow::EcoString;
use typst::foundations::{ParamInfo, Value};
use typst::syntax::{LinkedNode, SyntaxKind, ast};

use crate::IdeWorld;

/// Tries to find documentation for an arbitrary value.
pub fn find_value_docs(world: &dyn IdeWorld, value: &Value) -> Option<Docs> {
    if let Some(docs) = value.docs() {
        return Some(Docs::Native(docs));
    }

    // Try to find doc comment before a function definition.
    if let Value::Func(func) = value
        && let span = func.span()
        && let Some(id) = span.id()
        && let Ok(source) = world.source(id)
        && let Some(args) = source.find(span)
        && let Some(parent) = args.parent()
        && parent.kind() == SyntaxKind::Closure
        && let Some(grand) = parent.parent()
        && grand.kind() == SyntaxKind::LetBinding
        && let Some(docs) = Docs::collect_doc_comment(grand.clone())
    {
        return Some(docs);
    }

    None
}

/// Tries to determine documentation for a parameter.
pub fn find_param_docs(world: &dyn IdeWorld, param: &ParamInfo) -> Option<Docs> {
    match param {
        ParamInfo::Native(param) => Some(Docs::Native(param.docs)),
        ParamInfo::Closure(param) => {
            // Try to find doc comment before parameter.
            if let Some(id) = param.span.id()
                && let Ok(source) = world.source(id)
                && let Some(node) = source.find(param.span)
                && let Some(docs) = Docs::collect_doc_comment(node.clone())
            {
                return Some(docs);
            }
            None
        }
        ParamInfo::Plugin => None,
    }
}

/// Documentation for something.
pub enum Docs {
    Native(&'static str),
    Comment(EcoString),
}

impl Docs {
    /// Tries to collect the contents of a doc comment before the given node.
    ///
    /// Important: This is a pragmatic function that deals with doc comments that
    /// are currently in use in the ecosystem. It's solely used for best-effort IDE
    /// functionality.
    ///
    /// The presence of this function in typst/typst has *zero* implications on
    /// standardization of any kind of doc comment format at the language level!
    pub fn collect_doc_comment(node: LinkedNode) -> Option<Docs> {
        let mut lines = Vec::new();

        let mut current = node;
        while let Some(prev) = current.prev_sibling_with_trivia() {
            if let Some(comment) = prev.get().cast::<ast::LineComment>() {
                // Triple slash doc comments are pretty common in the ecosystem, so
                // we strip that extra slash.
                let text = comment.text();
                lines.push(text.strip_prefix('/').unwrap_or(text));
            } else if let Some(comment) = prev.get().cast::<ast::BlockComment>() {
                lines.push(comment.text());
            } else if !matches!(prev.kind(), SyntaxKind::Space | SyntaxKind::Hash) {
                break;
            }
            current = prev;
        }

        if lines.is_empty() {
            return None;
        }

        let mut output = EcoString::new();
        for line in lines.iter().rev() {
            // Remove up to one leading space for each line.
            output.push_str(line.strip_prefix(' ').unwrap_or(line));
            output.push('\n');
        }
        output.pop();

        Some(Self::Comment(output))
    }

    /// Extract the first sentence of plain text of a piece of documentation,
    /// removing Markdown formatting.
    ///
    /// For doc comments, it's unclear whether they contain plain text,
    /// Markdown, or Typst, but this is okay-ish for now.
    pub fn summary(&self) -> EcoString {
        let paragraph = self.split("\n\n").next().unwrap_or_default();
        let mut s = unscanny::Scanner::new(paragraph);
        let mut output = EcoString::new();
        let mut link = false;
        while let Some(c) = s.eat() {
            match c {
                '`' => {
                    let mut raw = s.eat_until('`');
                    if (raw.starts_with('{') && raw.ends_with('}'))
                        || (raw.starts_with('[') && raw.ends_with(']'))
                    {
                        raw = &raw[1..raw.len() - 1];
                    }

                    s.eat();
                    output.push('`');
                    output.push_str(raw);
                    output.push('`');
                }
                '[' => link = true,
                ']' if link => {
                    if s.eat_if('(') {
                        s.eat_until(')');
                        s.eat();
                    } else if s.eat_if('[') {
                        s.eat_until(']');
                        s.eat();
                    }
                    link = false
                }
                '*' | '_' => {}
                '.' => {
                    output.push('.');
                    // Avoid stopping on things like `See foo.bar.` or `e.g.`.
                    if (s.done() || s.at(char::is_whitespace)) && s.scout(-3) != Some('.')
                    {
                        break;
                    }
                }
                _ => output.push(c),
            }
        }

        output
    }
}

impl Deref for Docs {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Native(s) => s,
            Self::Comment(s) => s,
        }
    }
}

impl From<Docs> for EcoString {
    fn from(docs: Docs) -> Self {
        match docs {
            Docs::Native(s) => s.into(),
            Docs::Comment(s) => s,
        }
    }
}
