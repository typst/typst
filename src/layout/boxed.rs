//! Block-style layouting of boxes.

use crate::doc::{Document, Page, LayoutAction};
use crate::font::Font;
use crate::size::{Size, Size2D};
use super::{ActionList, LayoutSpace, LayoutResult, LayoutError};


/// A box layout has a fixed width and height and composes of actions.
#[derive(Debug, Clone)]
pub struct BoxLayout {
    /// The size of the box.
    pub dimensions: Size2D,
    /// The actions composing this layout.
    pub actions: Vec<LayoutAction>,
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
    actions: ActionList,
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
            actions: ActionList::new(),
            dimensions: Size2D::zero(),
            usable: space.usable(),
            cursor: Size2D::new(space.padding.left, space.padding.right),
        }
    }

    /// Add a sublayout.
    pub fn add_box(&mut self, layout: BoxLayout) -> LayoutResult<()> {
        // In the flow direction (vertical) add the layout and in the second
        // direction just consider the maximal size of any child layout.
        let new = Size2D {
            x: crate::size::max(self.dimensions.x, layout.dimensions.x),
            y: self.dimensions.y + layout.dimensions.y,
        };

        // Check whether this box fits.
        if self.overflows(new) {
            return Err(LayoutError::NotEnoughSpace);
        }

        // Apply the dimensions as they fit.
        let height = layout.dimensions.y;
        self.dimensions = new;

        // Add the box.
        self.add_box_absolute(self.cursor, layout);

        // Adjust the cursor.
        self.cursor.y += height;

        Ok(())
    }

    /// Add a sublayout at an absolute position.
    pub fn add_box_absolute(&mut self, position: Size2D, layout: BoxLayout) {
        // Move all actions into this layout and translate absolute positions.
        self.actions.reset_origin();
        self.actions.add(LayoutAction::MoveAbsolute(position));
        self.actions.set_origin(position);
        self.actions.extend(layout.actions);
    }

    /// Add some space in between two boxes.
    pub fn add_space(&mut self, space: Size) -> LayoutResult<()> {
        // Check whether this space fits.
        if self.overflows(self.dimensions + Size2D::with_y(space)) {
            return Err(LayoutError::NotEnoughSpace);
        }

        // Adjust the sizes.
        self.cursor.y += space;
        self.dimensions.y += space;

        Ok(())
    }

    /// The remaining space for new boxes.
    pub fn remaining(&self) -> Size2D {
        Size2D {
            x: self.usable.x,
            y: self.usable.y - self.dimensions.y,
        }
    }

    /// Whether this layouter contains any items.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Finish the layouting and create a box layout from this.
    pub fn finish(self) -> BoxLayout {
        BoxLayout {
            dimensions: if self.ctx.space.shrink_to_fit {
                self.dimensions.padded(self.ctx.space.padding)
            } else {
                self.ctx.space.dimensions
            },
            actions: self.actions.into_vec(),
        }
    }

    /// Whether the given box is bigger than what we can hold.
    fn overflows(&self, dimensions: Size2D) -> bool {
        dimensions.x > self.usable.x || dimensions.y > self.usable.y
    }
}
