//! Core typesetting engine.

use std::error;
use std::fmt;
use crate::syntax::{SyntaxTree, Node};
use crate::doc::{Document, Style, Page, Text, TextCommand};
use crate::font::Font;


/// The core typesetting engine, transforming an abstract syntax tree into a document.
#[derive(Debug, Clone)]
pub struct Engine<'s> {
    tree: SyntaxTree<'s>,
}

impl<'s> Engine<'s> {
    /// Create a new generator from a syntax tree.
    pub fn new(tree: SyntaxTree<'s>) -> Engine<'s> {
        Engine { tree }
    }

    /// Generate the abstract document.
    pub fn typeset(&mut self) -> TypeResult<Document> {
        let style = Style::default();

        // Load font defined by style
        let font_family = style.font_families.first().unwrap();
        let program = std::fs::read(format!("../fonts/{}-Regular.ttf", font_family)).unwrap();
        let font = Font::new(program).unwrap();

        let mut text = String::new();
        for node in &self.tree.nodes {
            match node {
                Node::Space if !text.is_empty() => text.push(' '),
                Node::Space | Node::Newline => (),
                Node::Word(word) => text.push_str(word),

                Node::ToggleItalics | Node::ToggleBold | Node::ToggleMath => unimplemented!(),
                Node::Func(_) => unimplemented!(),
            }
        }

        let page = Page {
            size: style.paper_size,
            text: vec![Text {
                commands: vec![
                    TextCommand::Move(style.margins[0], style.paper_size[1] - style.margins[1]),
                    TextCommand::SetFont(0, style.font_size),
                    TextCommand::Text(text)
                ]
            }],
        };

        Ok(Document {
            pages: vec![page],
            fonts: vec![font],
        })
    }
}

/// Result type used for parsing.
type TypeResult<T> = std::result::Result<T, TypesetError>;

/// The error type for typesetting.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TypesetError {
    message: String,
}

impl error::Error for TypesetError {}

impl fmt::Display for TypesetError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.message)
    }
}
