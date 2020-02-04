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

pub use toddle;

use std::cell::RefCell;
use std::fmt::Debug;
use async_trait::async_trait;
use smallvec::smallvec;

use toddle::{Font, OwnedData};
use toddle::query::{FontLoader, FontProvider, SharedFontLoader, FontDescriptor};

use crate::layout::{Layouted, MultiLayout};
use crate::style::{LayoutStyle, PageStyle, TextStyle};
use crate::syntax::{SyntaxModel, Scope, ParseContext, Parsed, parse};
use crate::syntax::span::Position;


/// Declare a module and reexport all its contents.
macro_rules! pub_use_mod {
    ($name:ident) => {
        mod $name;
        pub use $name::*;
    };
}

#[macro_use]
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
#[derive(Debug)]
pub struct Typesetter {
    /// The font loader shared by all typesetting processes.
    loader: GlobalFontLoader,
    /// The base layouting style.
    style: LayoutStyle,
    /// The standard library scope.
    scope: Scope,
}

/// The font loader type used in the [`Typesetter`].
///
/// This font loader is ref-cell protected and backed by a dynamic font
/// provider.
pub type GlobalFontLoader = SharedFontLoader<GlobalProvider>;

/// The provider type of font loaders used in the [`Typesetter`].
pub type GlobalProvider = Box<dyn FontProvider<Data=OwnedData, Error=Box<dyn Debug>>>;

impl Typesetter {
    /// Create a new typesetter.
    pub fn new(provider: (GlobalProvider, Vec<FontDescriptor>)) -> Typesetter {
        Typesetter {
            loader: RefCell::new(FontLoader::new(provider)),
            style: LayoutStyle::default(),
            scope: Scope::with_std(),
        }
    }

    /// Set the base text style.
    pub fn set_text_style(&mut self, style: TextStyle) {
        self.style.text = style;
    }

    /// Set the base page style.
    pub fn set_page_style(&mut self, style: PageStyle) {
        self.style.page = style;
    }

    /// A reference to the backing font loader.
    pub fn loader(&self) -> &GlobalFontLoader {
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

/// Wraps a font provider and transforms its errors into boxed [`Debug`] trait
/// objects. This enables font providers that do not return these boxed errors
/// to be used with the typesetter.
#[derive(Debug)]
pub struct DebugErrorProvider<P> {
    provider: P,
}

impl<P> DebugErrorProvider<P>
where P: FontProvider, P::Error: Debug + 'static {
    /// Create a new debug error provider from any provider.
    pub fn new(provider: P) -> DebugErrorProvider<P> {
        DebugErrorProvider { provider }
    }
}

#[async_trait(?Send)]
impl<P> FontProvider for DebugErrorProvider<P>
where P: FontProvider, P::Error: Debug + 'static {
    type Data = P::Data;
    type Error = Box<dyn Debug>;

    async fn load(&self, index: usize, variant: usize) -> Result<Font<P::Data>, Self::Error> {
        self.provider.load(index, variant).await
            .map_err(|d| Box::new(d) as Box<dyn Debug>)
    }
}
