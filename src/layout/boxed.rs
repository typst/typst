//! Block-style layouting of boxes.

use crate::doc::{Document, Page, TextAction};
use crate::font::Font;
use crate::size::{Size, Size2D};
use super::LayoutSpace;


/// A box layout has a fixed width and height and composes of actions.
#[derive(Debug, Clone)]
pub struct BoxLayout {
    /// The size of the box.
    pub dimensions: Size2D,
    /// The actions composing this layout.
    pub actions: Vec<TextAction>,
}

impl BoxLayout {
    /// Convert this layout into a document given the list of fonts referenced by it.
    pub fn into_doc(self, fonts: Vec<Font>) -> Document {
        Document {
            pages: vec![Page {
                width: self.dimensions.x,
                height: self.dimensions.y,
                actions: self.actions,
            }],
            fonts,
        }
    }
}

/// The context for layouting boxes.
#[derive(Debug, Copy, Clone)]
pub struct BoxContext {
    /// The space to layout the boxes in.
    pub space: LayoutSpace,
}

/// Layouts boxes block-style.
#[derive(Debug)]
pub struct BoxLayouter {
    ctx: BoxContext,
    actions: Vec<TextAction>,
    dimensions: Size2D,
    usable: Size2D,
    cursor: Size2D,
}

impl BoxLayouter {
    /// Create a new box layouter.
    pub fn new(ctx: BoxContext) -> BoxLayouter {
        let space = ctx.space;
        BoxLayouter {
            ctx,
            actions: vec![],
            dimensions: Size2D::zero(),
            usable: space.usable(),
            cursor: Size2D::new(space.padding.left, space.padding.right),
        }
    }

    /// Add a sublayout.
    pub fn add_box(&mut self, layout: BoxLayout) {
        // In the flow direction (vertical) add the layout and in the second
        // direction just consider the maximal size of any child layout.
        let new = Size2D {
            x: crate::size::max(self.dimensions.x, layout.dimensions.x),
            y: self.dimensions.y + layout.dimensions.y,
        };

        if self.overflows(new) {
            panic!("box layouter: would overflow in add_box");
        }

        // Apply the dimensions because they fit.
        self.dimensions = new;

        // Move all actions into this layout and translate absolute positions.
        self.actions.push(TextAction::MoveAbsolute(self.cursor));
        self.actions.extend(super::translate_actions(self.cursor, layout.actions));

        // Adjust the cursor.
        self.cursor.y += layout.dimensions.y;
    }

    /// Add some space in between two boxes.
    pub fn add_space(&mut self, space: Size) {
        if self.overflows(self.dimensions + Size2D::with_y(space)) {
            panic!("box layouter: would overflow in add_space");
        }

        self.cursor.y += space;
        self.dimensions.y += space;
    }

    /// Add a sublayout at an absolute position.
    pub fn add_box_absolute(&mut self, position: Size2D, layout: BoxLayout) {
        self.actions.push(TextAction::MoveAbsolute(position));
        self.actions.extend(layout.actions);
    }

    /// Whether this layouter contains any items.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// The remaining space for new boxes.
    pub fn remaining(&self) -> Size2D {
        Size2D {
            x: self.usable.x,
            y: self.usable.y - self.dimensions.y,
        }
    }

    /// Finish the layouting and create a box layout from this.
    pub fn finish(self) -> BoxLayout {
        BoxLayout {
            dimensions: if self.ctx.space.shrink_to_fit {
                self.dimensions.padded(self.ctx.space.padding)
            } else {
                self.ctx.space.dimensions
            },
            actions: self.actions,
        }
    }

    /// Whether the given box is bigger than what we can hold.
    fn overflows(&self, dimensions: Size2D) -> bool {
        dimensions.x > self.usable.x || dimensions.y > self.usable.y
    }
}
