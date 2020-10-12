use super::*;
use crate::geom::Linear;

/// A node that pads its child at the sides.
#[derive(Debug, Clone, PartialEq)]
pub struct Pad {
    /// The amount of padding.
    pub padding: Sides<Linear>,
    /// The child node whose sides to pad.
    pub child: LayoutNode,
}

impl Layout for Pad {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Layouted> {
        let shrink = |size| size - self.padding.eval(size).size();
        let areas = Areas {
            current: Area {
                rem: shrink(areas.current.rem),
                full: shrink(areas.current.full),
            },
            backlog: areas.backlog.iter().copied().map(shrink).collect(),
            last: areas.last.map(shrink),
        };

        let mut layouted = self.child.layout(ctx, &areas);

        for item in &mut layouted {
            if let Layouted::Boxed(boxed, _) = item {
                let padding = self.padding.eval(boxed.size);
                let origin = Point::new(padding.left, padding.top);

                boxed.size += padding.size();
                for (point, _) in &mut boxed.elements {
                    *point += origin;
                }
            }
        }

        layouted
    }
}

impl From<Pad> for LayoutNode {
    fn from(pad: Pad) -> Self {
        Self::dynamic(pad)
    }
}
