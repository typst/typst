use crate::library::prelude::*;
use crate::library::text::{HorizontalAlign, ParNode};

/// Align a node along the layouting axes.
#[derive(Debug, Hash)]
pub struct AlignNode {
    /// How to align the node horizontally and vertically.
    pub aligns: Axes<Option<RawAlign>>,
    /// The node to be aligned.
    pub child: LayoutNode,
}

#[node]
impl AlignNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        let aligns: Axes<Option<RawAlign>> = args.find()?.unwrap_or_default();
        let body: Content = args.expect("body")?;
        Ok(match (body, aligns) {
            (Content::Block(node), _) => Content::Block(node.aligned(aligns)),
            (other, Axes { x: Some(x), y: None }) => {
                other.styled(ParNode::ALIGN, HorizontalAlign(x))
            }
            (other, _) => Content::Block(other.pack().aligned(aligns)),
        })
    }
}

impl Layout for AlignNode {
    fn layout(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        // The child only needs to expand along an axis if there's no alignment.
        let mut pod = regions.clone();
        pod.expand &= self.aligns.as_ref().map(Option::is_none);

        // Align paragraphs inside the child.
        let mut passed = StyleMap::new();
        if let Some(align) = self.aligns.x {
            passed.set(ParNode::ALIGN, HorizontalAlign(align));
        }

        // Layout the child.
        let mut frames = self.child.layout(world, &pod, passed.chain(&styles))?;
        for (region, frame) in regions.iter().zip(&mut frames) {
            // Align in the target size. The target size depends on whether we
            // should expand.
            let target = regions.expand.select(region, frame.size());
            let aligns = self
                .aligns
                .map(|align| align.resolve(styles))
                .unwrap_or(Axes::new(Align::Left, Align::Top));

            frame.resize(target, aligns);
        }

        Ok(frames)
    }
}
