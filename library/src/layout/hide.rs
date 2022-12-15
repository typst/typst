use crate::prelude::*;

/// Hide content without affecting layout.
///
/// Tags: layout.
#[func]
#[capable(Layout, Inline)]
#[derive(Debug, Hash)]
pub struct HideNode(pub Content);

#[node]
impl HideNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Layout for HideNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut fragment = self.0.layout(vt, styles, regions)?;
        for frame in &mut fragment {
            frame.clear();
        }
        Ok(fragment)
    }
}

impl Inline for HideNode {}
