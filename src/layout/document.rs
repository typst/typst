use super::*;

/// The top-level layout node.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub runs: Vec<Pages>,
}

impl Document {
    /// Layout the document.
    pub async fn layout(&self, ctx: &mut LayoutContext) -> Vec<BoxLayout> {
        let mut layouts = vec![];
        for run in &self.runs {
            layouts.extend(run.layout(ctx).await);
        }
        layouts
    }
}

/// A variable-length run of pages that all have the same properties.
#[derive(Debug, Clone, PartialEq)]
pub struct Pages {
    /// The size of the pages.
    pub size: Size,
    /// The layout node that produces the actual pages.
    pub child: LayoutNode,
}

impl Pages {
    /// Layout the page run.
    pub async fn layout(&self, ctx: &mut LayoutContext) -> Vec<BoxLayout> {
        let constraints = LayoutConstraints {
            spaces: vec![LayoutSpace { base: self.size, size: self.size }],
            repeat: true,
        };

        self.child
            .layout(ctx, constraints)
            .await
            .into_iter()
            .filter_map(|item| match item {
                Layouted::Spacing(_) => None,
                Layouted::Box(layout, _) => Some(layout),
            })
            .collect()
    }
}
