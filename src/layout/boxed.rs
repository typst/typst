//! Block-style layouting of boxes.

use std::io::{self, Write};
use crate::doc::{Document, Page};
use crate::size::{Size, Size2D};
use super::*;


/// A box layout has a fixed width and height and composes of actions.
#[derive(Debug, Clone)]
pub struct BoxLayout {
    /// The size of the box.
    pub dimensions: Size2D,
    /// The actions composing this layout.
    pub actions: Vec<LayoutAction>,
    /// Whether to debug-render this box.
    pub debug_render: bool,
}

impl BoxLayout {
    /// Convert this layout into a document.
    pub fn into_doc(self) -> Document {
        Document {
            pages: vec![Page {
                width: self.dimensions.x,
                height: self.dimensions.y,
                actions: self.actions,
            }],
        }
    }

    /// Serialize this layout into a string representation.
    pub fn serialize<W: Write>(&self, f: &mut W) -> io::Result<()> {
        writeln!(f, "{:.4} {:.4}", self.dimensions.x.to_pt(), self.dimensions.y.to_pt())?;
        for action in &self.actions {
            action.serialize(f)?;
            writeln!(f)?;
        }
        Ok(())
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
    pub ctx: BoxContext,
    actions: LayoutActionList,
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
            actions: LayoutActionList::new(),
            dimensions: match ctx.space.alignment {
                Alignment::Left => Size2D::zero(),
                Alignment::Right => Size2D::with_x(space.usable().x),
            },
            usable: space.usable(),
            cursor: Size2D::new(match ctx.space.alignment {
                Alignment::Left => space.padding.left,
                Alignment::Right => space.dimensions.x - space.padding.right,
            }, space.padding.top),
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

        // Apply the dimensions if they fit.
        self.dimensions = new;
        let width = layout.dimensions.x;
        let height = layout.dimensions.y;

        let position = match self.ctx.space.alignment {
            Alignment::Left => self.cursor,
            Alignment::Right => self.cursor - Size2D::with_x(width),
        };

        // Add the box.
        self.add_box_absolute(position, layout);

        // Adjust the cursor.
        self.cursor.y += height;

        Ok(())
    }

    /// Add a sublayout at an absolute position.
    pub fn add_box_absolute(&mut self, position: Size2D, layout: BoxLayout) {
        self.actions.add_box(position, layout);
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
            debug_render: true,
        }
    }

    /// Whether the given box is bigger than what we can hold.
    fn overflows(&self, dimensions: Size2D) -> bool {
        dimensions.x > self.usable.x || dimensions.y > self.usable.y
    }
}
