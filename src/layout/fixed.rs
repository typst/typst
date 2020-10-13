use super::*;
use crate::geom::Linear;

/// A node that can fix its child's width and height.
#[derive(Debug, Clone, PartialEq)]
pub struct Fixed {
    /// The fixed width, if any.
    pub width: Option<Linear>,
    /// The fixed height, if any.
    pub height: Option<Linear>,
    /// The child node whose size to fix.
    pub child: LayoutNode,
}

impl Layout for Fixed {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
        let Area { rem, full } = areas.current;
        let size = Size::new(
            self.width.map(|w| w.resolve(full.width)).unwrap_or(rem.width),
            self.height.map(|h| h.resolve(full.height)).unwrap_or(rem.height),
        );

        let areas = Areas::once(size);
        self.child.layout(ctx, &areas)
    }
}

impl From<Fixed> for LayoutNode {
    fn from(fixed: Fixed) -> Self {
        Self::dynamic(fixed)
    }
}
