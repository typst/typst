//! Flexible and lazy layouting of boxes.

use super::{Layouter, LayoutContext, BoxLayout};


/// A flex layout consists of a yet unarranged list of boxes.
#[derive(Debug, Clone)]
pub struct FlexLayout {
    /// The sublayouts composing this layout.
    layouts: Vec<BoxLayout>,
}

impl FlexLayout {
    /// Compute the layout.
    pub fn into_box(self) -> BoxLayout {
        // TODO: Do the justification.
        unimplemented!()
    }
}

/// Layouts boxes next to each other (inline-style) lazily.
#[derive(Debug)]
pub struct FlexLayouter<'a, 'p> {
    ctx: &'a LayoutContext<'a, 'p>,
    layouts: Vec<BoxLayout>,
}

impl<'a, 'p> FlexLayouter<'a, 'p> {
    /// Create a new flex layouter.
    pub fn new(ctx: &'a LayoutContext<'a, 'p>) -> FlexLayouter<'a, 'p> {
        FlexLayouter {
            ctx,
            layouts: vec![],
        }
    }

    /// Add a sublayout.
    pub fn add_box(&mut self, layout: BoxLayout) {
        self.layouts.push(layout);
    }

    /// Add all sublayouts of another flex layout.
    pub fn add_flexible(&mut self, layout: FlexLayout) {
        self.layouts.extend(layout.layouts);
    }
}

impl Layouter for FlexLayouter<'_, '_> {
    type Layout = FlexLayout;

    /// Finish the layouting and create a flexible layout from this.
    fn finish(self) -> FlexLayout {
        FlexLayout {
            layouts: self.layouts
        }
    }

    fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }
}
