//! Generation of abstract documents from syntax trees.

#![allow(dead_code)]

use std::fmt;
use crate::parsing::{SyntaxTree, Node};


/// Abstract representation of a complete typesetted document.
///
/// This abstract thing can then be serialized into a specific format like PDF.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The pages of the document.
    pub pages: Vec<Page>,
}

impl Document {
    /// Create a new document without content.
    pub fn new() -> Document {
        Document {
            pages: vec![],
        }
    }
}

/// A page of a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// The width and height of the page.
    pub size: [Size; 2],
    /// The contents of the page.
    pub contents: Vec<Text>,
}

/// Plain text.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Text(pub String);

/// A general distance type that can convert between units.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Size {
    /// The size in typographic points (1/72 inches).
    points: f32,
}

impl Size {
    /// Create a size from a number of points.
    #[inline]
    pub fn from_points(points: f32) -> Size {
        Size { points }
    }

    /// Create a size from a number of inches.
    #[inline]
    pub fn from_inches(inches: f32) -> Size {
        Size { points: 72.0 * inches }
    }

    /// Create a size from a number of millimeters.
    #[inline]
    pub fn from_mm(mm: f32) -> Size {
        Size { points: 2.83465 * mm  }
    }

    /// Create a size from a number of centimeters.
    #[inline]
    pub fn from_cm(cm: f32) -> Size {
        Size { points: 28.3465 * cm }
    }

    /// Create a size from a number of points.
    #[inline]
    pub fn to_points(&self) -> f32 {
        self.points
    }

    /// Create a size from a number of inches.
    #[inline]
    pub fn to_inches(&self) -> f32 {
        self.points * 0.0138889
    }

    /// Create a size from a number of millimeters.
    #[inline]
    pub fn to_mm(&self) -> f32 {
        self.points * 0.352778
    }

    /// Create a size from a number of centimeters.
    #[inline]
    pub fn to_cm(&self) -> f32 {
        self.points * 0.0352778
    }
}


/// A type that can be generated into a document.
pub trait Generate {
    /// Generate a document from self.
    fn generate(self) -> GenResult<Document>;
}

impl Generate for SyntaxTree<'_> {
    fn generate(self) -> GenResult<Document> {
        Generator::new(self).generate()
    }
}

/// Result type used for parsing.
type GenResult<T> = std::result::Result<T, GenerationError>;

/// A failure when generating.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GenerationError {
    /// A message describing the error.
    message: String,
}

impl fmt::Display for GenerationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "generation error: {}", self.message)
    }
}


/// Transforms an abstract syntax tree into a document.
#[derive(Debug, Clone)]
struct Generator<'s> {
    tree: SyntaxTree<'s>,
}

impl<'s> Generator<'s> {
    /// Create a new generator from a syntax tree.
    fn new(tree: SyntaxTree<'s>) -> Generator<'s> {
        Generator { tree }
    }

    /// Generate the abstract document.
    fn generate(&mut self) -> GenResult<Document> {
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
            size: [Size::from_mm(210.0), Size::from_mm(297.0)],
            contents: vec![ Text(text) ],
        };

        Ok(Document {
            pages: vec![page],
        })
    }

    /// Gives a generation error with a message.
    #[inline]
    fn err<R, S: Into<String>>(&self, message: S) -> GenResult<R> {
        Err(GenerationError { message: message.into() })
    }
}


#[cfg(test)]
mod generator_tests {
    use super::*;
    use crate::parsing::{Tokenize, Parse};

    /// Test if the source gets generated into the document.
    fn test(src: &str, doc: Document) {
        assert_eq!(src.tokenize().parse().unwrap().generate(), Ok(doc));
    }

    /// Test if generation gives this error for the source code.
    fn test_err(src: &str, err: GenerationError) {
        assert_eq!(src.tokenize().parse().unwrap().generate(), Err(err));
    }

    #[test]
    fn generator_simple() {
        test("This is an example of a sentence.", Document {
            pages: vec![
                Page {
                    size: [Size::from_mm(210.0), Size::from_mm(297.0)],
                    contents: vec![
                        Text("This is an example of a sentence.".to_owned()),
                    ]
                }
            ],
        });
    }
}
