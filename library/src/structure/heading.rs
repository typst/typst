use typst::font::FontWeight;

use crate::layout::{BlockNode, VNode};
use crate::prelude::*;
use crate::text::{TextNode, TextSize};

/// A section heading.
#[derive(Debug, Hash)]
pub struct HeadingNode {
    /// The logical nesting depth of the section, starting from one. In the
    /// default style, this controls the text size of the heading.
    pub level: NonZeroUsize,
    /// The heading's contents.
    pub body: Content,
}

#[node(Show, Finalize)]
impl HeadingNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self {
            body: args.expect("body")?,
            level: args.named("level")?.unwrap_or(NonZeroUsize::new(1).unwrap()),
        }
        .pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "level" => Some(Value::Int(self.level.get() as i64)),
            "body" => Some(Value::Content(self.body.clone())),
            _ => None,
        }
    }
}

impl Show for HeadingNode {
    fn show(&self, _: Tracked<dyn World>, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockNode(self.body.clone()).pack())
    }
}

impl Finalize for HeadingNode {
    fn finalize(
        &self,
        _: Tracked<dyn World>,
        _: StyleChain,
        realized: Content,
    ) -> SourceResult<Content> {
        let scale = match self.level.get() {
            1 => 1.4,
            2 => 1.2,
            _ => 1.0,
        };

        let size = Em::new(scale);
        let above = Em::new(if self.level.get() == 1 { 1.8 } else { 1.44 }) / scale;
        let below = Em::new(0.66) / scale;

        let mut map = StyleMap::new();
        map.set(TextNode::SIZE, TextSize(size.into()));
        map.set(TextNode::WEIGHT, FontWeight::BOLD);
        map.set(BlockNode::ABOVE, VNode::block_around(above.into()));
        map.set(BlockNode::BELOW, VNode::block_around(below.into()));

        Ok(realized.styled_with_map(map))
    }
}
