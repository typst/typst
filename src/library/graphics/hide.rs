use crate::library::prelude::*;

/// Hide a node without affecting layout.
#[derive(Debug, Hash)]
pub struct HideNode(pub LayoutNode);

#[node]
impl HideNode {
    fn construct(_: &mut Machine, args: &mut Args) -> TypResult<Content> {
        Ok(Content::inline(Self(args.expect("body")?)))
    }
}

impl Layout for HideNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Frame>> {
        let mut frames = self.0.layout(ctx, regions, styles)?;

        // Clear the frames.
        for frame in &mut frames {
            frame.clear();
        }

        Ok(frames)
    }
}
