use super::*;
use crate::geom::Linear;

/// A node that can fix its child's width and height.
#[derive(Debug, Clone, PartialEq)]
pub struct Fixed {
    pub width: Option<Linear>,
    pub height: Option<Linear>,
    pub child: LayoutNode,
}

#[async_trait(?Send)]
impl Layout for Fixed {
    async fn layout(
        &self,
        ctx: &mut LayoutContext,
        constraints: LayoutConstraints,
    ) -> Vec<LayoutItem> {
        let space = constraints.spaces[0];
        let size = Size::new(
            self.width
                .map(|w| w.eval(space.base.width))
                .unwrap_or(space.size.width),
            self.height
                .map(|h| h.eval(space.base.height))
                .unwrap_or(space.size.height),
        );

        self.child
            .layout(ctx, LayoutConstraints {
                spaces: vec![LayoutSpace { base: size, size }],
                repeat: false,
            })
            .await
    }
}

impl From<Fixed> for LayoutNode {
    fn from(fixed: Fixed) -> Self {
        Self::dynamic(fixed)
    }
}
