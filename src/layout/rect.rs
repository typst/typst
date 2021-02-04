use std::cmp::Ordering;

use super::*;
use crate::{color::RgbaColor, geom::Linear};

/// A node that represents a rectangular box.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeRect {
    /// The fixed width, if any.
    pub width: Option<Linear>,
    /// The fixed height, if any.
    pub height: Option<Linear>,
    /// The background color.
    pub color: Option<Color>,
    /// The child node whose sides to pad.
    pub child: Node,
}

impl Layout for NodeRect {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
        let Areas { current, full, .. } = areas;

        let height_opt = self.height.map(|h| h.resolve(full.height));
        let mut size = Size::new(
            self.width.map(|w| w.resolve(full.width)).unwrap_or(current.width),
            height_opt.unwrap_or(current.height),
        );

        let areas = Areas::once(size);
        let mut layouted = self.child.layout(ctx, &areas);

        // If the children have some height, apply that,
        // otherwise fall back to zero or the height property.
        if let Some(max) = layouted
            .frames()
            .iter()
            .map(|f| f.size.height)
            .max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
        {
            size.height = max;
        } else {
            size.height = height_opt.unwrap_or(Length::ZERO)
        }

        if let Some(first) = layouted.frames_mut().first_mut() {
            first.elements.insert(
                0,
                (
                    Point::ZERO,
                    Element::Geometry(Geometry {
                        shape: Shape::Rect(Rect { size }),
                        fill: Fill::Color(self.color.unwrap_or(Color::Rgba(RgbaColor {
                            r: 255,
                            g: 255,
                            b: 255,
                            a: 0,
                        }))),
                    }),
                ),
            )
        }

        layouted
    }
}

impl From<NodeRect> for NodeAny {
    fn from(pad: NodeRect) -> Self {
        Self::new(pad)
    }
}
