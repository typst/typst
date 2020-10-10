//! Layouting of documents.

mod document;
mod fixed;
mod node;
mod pad;
mod par;
mod spacing;
mod stack;
mod text;

use async_trait::async_trait;

use crate::font::SharedFontLoader;
use crate::geom::*;
use crate::shaping::Shaped;

pub use document::*;
pub use fixed::*;
pub use node::*;
pub use pad::*;
pub use par::*;
pub use spacing::*;
pub use stack::*;
pub use text::*;

/// Layout a document and return the produced layouts.
pub async fn layout(document: &Document, loader: SharedFontLoader) -> Vec<BoxLayout> {
    let mut ctx = LayoutContext { loader };
    document.layout(&mut ctx).await
}

/// The context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext {
    /// The font loader to query fonts from when typesetting text.
    pub loader: SharedFontLoader,
}

/// Layout a node.
#[async_trait(?Send)]
pub trait Layout {
    /// Layout the node in the given layout context.
    ///
    /// This signature looks pretty horrible due to async in trait methods, but
    /// it's actually just the following:
    /// ```rust,ignore
    /// async fn layout(
    ///     &self,
    ///     ctx: &mut LayoutContext,
    ///     constraints: LayoutConstraints,
    /// ) -> Vec<LayoutItem>;
    /// ```
    async fn layout(
        &self,
        ctx: &mut LayoutContext,
        constraints: LayoutConstraints,
    ) -> Vec<LayoutItem>;
}

/// An item that is produced by [layouting] a node.
///
/// [layouting]: trait.Layout.html#method.layout
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutItem {
    /// Spacing that should be added to the parent.
    Spacing(Length),
    /// A box that should be aligned in the parent.
    Box(BoxLayout, Gen<Align>),
}

/// The constraints for layouting a single node.
#[derive(Debug, Clone)]
pub struct LayoutConstraints {
    /// The spaces to layout into.
    pub spaces: Vec<LayoutSpace>,
    /// Whether to spill over into copies of the last space or finish layouting
    /// when the last space is used up.
    pub repeat: bool,
}

/// The space into which content is laid out.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LayoutSpace {
    /// The full size of this container (the base for relative sizes).
    pub base: Size,
    /// The maximum size of the rectangle to layout into.
    pub size: Size,
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
            self.push(pos + subpos, element);
        }
    }
}

/// A layout element, the basic building block layouts are composed of.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutElement {
    /// Shaped text.
    Text(Shaped),
}
