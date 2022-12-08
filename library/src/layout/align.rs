use super::{HorizontalAlign, ParNode};
use crate::prelude::*;

/// Align content along the layouting axes.
#[derive(Debug, Hash)]
pub struct AlignNode {
    /// How to align the content horizontally and vertically.
    pub aligns: Axes<Option<GenAlign>>,
    /// The content to be aligned.
    pub child: Content,
}

#[node(Layout)]
impl AlignNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let aligns: Axes<Option<GenAlign>> = args.find()?.unwrap_or_default();
        let body: Content = args.expect("body")?;

        if let Axes { x: Some(x), y: None } = aligns {
            if !body.has::<dyn Layout>() || body.has::<dyn Inline>() {
                return Ok(body.styled(ParNode::ALIGN, HorizontalAlign(x)));
            }
        }

        Ok(body.aligned(aligns))
    }
}

impl Layout for AlignNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // The child only needs to expand along an axis if there's no alignment.
        let mut pod = regions.clone();
        pod.expand &= self.aligns.as_ref().map(Option::is_none);

        // Align paragraphs inside the child.
        let mut map = StyleMap::new();
        if let Some(align) = self.aligns.x {
            map.set(ParNode::ALIGN, HorizontalAlign(align));
        }

        // Layout the child.
        let mut fragment = self.child.layout(vt, styles.chain(&map), pod)?;
        for (region, frame) in regions.iter().zip(&mut fragment) {
            // Align in the target size. The target size depends on whether we
            // should expand.
            let target = regions.expand.select(region, frame.size());
            let aligns = self
                .aligns
                .map(|align| align.resolve(styles))
                .unwrap_or(Axes::new(Align::Left, Align::Top));

            frame.resize(target, aligns);
        }

        Ok(fragment)
    }
}
