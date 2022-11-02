use crate::library::prelude::*;

/// Hide content without affecting layout.
#[derive(Debug, Hash)]
pub struct HideNode(pub Content);

#[node(LayoutInline)]
impl HideNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl LayoutInline for HideNode {
    fn layout_inline(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let mut frames = self.0.layout_inline(world, regions, styles)?;
        for frame in &mut frames {
            frame.clear();
        }
        Ok(frames)
    }
}
