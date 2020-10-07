use super::*;
use crate::geom::Linear;

/// A node that pads its child at the sides.
#[derive(Debug, Clone, PartialEq)]
pub struct Pad {
    pub padding: Sides<Linear>,
    pub child: LayoutNode,
}

#[async_trait(?Send)]
impl Layout for Pad {
    async fn layout(
        &self,
        ctx: &mut LayoutContext,
        constraints: LayoutConstraints,
    ) -> Vec<LayoutItem> {
        self.child
            .layout(ctx, LayoutConstraints {
                spaces: constraints
                    .spaces
                    .into_iter()
                    .map(|space| LayoutSpace {
                        base: space.base + self.padding.insets(space.base).size(),
                        size: space.size + self.padding.insets(space.size).size(),
                    })
                    .collect(),
                repeat: constraints.repeat,
            })
            .await
            .into_iter()
            .map(|item| match item {
                LayoutItem::Box(boxed, align) => {
                    let padding = self.padding.insets(boxed.size);
                    let padded = boxed.size - padding.size();

                    let mut outer = BoxLayout::new(padded);
                    let start = Point::new(-padding.x0, -padding.y0);
                    outer.push_layout(start, boxed);

                    LayoutItem::Box(outer, align)
                }
                item => item,
            })
            .collect()
    }
}

impl From<Pad> for LayoutNode {
    fn from(pad: Pad) -> Self {
        Self::dynamic(pad)
    }
}
