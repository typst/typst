//! Flexible and lazy layouting of boxes.

use crate::doc::TextAction;
use crate::size::Size2D;
use super::{BoxLayout, ActionList, LayoutSpace, LayoutResult, LayoutError};


/// A flex layout consists of a yet unarranged list of boxes.
#[derive(Debug, Clone)]
pub struct FlexLayout {
    /// The sublayouts composing this layout.
    pub units: Vec<FlexUnit>,
    /// The layout space to arrange in.
    pub ctx: FlexContext,
}

/// A unit in a flex layout.
#[derive(Debug, Clone)]
pub enum FlexUnit {
    /// A content unit to be arranged flexibly.
    Boxed(BoxLayout),
    /// A unit which acts as glue between two [`FlexUnit::Boxed`] units and
    /// is only present if there was no flow break in between the two surrounding boxes.
    Glue(BoxLayout),
}

impl FlexLayout {
    /// Create a new flex layouter.
    pub fn new(ctx: FlexContext) -> FlexLayout {
        FlexLayout {
            ctx,
            units: vec![],
        }
    }

    /// Add a sublayout.
    pub fn add_box(&mut self, layout: BoxLayout) {
        self.units.push(FlexUnit::Boxed(layout));
    }

    /// Add a glue layout which can be replaced by a line break.
    pub fn add_glue(&mut self, glue: BoxLayout) {
        self.units.push(FlexUnit::Glue(glue));
    }

    /// Add all sublayouts of another flex layout.
    pub fn add_flexible(&mut self, layout: FlexLayout) {
        self.units.extend(layout.units);
    }

    /// Whether this layouter contains any items.
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// Compute the justified layout.
    pub fn into_box(self) -> LayoutResult<BoxLayout> {
        FlexFinisher::new(self).finish()
    }
}

/// The context for flex layouting.
#[derive(Debug, Copy, Clone)]
pub struct FlexContext {
    /// The space to layout the boxes in.
    pub space: LayoutSpace,
    /// The flex spacing (like line spacing).
    pub flex_spacing: f32,
}

/// Finishes a flex layout by justifying the positions of the individual boxes.
#[derive(Debug)]
struct FlexFinisher {
    units: Vec<FlexUnit>,
    ctx: FlexContext,
    actions: ActionList,
    dimensions: Size2D,
    usable: Size2D,
    cursor: Size2D,
    line: Size2D,
}

impl FlexFinisher {
    /// Create the finisher from the layout.
    fn new(layout: FlexLayout) -> FlexFinisher {
        let space = layout.ctx.space;
        FlexFinisher {
            units: layout.units,
            ctx: layout.ctx,
            actions: ActionList::new(),
            dimensions: Size2D::zero(),
            usable: space.usable(),
            cursor: Size2D::new(space.padding.left, space.padding.top),
            line: Size2D::zero(),
        }
    }

    /// Finish the flex layout into the justified box layout.
    fn finish(mut self) -> LayoutResult<BoxLayout> {
        // Move the units out of the layout.
        let units = self.units;
        self.units = vec![];

        // Arrange the units.
        for unit in units {
            match unit {
                FlexUnit::Boxed(boxed) => self.boxed(boxed)?,
                FlexUnit::Glue(glue) => self.glue(glue),
            }
        }

        // Flush everything to get the correct dimensions.
        self.newline();

        Ok(BoxLayout {
            dimensions: if self.ctx.space.shrink_to_fit {
                self.dimensions.padded(self.ctx.space.padding)
            } else {
                self.ctx.space.dimensions
            },
            actions: self.actions.into_vec(),
        })
    }

    /// Layout the box.
    fn boxed(&mut self, boxed: BoxLayout) -> LayoutResult<()> {
        // Move to the next line if necessary.
        if self.line.x + boxed.dimensions.x > self.usable.x {
            // If it still does not fit, we stand no chance.
            if boxed.dimensions.x > self.usable.x {
                return Err(LayoutError::NotEnoughSpace);
            }

            self.newline();
        }

        self.append(boxed);

        Ok(())
    }

    /// Layout the glue.
    fn glue(&mut self, glue: BoxLayout) {
        // Only add the glue if it fits on the line, otherwise move to the next line.
        if self.line.x + glue.dimensions.x > self.usable.x {
            self.newline();
        } else {
            self.append(glue);
        }
    }

    /// Append a box to the layout without checking anything.
    fn append(&mut self, layout: BoxLayout) {
        // Move all actions into this layout and translate absolute positions.
        self.actions.reset_origin();
        self.actions.add(TextAction::MoveAbsolute(self.cursor));
        self.actions.set_origin(self.cursor);
        self.actions.extend(layout.actions);

        // Adjust the sizes.
        self.line.x += layout.dimensions.x;
        self.line.y = crate::size::max(self.line.y, layout.dimensions.y);
        self.cursor.x += layout.dimensions.x;
    }

    /// Move to the next line.
    fn newline(&mut self) {
        self.line.y *= self.ctx.flex_spacing;
        self.dimensions.x = crate::size::max(self.dimensions.x, self.line.x);
        self.dimensions.y += self.line.y;
        self.cursor.x = self.ctx.space.padding.left;
        self.cursor.y += self.line.y;
        self.line = Size2D::zero();
    }
}
