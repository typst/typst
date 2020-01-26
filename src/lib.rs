//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens](crate::syntax::Tokens). Then, a parser constructs a
//!   syntax tree from the token stream. The structures describing the tree can
//!   be found in the [syntax](crate::syntax) module.
//! - **Layouting:** The next step is to transform the syntax tree into a
//!   portable representation of the typesetted document. Types for these can be
//!   found in the [layout](crate::layout) module. A finished layout reading for
//!   exporting is a [MultiLayout](crate::layout::MultiLayout) consisting of
//!   multiple boxes (or pages).
//! - **Exporting:** The finished layout can then be exported into a supported
//!   format. Submodules for these formats are located in the
//!   [export](crate::export) module. Currently, the only supported output
//!   format is [_PDF_](crate::export::pdf). Alternatively, the layout can be
//!   serialized to pass it to a suitable renderer.

#![allow(unused)]

pub use toddle;

use std::cell::RefCell;
use smallvec::smallvec;
use toddle::query::{FontLoader, FontProvider, SharedFontLoader};

use crate::layout::{Layouted, MultiLayout};
use crate::style::{LayoutStyle, PageStyle, TextStyle};
use crate::syntax::{SyntaxModel, Scope, ParseContext, Parsed, parse};
use crate::syntax::span::Position;

#[macro_use]
mod macros;
pub mod error;
pub mod export;
#[macro_use]
pub mod func;
pub mod layout;
pub mod library;
pub mod size;
pub mod style;
pub mod syntax;


/// Transforms source code into typesetted layouts.
///
/// A typesetter can be configured through various methods.
pub struct Typesetter<'p> {
    /// The font loader shared by all typesetting processes.
    loader: SharedFontLoader<'p>,
    /// The base layouting style.
    style: LayoutStyle,
    /// The standard library scope.
    scope: Scope,
}

impl<'p> Typesetter<'p> {
    /// Create a new typesetter.
    pub fn new() -> Typesetter<'p> {
        Typesetter {
            loader: RefCell::new(FontLoader::new()),
            style: LayoutStyle::default(),
            scope: Scope::with_std(),
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
    pub fn parse(&self, src: &str) -> Parsed<SyntaxModel> {
        parse(Position::ZERO, src, ParseContext { scope: &self.scope })
    }

    /// Layout a syntax tree and return the produced layout.
    pub async fn layout(&self, model: &SyntaxModel) -> Layouted<MultiLayout> {
        use crate::layout::prelude::*;

        let margins = self.style.page.margins();
        crate::layout::layout(
            &model,
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
        ).await
    }

    /// Process source code directly into a collection of layouts.
    pub async fn typeset(&self, src: &str) -> MultiLayout {
        let tree = self.parse(src).output;
        self.layout(&tree).await.output
    }
}
