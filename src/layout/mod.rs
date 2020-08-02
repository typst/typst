//! Layouting types and engines.

use async_trait::async_trait;

use crate::Pass;
use crate::font::SharedFontLoader;
use crate::geom::{Size, Margins};
use crate::style::{LayoutStyle, TextStyle, PageStyle};
use crate::syntax::tree::SyntaxTree;

use elements::LayoutElements;
use tree::TreeLayouter;
use prelude::*;

pub mod elements;
pub mod line;
pub mod primitive;
pub mod stack;
pub mod text;
pub mod tree;

pub use primitive::*;

/// Basic types used across the layouting engine.
pub mod prelude {
    pub use super::layout;
    pub use super::primitive::*;
    pub use Dir::*;
    pub use GenAxis::*;
    pub use SpecAxis::*;
    pub use GenAlign::*;
    pub use SpecAlign::*;
}

/// A collection of layouts.
pub type MultiLayout = Vec<BoxLayout>;

/// A finished box with content at fixed positions.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxLayout {
    /// The size of the box.
    pub size: Size,
    /// How to align this layout in a parent container.
    pub align: LayoutAlign,
    /// The elements composing this layout.
    pub elements: LayoutElements,
}

/// Layouting of elements.
#[async_trait(?Send)]
pub trait Layout {
    /// Layout self into a sequence of layouting commands.
    async fn layout<'a>(&'a self, _: LayoutContext<'_>) -> Pass<Commands<'a>>;
}

/// Layout a syntax tree into a list of boxes.
pub async fn layout(tree: &SyntaxTree, ctx: LayoutContext<'_>) -> Pass<MultiLayout> {
    let mut layouter = TreeLayouter::new(ctx);
    layouter.layout_tree(tree).await;
    layouter.finish()
}

/// The context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext<'a> {
    /// The font loader to retrieve fonts from when typesetting text
    /// using [`layout_text`].
    pub loader: &'a SharedFontLoader,
    /// The style for pages and text.
    pub style: &'a LayoutStyle,
    /// The base unpadded size of this container (for relative sizing).
    pub base: Size,
    /// The spaces to layout in.
    pub spaces: LayoutSpaces,
    /// Whether to have repeated spaces or to use only the first and only once.
    pub repeat: bool,
    /// The initial axes along which content is laid out.
    pub axes: LayoutAxes,
    /// The alignment of the finished layout.
    pub align: LayoutAlign,
    /// Whether the layout that is to be created will be nested in a parent
    /// container.
    pub nested: bool,
}

/// A collection of layout spaces.
pub type LayoutSpaces = Vec<LayoutSpace>;

/// The space into which content is laid out.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LayoutSpace {
    /// The maximum size of the box to layout in.
    pub size: Size,
    /// Padding that should be respected on each side.
    pub padding: Margins,
    /// Whether to expand the size of the resulting layout to the full size of
    /// this space or to shrink them to fit the content.
    pub expansion: LayoutExpansion,
}

impl LayoutSpace {
    /// The offset from the origin to the start of content, that is,
    /// `(padding.left, padding.top)`.
    pub fn start(&self) -> Size {
        Size::new(self.padding.left, self.padding.top)
    }

    /// The actually usable area (size minus padding).
    pub fn usable(&self) -> Size {
        self.size.unpadded(self.padding)
    }

    /// A layout space without padding and size reduced by the padding.
    pub fn usable_space(&self) -> LayoutSpace {
        LayoutSpace {
            size: self.usable(),
            padding: Margins::ZERO,
            expansion: LayoutExpansion::new(false, false),
        }
    }
}

/// A sequence of layouting commands.
pub type Commands<'a> = Vec<Command<'a>>;

/// Commands issued to the layouting engine by trees.
#[derive(Debug, Clone)]
pub enum Command<'a> {
    /// Layout the given tree in the current context (i.e. not nested). The
    /// content of the tree is not laid out into a separate box and then added,
    /// but simply laid out flat in the active layouting process.
    ///
    /// This has the effect that the content fits nicely into the active line
    /// layouting, enabling functions to e.g. change the style of some piece of
    /// text while keeping it integrated in the current paragraph.
    LayoutSyntaxTree(&'a SyntaxTree),

    /// Add a already computed layout.
    Add(BoxLayout),
    /// Add multiple layouts, one after another. This is equivalent to multiple
    /// [Add](Command::Add) commands.
    AddMultiple(MultiLayout),

    /// Add spacing of given [kind](super::SpacingKind) along the primary or
    /// secondary axis. The spacing kind defines how the spacing interacts with
    /// surrounding spacing.
    AddSpacing(f64, SpacingKind, GenAxis),

    /// Start a new line.
    BreakLine,
    /// Start a new paragraph.
    BreakParagraph,
    /// Start a new page, which will exist in the finished layout even if it
    /// stays empty (since the page break is a _hard_ space break).
    BreakPage,

    /// Update the text style.
    SetTextStyle(TextStyle),
    /// Update the page style.
    SetPageStyle(PageStyle),

    /// Update the alignment for future boxes added to this layouting process.
    SetAlignment(LayoutAlign),
    /// Update the layouting axes along which future boxes will be laid
    /// out. This finishes the current line.
    SetAxes(LayoutAxes),
}
