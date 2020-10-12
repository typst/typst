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
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
        let areas = shrink_areas(areas, self.padding);

        let mut layouted = self.child.layout(ctx, &areas);
        match &mut layouted {
            Layouted::Spacing(_) => {}
            Layouted::Boxed(boxed, _) => pad_box(boxed, self.padding),
            Layouted::Boxes(boxes) => {
                for (boxed, _) in boxes {
                    pad_box(boxed, self.padding);
                }
            }
        }

        layouted
    }
}

/// Shrink all areas by the padding.
fn shrink_areas(areas: &Areas, padding: Sides<Linear>) -> Areas {
    let shrink = |size| size - padding.eval(size).size();
    Areas {
        current: Area {
            rem: shrink(areas.current.rem),
            full: shrink(areas.current.full),
        },
        backlog: areas.backlog.iter().copied().map(shrink).collect(),
        last: areas.last.map(shrink),
    }
}

/// Enlarge the box and move all elements inwards.
fn pad_box(boxed: &mut BoxLayout, padding: Sides<Linear>) {
    let padding = padding.eval(boxed.size);
    let origin = Point::new(padding.left, padding.top);

    boxed.size += padding.size();
    for (point, _) in &mut boxed.elements {
        *point += origin;
    }
}

impl From<Pad> for LayoutNode {
    fn from(pad: Pad) -> Self {
        Self::dynamic(pad)
    }
}
