use crate::prelude::*;

/// Repeats content to fill a line.
#[derive(Debug, Hash)]
pub struct RepeatNode(pub Content);

#[node(Layout, Inline)]
impl RepeatNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Layout for RepeatNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        self.0.layout(vt, styles, regions)
    }
}

impl Inline for RepeatNode {}
