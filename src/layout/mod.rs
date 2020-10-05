//! Layouting of syntax trees.

pub mod primitive;

mod line;
mod stack;
mod tree;

pub use line::*;
pub use primitive::*;
pub use stack::*;
pub use tree::*;

use crate::geom::{Insets, Point, Rect, RectExt, Sides, Size, SizeExt};

use crate::eval::{PageState, State, TextState};
use crate::font::SharedFontLoader;
use crate::shaping::Shaped;
use crate::syntax::SynTree;
use crate::{Feedback, Pass};

/// Layout a syntax tree and return the produced layout.
pub async fn layout(
    tree: &SynTree,
    state: State,
    loader: SharedFontLoader,
) -> Pass<Vec<BoxLayout>> {
    let space = LayoutSpace {
        size: state.page.size,
        insets: state.page.insets(),
        expansion: LayoutExpansion::new(true, true),
    };

    let constraints = LayoutConstraints {
        root: true,
        base: space.usable(),
        spaces: vec![space],
        repeat: true,
    };

    let mut ctx = LayoutContext {
        loader,
        state,
        constraints,
        f: Feedback::new(),
    };

    let layouts = layout_tree(&tree, &mut ctx).await;
    Pass::new(layouts, ctx.f)
}

/// A finished box with content at fixed positions.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxLayout {
    /// The size of the box.
    pub size: Size,
    /// The elements composing this layout.
    pub elements: Vec<(Point, LayoutElement)>,
}

impl BoxLayout {
    /// Create a new empty collection.
    pub fn new(size: Size) -> Self {
        Self { size, elements: vec![] }
    }

    /// Add an element at a position.
    pub fn push(&mut self, pos: Point, element: LayoutElement) {
        self.elements.push((pos, element));
    }

    /// Add all elements of another collection, placing them relative to the
    /// given position.
    pub fn push_layout(&mut self, pos: Point, more: Self) {
        for (subpos, element) in more.elements {
            self.push(pos + subpos.to_vec2(), element);
        }
    }
}

/// A layout element, the basic building block layouts are composed of.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutElement {
    /// Shaped text.
    Text(Shaped),
}

/// The context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext {
    /// The font loader to query fonts from when typesetting text.
    pub loader: SharedFontLoader,
    /// The active state.
    pub state: State,
    /// The active constraints.
    pub constraints: LayoutConstraints,
    /// The accumulated feedback.
    pub f: Feedback,
}

/// The constraints for layouting a single node.
#[derive(Debug, Clone)]
pub struct LayoutConstraints {
    /// Whether this layouting process is the root page-building process.
    pub root: bool,
    /// The unpadded size of this container (the base 100% for relative sizes).
    pub base: Size,
    /// The spaces to layout into.
    pub spaces: Vec<LayoutSpace>,
    /// Whether to spill over into copies of the last space or finish layouting
    /// when the last space is used up.
    pub repeat: bool,
}

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
    Add(BoxLayout, LayoutAlign),
    /// Add spacing of the given kind along the primary or secondary axis. The
    /// kind defines how the spacing interacts with surrounding spacing.
    AddSpacing(f64, SpacingKind, GenAxis),

    /// Start a new line.
    BreakLine,
    /// Start a new page, which will be part of the finished layout even if it
    /// stays empty (since the page break is a _hard_ space break).
    BreakPage,

    /// Update the text style.
    SetTextState(TextState),
    /// Update the page style.
    SetPageState(PageState),
    /// Update the layouting system along which future boxes will be laid
    /// out. This ends the current line.
    SetSystem(LayoutSystem),
    /// Update the alignment for future boxes added to this layouting process.
    SetAlignment(LayoutAlign),
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
