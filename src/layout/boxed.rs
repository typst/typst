//! Layouting of layout boxes.

use crate::doc::TextAction;
use super::{Layouter, Layout, LayoutContext, LayoutResult, Position};


/// Layouts sublayouts within the constraints of a layouting context.
#[derive(Debug)]
pub struct BoxLayouter<'a, 'p> {
    ctx: &'a LayoutContext<'a, 'p>,
    actions: Vec<TextAction>,
}

impl<'a, 'p> BoxLayouter<'a, 'p> {
    /// Create a new box layouter.
    pub fn new(ctx: &'a LayoutContext<'a, 'p>) -> BoxLayouter<'a, 'p> {
        BoxLayouter {
            ctx,
            actions: vec![],
        }
    }

    /// Add a sublayout.
    pub fn add_layout_absolute(&mut self, position: Position, layout: Layout) {
        self.actions.push(TextAction::MoveAbsolute(position));
        self.actions.extend(layout.actions);
    }
}

impl Layouter for BoxLayouter<'_, '_> {
    fn finish(self) -> LayoutResult<Layout> {
        Ok(Layout {
            extent: self.ctx.max_extent.clone(),
            actions: self.actions
        })
    }
}
