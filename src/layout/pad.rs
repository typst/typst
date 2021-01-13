use super::*;
use crate::geom::Linear;

/// A node that adds padding to its child.
#[derive(Debug, Clone, PartialEq)]
pub struct NodePad {
    /// The amount of padding.
    pub padding: Sides<Linear>,
    /// The child node whose sides to pad.
    pub child: Node,
}

impl Layout for NodePad {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
        let areas = shrink(areas, self.padding);

        let mut layouted = self.child.layout(ctx, &areas);
        match &mut layouted {
            Layouted::Spacing(_) => {}
            Layouted::Frame(frame, _) => pad(frame, self.padding),
            Layouted::Frames(frames, _) => {
                for frame in frames {
                    pad(frame, self.padding);
                }
            }
        }

        layouted
    }
}

impl From<NodePad> for NodeAny {
    fn from(pad: NodePad) -> Self {
        Self::new(pad)
    }
}

/// Shrink all areas by the padding.
fn shrink(areas: &Areas, padding: Sides<Linear>) -> Areas {
    let shrink = |size| size - padding.resolve(size).size();
    Areas {
        current: shrink(areas.current),
        full: shrink(areas.full),
        backlog: areas.backlog.iter().copied().map(shrink).collect(),
        last: areas.last.map(shrink),
    }
}

/// Enlarge the box and move all elements inwards.
fn pad(frame: &mut Frame, padding: Sides<Linear>) {
    let padding = padding.resolve(frame.size);
    let origin = Point::new(padding.left, padding.top);

    frame.size += padding.size();
    for (point, _) in &mut frame.elements {
        *point += origin;
    }
}
