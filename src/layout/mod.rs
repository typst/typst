//! Layouting of syntax trees into box layouts.

pub mod elements;
pub mod line;
pub mod primitive;
pub mod stack;
pub mod text;
mod tree;

/// Basic types used across the layouting engine.
pub mod prelude {
    pub use super::primitive::*;
    pub use super::layout;
    pub use Dir::*;
    pub use GenAlign::*;
    pub use GenAxis::*;
    pub use SpecAlign::*;
    pub use SpecAxis::*;
}

pub use primitive::*;
pub use tree::layout_tree as layout;

use async_trait::async_trait;

use crate::font::SharedFontLoader;
use crate::geom::{Margins, Size};
use crate::style::{LayoutStyle, PageStyle, TextStyle};
use crate::syntax::tree::SyntaxTree;
use crate::Pass;

use elements::LayoutElements;
use prelude::*;

/// A collection of layouts.
pub type MultiLayout = Vec<BoxLayout>;

/// A finished box with content at fixed positions.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxLayout {
    /// The size of the box.
    pub size: Size,
    /// How to align this box in a parent container.
    pub align: LayoutAlign,
    /// The elements composing this layout.
    pub elements: LayoutElements,
}

/// Comamnd-based layout.
#[async_trait(?Send)]
pub trait Layout {
    /// Create a sequence of layouting commands to execute.
    async fn layout<'a>(&'a self, ctx: LayoutContext<'_>) -> Pass<Commands<'a>>;
}

/// The context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext<'a> {
    /// The font loader to query fonts from when typesetting text.
    pub loader: &'a SharedFontLoader,
    /// The style for pages and text.
    pub style: &'a LayoutStyle,
    /// The unpadded size of this container (the base 100% for relative sizes).
    pub base: Size,
    /// The spaces to layout into.
    pub spaces: LayoutSpaces,
    /// Whether to spill over into copies of the last space or finish layouting
    /// when the last space is used up.
    pub repeat: bool,
    /// The axes along which content is laid out.
    pub axes: LayoutAxes,
    /// The alignment of the _resulting_ layout. This does not effect the line
    /// layouting itself, but rather how the finished layout will be positioned
    /// in a parent layout.
    pub align: LayoutAlign,
    /// Whether this layouting process is the root page-building process.
    pub root: bool,
}

/// A collection of layout spaces.
pub type LayoutSpaces = Vec<LayoutSpace>;

/// The space into which content is laid out.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LayoutSpace {
    /// The maximum size of the rectangle to layout into.
    pub size: Size,
    /// Padding that should be respected on each side.
    pub padding: Margins,
    /// Whether to expand the size of the resulting layout to the full size of
    /// this space or to shrink it to fit the content.
    pub expansion: LayoutExpansion,
}

impl LayoutSpace {
    /// The offset from the origin to the start of content, i.e.
    /// `(padding.left, padding.top)`.
    pub fn start(&self) -> Size {
        Size::new(self.padding.left, self.padding.top)
    }

    /// The actually usable area (size minus padding).
    pub fn usable(&self) -> Size {
        self.size.unpadded(self.padding)
    }

    /// The inner layout space with size reduced by the padding, zero padding of
    /// its own and no layout expansion.
    pub fn inner(&self) -> Self {
        Self {
            size: self.usable(),
            padding: Margins::ZERO,
            expansion: LayoutExpansion::new(false, false),
        }
    }
}

/// A sequence of layouting commands.
pub type Commands<'a> = Vec<Command<'a>>;

/// Commands executable by the layouting engine.
#[derive(Debug, Clone)]
pub enum Command<'a> {
    /// Layout the given tree in the current context (i.e. not nested). The
    /// content of the tree is not laid out into a separate box and then added,
    /// but simply laid out flatly in the active layouting process.
    ///
    /// This has the effect that the content fits nicely into the active line
    /// layouting, enabling functions to e.g. change the style of some piece of
    /// text while keeping it part of the current paragraph.
    LayoutSyntaxTree(&'a SyntaxTree),

    /// Add a finished layout.
    Add(BoxLayout),
    /// Add multiple layouts, one after another. This is equivalent to multiple
    /// `Add` commands.
    AddMultiple(MultiLayout),

    /// Add spacing of the given kind along the primary or secondary axis. The
    /// kind defines how the spacing interacts with surrounding spacing.
    AddSpacing(f64, SpacingKind, GenAxis),

    /// Start a new line.
    BreakLine,
    /// Start a new page, which will be part of the finished layout even if it
    /// stays empty (since the page break is a _hard_ space break).
    BreakPage,

    /// Update the text style.
    SetTextStyle(TextStyle),
    /// Update the page style.
    SetPageStyle(PageStyle),

    /// Update the alignment for future boxes added to this layouting process.
    SetAlignment(LayoutAlign),
    /// Update the layouting axes along which future boxes will be laid
    /// out. This ends the current line.
    SetAxes(LayoutAxes),
}
