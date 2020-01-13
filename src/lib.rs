//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens](crate::syntax::Tokens). Then, a parser constructs a
//!   syntax tree from the token stream. The structures describing the tree can
//!   be found in the [syntax](crate::syntax) module.
//! - **Layouting:** The next step is to transform the syntax tree into a
//!   portable representation of the typesetted document. Types for these can be
//!   found in the [layout] module. A finished layout reading for exporting is a
//!   [multi-layout](crate::layout::MultiLayout) consisting of multiple boxes
//!   (or pages).
//! - **Exporting:** The finished layout can then be exported into a supported
//!   format. Submodules for these formats are located in the
//!   [export](crate::export) module. Currently, the only supported output
//!   format is _PDF_. Alternatively, the layout can be serialized to pass it to
//!   a suitable renderer.

#![allow(unused)]

pub extern crate toddle;

use std::cell::RefCell;
use smallvec::smallvec;

use toddle::query::{FontLoader, FontProvider, SharedFontLoader};
use toddle::Error as FontError;

use crate::func::Scope;
use crate::layout::{MultiLayout, LayoutResult};
use crate::syntax::{parse, SyntaxTree, ParseContext, Span, ParseResult};
use crate::style::{LayoutStyle, PageStyle, TextStyle};

#[macro_use]
mod macros;
pub mod export;
#[macro_use]
pub mod func;
pub mod layout;
pub mod library;
pub mod syntax;
pub mod size;
pub mod style;


/// Transforms source code into typesetted layouts.
///
/// A typesetter can be configured through various methods.
pub struct Typesetter<'p> {
    /// The font loader shared by all typesetting processes.
    loader: SharedFontLoader<'p>,
    /// The base layouting style.
    style: LayoutStyle,
}

impl<'p> Typesetter<'p> {
    /// Create a new typesetter.
    pub fn new() -> Typesetter<'p> {
        Typesetter {
            loader: RefCell::new(FontLoader::new()),
            style: LayoutStyle::default(),
        }
    }

    /// Set the base page style.
    pub fn set_page_style(&mut self, style: PageStyle) {
        self.style.page = style;
    }

    /// Set the base text style.
    pub fn set_text_style(&mut self, style: TextStyle) {
        self.style.text = style;
    }

    /// Add a font provider to the context of this typesetter.
    pub fn add_font_provider<P: 'p>(&mut self, provider: P)
    where P: FontProvider {
        self.loader.get_mut().add_provider(provider);
    }

    /// A reference to the backing font loader.
    pub fn loader(&self) -> &SharedFontLoader<'p> {
        &self.loader
    }

    /// Parse source code into a syntax tree.
    pub fn parse(&self, src: &str) -> SyntaxTree {
        let scope = Scope::with_std();
        parse(src, ParseContext { scope: &scope })
    }

    /// Layout a syntax tree and return the produced layout.
    pub async fn layout(&self, tree: &SyntaxTree) -> LayoutResult<MultiLayout> {
        use crate::layout::prelude::*;
        let margins = self.style.page.margins();
        Ok(layout(
            &tree,
            LayoutContext {
                loader: &self.loader,
                style: &self.style,
                base: self.style.page.dimensions.unpadded(margins),
                spaces: smallvec![LayoutSpace {
                    dimensions: self.style.page.dimensions,
                    padding: margins,
                    expansion: LayoutExpansion::new(true, true),
                }],
                repeat: true,
                axes: LayoutAxes::new(LeftToRight, TopToBottom),
                alignment: LayoutAlignment::new(Origin, Origin),
                nested: false,
                debug: false,
            },
        ).await?)
    }

    /// Process source code directly into a layout.
    pub async fn typeset(&self, src: &str) -> TypesetResult<MultiLayout> {
        let tree = self.parse(src);
        let layout = self.layout(&tree).await?;
        Ok(layout)
    }
}

/// The result type for typesetting.
pub type TypesetResult<T> = Result<T, TypesetError>;

/// The error type for typesetting.
pub struct TypesetError {
    pub message: String,
    pub span: Option<Span>,
}

impl TypesetError {
    /// Create a new typesetting error.
    pub fn with_message(message: String) -> TypesetError {
        TypesetError { message, span: None }
    }
}

error_type! {
    self: TypesetError,
    show: f => {
        write!(f, "{}", self.message)?;
        if let Some(span) = self.span {
            write!(f, " at {}", span)?;
        }
        Ok(())
    },
    from: (err: std::io::Error, TypesetError::with_message(err.to_string())),
    from: (err: FontError, TypesetError::with_message(err.to_string())),
}
