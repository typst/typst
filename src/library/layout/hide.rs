use crate::library::prelude::*;

/// Hide a node without affecting layout.
#[derive(Debug, Hash)]
pub struct HideNode(pub LayoutNode);

#[class]
impl HideNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Template> {
        Ok(Template::inline(Self(args.expect("body")?)))
    }
}

impl Layout for HideNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let mut frames = self.0.layout(ctx, regions, styles)?;

        // Clear the frames.
        for frame in &mut frames {
            *frame = Arc::new(Frame { elements: vec![], ..**frame });
        }

        Ok(frames)
    }
}
