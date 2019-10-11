//! Flexible and lazy layouting of boxes.

use crate::size::{Size, Size2D};
use super::{BoxLayout, ActionList, LayoutSpace, Alignment, LayoutResult, LayoutError};


/// A flex layout consists of a yet unarranged list of boxes.
#[derive(Debug, Clone)]
pub struct FlexLayout {
    /// The sublayouts composing this layout.
    pub units: Vec<FlexUnit>,
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
    /// Create a new flex layout.
    pub fn new() -> FlexLayout {
        FlexLayout {
            units: vec![],
        }
    }

    /// Create a new flex layout containing just one box.
    pub fn from_box(boxed: BoxLayout) -> FlexLayout {
        FlexLayout {
            units: vec![FlexUnit::Boxed(boxed)],
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
    pub fn finish(self, ctx: FlexContext) -> LayoutResult<BoxLayout> {
        FlexFinisher::new(self, ctx).finish()
    }
}

/// The context for flex layouting.
#[derive(Debug, Copy, Clone)]
pub struct FlexContext {
    /// The space to layout the boxes in.
    pub space: LayoutSpace,
    /// The flex spacing between two lines of boxes.
    pub flex_spacing: Size,
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
    line_metrics: Size2D,
    line_content: Vec<(Size2D, BoxLayout)>,
    glue: Option<BoxLayout>,
}

impl FlexFinisher {
    /// Create the finisher from the layout.
    fn new(layout: FlexLayout, ctx: FlexContext) -> FlexFinisher {
        let space = ctx.space;
        FlexFinisher {
            units: layout.units,
            ctx,
            actions: ActionList::new(),
            dimensions: match ctx.space.alignment {
                Alignment::Left => Size2D::zero(),
                Alignment::Right => Size2D::with_x(space.usable().x),
            },
            usable: space.usable(),
            cursor: Size2D::new(space.padding.left, space.padding.top),
            line_metrics: Size2D::zero(),
            line_content: vec![],
            glue: None,
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
        let last_glue_x = self.glue.as_ref()
            .map(|g| g.dimensions.x)
            .unwrap_or(Size::zero());

        // Move to the next line if necessary.
        if self.line_metrics.x + boxed.dimensions.x + last_glue_x > self.usable.x {
            // If it still does not fit, we stand no chance.
            if boxed.dimensions.x > self.usable.x {
                return Err(LayoutError::NotEnoughSpace);
            }

            self.newline();
        } else if let Some(glue) = self.glue.take() {
            self.append(glue);
        }

        self.append(boxed);

        Ok(())
    }

    /// Layout the glue.
    fn glue(&mut self, glue: BoxLayout) {
        if let Some(glue) = self.glue.take() {
            self.append(glue);
        }
        self.glue = Some(glue);
    }

    /// Append a box to the layout without checking anything.
    fn append(&mut self, layout: BoxLayout) {
        let dim = layout.dimensions;
        self.line_content.push((self.cursor, layout));

        self.line_metrics.x += dim.x;
        self.line_metrics.y = crate::size::max(self.line_metrics.y, dim.y);
        self.cursor.x += dim.x;
    }

    /// Move to the next line.
    fn newline(&mut self) {
        // Move all actions into this layout and translate absolute positions.
        let remaining_space = Size2D::with_x(self.ctx.space.usable().x - self.line_metrics.x);
        for (cursor, layout) in self.line_content.drain(..) {
            let position = match self.ctx.space.alignment {
                Alignment::Left => cursor,
                Alignment::Right => {
                    // Right align everything by shifting it right by the
                    // amount of space left to the right of the line.
                    cursor + remaining_space
                },
            };

            self.actions.add_box_absolute(position, layout);
        }

        // Stretch the dimensions to at least the line width.
        self.dimensions.x = crate::size::max(self.dimensions.x, self.line_metrics.x);

        // If we wrote a line previously add the inter-line spacing.
        if self.dimensions.y > Size::zero() {
            self.dimensions.y += self.ctx.flex_spacing;
        }

        self.dimensions.y += self.line_metrics.y;

        // Reset the cursor the left and move down by the line and the inter-line spacing.
        self.cursor.x = self.ctx.space.padding.left;
        self.cursor.y += self.line_metrics.y + self.ctx.flex_spacing;

        // Reset the current line metrics.
        self.line_metrics = Size2D::zero();
    }
}
