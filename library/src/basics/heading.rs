use typst::font::FontWeight;

use crate::compute::NumberingPattern;
use crate::layout::{BlockNode, VNode};
use crate::prelude::*;
use crate::text::{SpaceNode, TextNode, TextSize};

/// A section heading.
#[func]
#[capable(Prepare, Show, Finalize)]
#[derive(Debug, Hash)]
pub struct HeadingNode {
    /// The logical nesting depth of the section, starting from one. In the
    /// default style, this controls the text size of the heading.
    pub level: NonZeroUsize,
    /// The heading's contents.
    pub body: Content,
}

#[node]
impl HeadingNode {
    /// How to number the heading.
    #[property(referenced)]
    pub const NUMBERING: Option<NumberingPattern> = None;

    /// Whether the heading should appear in the outline.
    pub const OUTLINED: bool = true;

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

impl Prepare for HeadingNode {
    fn prepare(&self, vt: &mut Vt, mut this: Content, styles: StyleChain) -> Content {
        let my_id = vt.identify(&this);

        let mut counter = HeadingCounter::new();
        for (node_id, node) in vt.locate(Selector::node::<HeadingNode>()) {
            if node_id == my_id {
                break;
            }

            if matches!(node.field("numbers"), Some(Value::Str(_))) {
                let heading = node.to::<Self>().unwrap();
                counter.advance(heading);
            }
        }

        let mut numbers = Value::None;
        if let Some(pattern) = styles.get(Self::NUMBERING) {
            numbers = Value::Str(pattern.apply(counter.advance(self)).into());
        }

        this.push_field("outlined", Value::Bool(styles.get(Self::OUTLINED)));
        this.push_field("numbers", numbers);

        let meta = Meta::Node(my_id, this.clone());
        this.styled(Meta::DATA, vec![meta])
    }
}

impl Show for HeadingNode {
    fn show(&self, _: &mut Vt, this: &Content, _: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body.clone();
        if let Some(Value::Str(numbering)) = this.field("numbers") {
            realized = TextNode::packed(numbering) + SpaceNode.pack() + realized;
        }
        Ok(BlockNode(realized).pack())
    }
}

impl Finalize for HeadingNode {
    fn finalize(&self, realized: Content) -> Content {
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
        map.set(BlockNode::STICKY, true);
        realized.styled_with_map(map)
    }
}

/// Counters through headings with different levels.
pub struct HeadingCounter(Vec<NonZeroUsize>);

impl HeadingCounter {
    /// Create a new heading counter.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Advance the counter and return the numbers for the given heading.
    pub fn advance(&mut self, heading: &HeadingNode) -> &[NonZeroUsize] {
        let level = heading.level.get();

        if self.0.len() >= level {
            self.0[level - 1] = self.0[level - 1].saturating_add(1);
            self.0.truncate(level);
        }

        while self.0.len() < level {
            self.0.push(NonZeroUsize::new(1).unwrap());
        }

        &self.0
    }
}
