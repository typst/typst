//! Definitive layouting of boxes.

use crate::doc::{Document, Page, TextAction};
use crate::font::Font;
use super::{Layouter, LayoutContext, Size2D};


/// A box layout has a fixed width and height and consists of actions.
#[derive(Debug, Clone)]
pub struct BoxLayout {
    /// The size of the box.
    dimensions: Size2D,
    /// The actions composing this layout.
    actions: Vec<TextAction>,
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

/// Layouts boxes block-style.
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
    pub fn add_box(&mut self, layout: BoxLayout) {
        unimplemented!()
    }

    /// Add a sublayout at an absolute position.
    pub fn add_box_absolute(&mut self, position: Size2D, layout: BoxLayout) {
        self.actions.push(TextAction::MoveAbsolute(position));
        self.actions.extend(layout.actions);
    }
}

impl Layouter for BoxLayouter<'_, '_> {
    type Layout = BoxLayout;

    /// Finish the layouting and create a box layout from this.
    fn finish(self) -> BoxLayout {
        BoxLayout {
            dimensions: self.ctx.space.dimensions.clone(),
            actions: self.actions
        }
    }

    fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}
