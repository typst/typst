//! Document-structuring section headings.

use super::prelude::*;
use super::{FontFamily, TextNode};

/// A section heading.
#[derive(Debug, Hash)]
pub struct HeadingNode {
    /// The logical nesting depth of the section, starting from one. In the
    /// default style, this controls the text size of the heading.
    pub level: usize,
    /// The node that produces the heading's contents.
    pub child: PackedNode,
}

#[class]
impl HeadingNode {
    /// The heading's font family.
    pub const FAMILY: Smart<FontFamily> = Smart::Auto;
    /// The fill color of text in the heading. Just the surrounding text color
    /// if `auto`.
    pub const FILL: Smart<Paint> = Smart::Auto;

    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        Ok(Node::block(Self {
            child: args.expect("body")?,
            level: args.named("level")?.unwrap_or(1),
        }))
    }

    fn set(args: &mut Args, styles: &mut StyleMap) -> TypResult<()> {
        styles.set_opt(Self::FAMILY, args.named("family")?);
        styles.set_opt(Self::FILL, args.named("fill")?);
        Ok(())
    }
}

impl Layout for HeadingNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let upscale = (1.6 - 0.1 * self.level as f64).max(0.75);

        let mut passed = StyleMap::new();
        passed.set(TextNode::STRONG, true);
        passed.set(TextNode::SIZE, Relative::new(upscale).into());

        if let Smart::Custom(family) = styles.get_ref(Self::FAMILY) {
            passed.set(
                TextNode::FAMILY_LIST,
                std::iter::once(family)
                    .chain(styles.get_ref(TextNode::FAMILY_LIST))
                    .cloned()
                    .collect(),
            );
        }

        if let Smart::Custom(fill) = styles.get(Self::FILL) {
            passed.set(TextNode::FILL, fill);
        }

        self.child.layout(ctx, regions, passed.chain(&styles))
    }
}
