use crate::library::prelude::*;

/// Hide a node without affecting layout.
#[derive(Debug, Hash)]
pub struct HideNode(pub Content);

#[node(Layout)]
impl HideNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Layout for HideNode {
    fn layout(
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

    fn level(&self) -> Level {
        Level::Inline
    }
}
