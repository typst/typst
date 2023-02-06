use typst::font::FontWeight;

use crate::compute::Numbering;
use crate::layout::{BlockNode, VNode};
use crate::prelude::*;
use crate::text::{SpaceNode, TextNode, TextSize};

/// # Heading
/// A section heading.
///
/// With headings, you can structure your document into sections. Each heading
/// has a _level,_ which starts at one and is unbounded upwards. This level
/// indicates the logical role of the following content (section, subsection,
/// etc.)  A top-level heading indicates a top-level section of the document
/// (not the document's title).
///
/// Typst can automatically number your headings for you. To enable numbering,
/// specify how you want your headings to be numbered with a
/// [numbering pattern or function]($func/numbering).
///
/// Independently from the numbering, Typst can also automatically generate an
/// [outline]($func/outline) of all headings for you. To exclude one or more
/// headings from this outline, you can set the `outlined` parameter to
/// `{false}`.
///
/// ## Example
/// ```example
/// #set heading(numbering: "1.a)")
///
/// = Introduction
/// In recent years, ...
///
/// == Preliminaries
/// To start, ...
/// ```
///
/// ## Syntax
/// Headings have dedicated syntax: They can be created by starting a line with
/// one or multiple equals signs, followed by a space. The number of equals
/// signs determines the heading's logical nesting depth.
///
/// ## Parameters
/// - title: `Content` (positional, required)
///   The heading's title.
///
/// - level: `NonZeroUsize` (named)
///   The logical nesting depth of the heading, starting from one.
///
/// ## Category
/// basics
#[func]
#[capable(Prepare, Show, Finalize)]
#[derive(Debug, Hash)]
pub struct HeadingNode {
    /// The logical nesting depth of the section, starting from one. In the
    /// default style, this controls the text size of the heading.
    pub level: NonZeroUsize,
    /// The heading's contents.
    pub title: Content,
}

#[node]
impl HeadingNode {
    /// How to number the heading. Accepts a
    /// [numbering pattern or function]($func/numbering).
    ///
    /// ```example
    /// #set heading(numbering: "1.a.")
    ///
    /// = A section
    /// == A subsection
    /// === A sub-subsection
    /// ```
    #[property(referenced)]
    pub const NUMBERING: Option<Numbering> = None;

    /// Whether the heading should appear in the outline.
    ///
    /// ```example
    /// #outline()
    ///
    /// #heading[Normal]
    /// This is a normal heading.
    ///
    /// #heading(outlined: false)[Hidden]
    /// This heading does not appear
    /// in the outline.
    /// ```
    pub const OUTLINED: bool = true;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self {
            title: args.expect("title")?,
            level: args.named("level")?.unwrap_or(NonZeroUsize::new(1).unwrap()),
        }
        .pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "level" => Some(Value::Int(self.level.get() as i64)),
            "title" => Some(Value::Content(self.title.clone())),
            _ => None,
        }
    }
}

impl Prepare for HeadingNode {
    fn prepare(
        &self,
        vt: &mut Vt,
        mut this: Content,
        styles: StyleChain,
    ) -> SourceResult<Content> {
        let my_id = vt.identify(&this);

        let mut counter = HeadingCounter::new();
        for (node_id, node) in vt.locate(Selector::node::<HeadingNode>()) {
            if node_id == my_id {
                break;
            }

            let numbers = node.field("numbers").unwrap();
            if numbers != Value::None {
                let heading = node.to::<Self>().unwrap();
                counter.advance(heading);
            }
        }

        let mut numbers = Value::None;
        if let Some(numbering) = styles.get(Self::NUMBERING) {
            numbers = numbering.apply(vt.world(), counter.advance(self))?;
        }

        this.push_field("outlined", Value::Bool(styles.get(Self::OUTLINED)));
        this.push_field("numbers", numbers);

        let meta = Meta::Node(my_id, this.clone());
        Ok(this.styled(Meta::DATA, vec![meta]))
    }
}

impl Show for HeadingNode {
    fn show(&self, _: &mut Vt, this: &Content, _: StyleChain) -> SourceResult<Content> {
        let mut realized = self.title.clone();
        let numbers = this.field("numbers").unwrap();
        if numbers != Value::None {
            realized = numbers.display() + SpaceNode.pack() + realized;
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
