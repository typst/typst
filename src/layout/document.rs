use super::*;

/// The top-level layout node.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The runs of pages with same properties.
    pub runs: Vec<Pages>,
}

impl Document {
    /// Layout the document.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<BoxLayout> {
        let mut layouts = vec![];
        for run in &self.runs {
            layouts.extend(run.layout(ctx));
        }
        layouts
    }
}

/// A variable-length run of pages that all have the same properties.
#[derive(Debug, Clone, PartialEq)]
pub struct Pages {
    /// The size of the pages.
    pub size: Size,
    /// The layout node that produces the actual pages (typically a [stack]).
    ///
    /// [stack]: struct.Stack.html
    pub child: LayoutNode,
}

impl Pages {
    /// Layout the page run.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<BoxLayout> {
        let areas = Areas::repeat(self.size);
        let layouted = self.child.layout(ctx, &areas);
        layouted.into_boxes()
    }
}
