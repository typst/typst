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
//!   [multi layout](crate::layout::MultiLayout) consisting of multiple boxes (or
//!   pages).
//! - **Exporting:** The finished document can finally be exported into a supported
//!   format. Submodules for these formats are located in the [export](crate::export)
//!   module. Currently, the only supported output format is _PDF_.

pub extern crate toddle;

use std::cell::RefCell;
use smallvec::smallvec;
use toddle::query::{FontLoader, FontProvider, SharedFontLoader};

use crate::func::Scope;
use crate::layout::{layout_tree, MultiLayout, LayoutContext};
use crate::layout::{LayoutAxes, AlignedAxis, Axis, Alignment};
use crate::layout::{LayoutError, LayoutResult, LayoutSpace};
use crate::syntax::{SyntaxTree, parse, ParseContext, ParseError, ParseResult};
use crate::style::{PageStyle, TextStyle};

#[macro_use]
mod macros;
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
    /// The base text style.
    text_style: TextStyle,
    /// The base page style.
    page_style: PageStyle,
}

impl<'p> Typesetter<'p> {
    /// Create a new typesetter.
    #[inline]
    pub fn new() -> Typesetter<'p> {
        Typesetter {
            loader: RefCell::new(FontLoader::new()),
            text_style: TextStyle::default(),
            page_style: PageStyle::default(),
        }
    }

    /// Set the base page style.
    #[inline]
    pub fn set_page_style(&mut self, style: PageStyle) {
        self.page_style = style;
    }

    /// Set the base text style.
    #[inline]
    pub fn set_text_style(&mut self, style: TextStyle) {
        self.text_style = style;
    }

    /// Add a font provider to the context of this typesetter.
    #[inline]
    pub fn add_font_provider<P: 'p>(&mut self, provider: P)
    where P: FontProvider {
        self.loader.get_mut().add_provider(provider);
    }

    /// A reference to the backing font loader.
    #[inline]
    pub fn loader(&self) -> &SharedFontLoader<'p> {
        &self.loader
    }

    /// Parse source code into a syntax tree.
    pub fn parse(&self, src: &str) -> ParseResult<SyntaxTree> {
        let scope = Scope::with_std();
        parse(src, ParseContext { scope: &scope })
    }

    /// Layout a syntax tree and return the produced layout.
    pub fn layout(&self, tree: &SyntaxTree) -> LayoutResult<MultiLayout> {
        Ok(layout_tree(
            &tree,
            LayoutContext {
                loader: &self.loader,
                top_level: true,
                text_style: &self.text_style,
                page_style: self.page_style,
                spaces: smallvec![LayoutSpace {
                    dimensions: self.page_style.dimensions,
                    padding: self.page_style.margins,
                }],
                axes: LayoutAxes {
                    primary: AlignedAxis::new(Axis::LeftToRight, Alignment::Origin),
                    secondary: AlignedAxis::new(Axis::TopToBottom, Alignment::Origin),
                },
                shrink_to_fit: false,
            },
        )?)
    }

    /// Process source code directly into a layout.
    pub fn typeset(&self, src: &str) -> Result<MultiLayout, TypesetError> {
        let tree = self.parse(src)?;
        let layout = self.layout(&tree)?;
        Ok(layout)
    }
}

/// The general error type for typesetting.
pub enum TypesetError {
    /// An error that occured in the parsing step.
    Parse(ParseError),
    /// An error that occured in the layouting step.
    Layout(LayoutError),
}

error_type! {
    err: TypesetError,
    show: f => match err {
        TypesetError::Parse(e) => write!(f, "parse error: {}", e),
        TypesetError::Layout(e) => write!(f, "layout error: {}", e),
    },
    source: match err {
        TypesetError::Parse(e) => Some(e),
        TypesetError::Layout(e) => Some(e),
    },
    from: (ParseError, TypesetError::Parse(err)),
    from: (LayoutError, TypesetError::Layout(err)),
}
