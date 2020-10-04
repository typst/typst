//! Layouting of syntax trees into box layouts.

pub mod elements;
pub mod line;
pub mod primitive;
pub mod stack;
mod tree;

pub use primitive::*;
pub use tree::layout_tree as layout;

use crate::geom::{Insets, Point, Rect, RectExt, Sides, Size, SizeExt};

use crate::eval::Scope;
use crate::font::SharedFontLoader;
use crate::style::{LayoutStyle, PageStyle, TextStyle};
use crate::syntax::SynTree;

use elements::LayoutElements;

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

/// The context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext<'a> {
    /// The font loader to query fonts from when typesetting text.
    pub loader: &'a SharedFontLoader,
    /// The function scope.
    pub scope: &'a Scope,
    /// The style for pages and text.
    pub style: &'a LayoutStyle,
    /// The unpadded size of this container (the base 100% for relative sizes).
    pub base: Size,
    /// The spaces to layout into.
    pub spaces: LayoutSpaces,
    /// Whether to spill over into copies of the last space or finish layouting
    /// when the last space is used up.
    pub repeat: bool,
    /// The system into which content is laid out.
    pub sys: LayoutSystem,
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
    pub insets: Insets,
    /// Whether to expand the size of the resulting layout to the full size of
    /// this space or to shrink it to fit the content.
    pub expansion: LayoutExpansion,
}

impl LayoutSpace {
    /// The position of the padded start in the space.
    pub fn start(&self) -> Point {
        Point::new(-self.insets.x0, -self.insets.y0)
    }

    /// The actually usable area (size minus padding).
    pub fn usable(&self) -> Size {
        self.size + self.insets.size()
    }

    /// The inner layout space with size reduced by the padding, zero padding of
    /// its own and no layout expansion.
    pub fn inner(&self) -> Self {
        Self {
            size: self.usable(),
            insets: Insets::ZERO,
            expansion: LayoutExpansion::new(false, false),
        }
    }
}

/// A sequence of layouting commands.
pub type Commands = Vec<Command>;

/// Commands executable by the layouting engine.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Layout the given tree in the current context (i.e. not nested). The
    /// content of the tree is not laid out into a separate box and then added,
    /// but simply laid out flatly in the active layouting process.
    ///
    /// This has the effect that the content fits nicely into the active line
    /// layouting, enabling functions to e.g. change the style of some piece of
    /// text while keeping it part of the current paragraph.
    LayoutSyntaxTree(SynTree),

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
    /// Update the layouting system along which future boxes will be laid
    /// out. This ends the current line.
    SetSystem(LayoutSystem),
}

/// Defines how spacing interacts with surrounding spacing.
///
/// There are two options for interaction: Hard and soft spacing. Typically,
/// hard spacing is used when a fixed amount of space needs to be inserted no
/// matter what. In contrast, soft spacing can be used to insert a default
/// spacing between e.g. two words or paragraphs that can still be overridden by
/// a hard space.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SpacingKind {
    /// Hard spaces are always laid out and consume surrounding soft space.
    Hard,
    /// Soft spaces are not laid out if they are touching a hard space and
    /// consume neighbouring soft spaces with higher levels.
    Soft(u32),
}

impl SpacingKind {
    /// The standard spacing kind used for paragraph spacing.
    pub const PARAGRAPH: Self = Self::Soft(1);

    /// The standard spacing kind used for line spacing.
    pub const LINE: Self = Self::Soft(2);

    /// The standard spacing kind used for word spacing.
    pub const WORD: Self = Self::Soft(1);
}

/// The spacing kind of the most recently inserted item in a layouting process.
///
/// Since the last inserted item may not be spacing at all, this can be `None`.
#[derive(Debug, Copy, Clone, PartialEq)]
enum LastSpacing {
    /// The last item was hard spacing.
    Hard,
    /// The last item was soft spacing with the given width and level.
    Soft(f64, u32),
    /// The last item wasn't spacing.
    None,
}

impl LastSpacing {
    /// The width of the soft space if this is a soft space or zero otherwise.
    fn soft_or_zero(self) -> f64 {
        match self {
            LastSpacing::Soft(space, _) => space,
            _ => 0.0,
        }
    }
}
