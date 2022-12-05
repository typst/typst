use crate::basics::HeadingNode;
use crate::layout::{BlockNode, HNode, HideNode, RepeatNode, Spacing};
use crate::prelude::*;
use crate::text::{LinebreakNode, SpaceNode, TextNode};

/// A section outline (table of contents).
#[derive(Debug, Hash)]
pub struct OutlineNode;

#[node(Prepare, Show)]
impl OutlineNode {
    /// The title of the outline.
    #[property(referenced)]
    pub const TITLE: Option<Smart<Content>> = Some(Smart::Auto);

    /// The maximum depth up to which headings are included in the outline.
    pub const DEPTH: Option<NonZeroUsize> = None;

    /// Whether to indent the subheadings to match their parents.
    pub const INDENT: bool = false;

    /// The fill symbol.
    #[property(referenced)]
    pub const FILL: Option<EcoString> = Some('.'.into());

    fn construct(_: &Vm, _: &mut Args) -> SourceResult<Content> {
        Ok(Self.pack())
    }
}

impl Prepare for OutlineNode {
    fn prepare(&self, vt: &mut Vt, mut this: Content, _: StyleChain) -> Content {
        let headings = vt
            .locate(Selector::node::<HeadingNode>())
            .into_iter()
            .map(|(_, node)| node)
            .filter(|node| node.field("outlined").unwrap() == Value::Bool(true))
            .map(|node| Value::Content(node.clone()))
            .collect();

        this.push_field("headings", Value::Array(Array::from_vec(headings)));
        this
    }
}

impl Show for OutlineNode {
    fn show(
        &self,
        vt: &mut Vt,
        _: &Content,
        styles: StyleChain,
    ) -> SourceResult<Content> {
        let mut seq = vec![];
        if let Some(title) = styles.get(Self::TITLE) {
            let body = title.clone().unwrap_or_else(|| {
                TextNode::packed(match styles.get(TextNode::LANG) {
                    Lang::GERMAN => "Inhaltsverzeichnis",
                    Lang::ENGLISH | _ => "Contents",
                })
            });

            seq.push(
                HeadingNode { body, level: NonZeroUsize::new(1).unwrap() }
                    .pack()
                    .styled(HeadingNode::NUMBERING, None)
                    .styled(HeadingNode::OUTLINED, false),
            );
        }

        let indent = styles.get(Self::INDENT);
        let depth = styles.get(Self::DEPTH);

        let mut ancestors: Vec<&Content> = vec![];
        for (_, node) in vt.locate(Selector::node::<HeadingNode>()) {
            if node.field("outlined").unwrap() != Value::Bool(true) {
                continue;
            }

            let heading = node.to::<HeadingNode>().unwrap();
            if let Some(depth) = depth {
                if depth < heading.level {
                    continue;
                }
            }

            while ancestors.last().map_or(false, |last| {
                last.to::<HeadingNode>().unwrap().level >= heading.level
            }) {
                ancestors.pop();
            }

            // Adjust the link destination a bit to the topleft so that the
            // heading is fully visible.
            let mut loc = node.field("loc").unwrap().cast::<Location>().unwrap();
            loc.pos -= Point::splat(Abs::pt(10.0));

            // Add hidden ancestors numberings to realize the indent.
            if indent {
                let text = ancestors
                    .iter()
                    .filter_map(|node| match node.field("numbers").unwrap() {
                        Value::Str(numbering) => {
                            Some(EcoString::from(numbering) + ' '.into())
                        }
                        _ => None,
                    })
                    .collect::<EcoString>();

                if !text.is_empty() {
                    seq.push(HideNode(TextNode::packed(text)).pack());
                    seq.push(SpaceNode.pack());
                }
            }

            // Format the numbering.
            let numbering = match node.field("numbers").unwrap() {
                Value::Str(numbering) => {
                    TextNode::packed(EcoString::from(numbering) + ' '.into())
                }
                _ => Content::empty(),
            };

            // Add the numbering and section name.
            let start = numbering + heading.body.clone();
            seq.push(start.linked(Destination::Internal(loc)));

            // Add filler symbols between the section name and page number.
            if let Some(filler) = styles.get(Self::FILL) {
                seq.push(SpaceNode.pack());
                seq.push(RepeatNode(TextNode::packed(filler.clone())).pack());
                seq.push(SpaceNode.pack());
            } else {
                let amount = Spacing::Fractional(Fr::one());
                seq.push(HNode { amount, weak: false }.pack());
            }

            // Add the page number and linebreak.
            let end = TextNode::packed(format_eco!("{}", loc.page));
            seq.push(end.linked(Destination::Internal(loc)));
            seq.push(LinebreakNode { justify: false }.pack());

            ancestors.push(node);
        }

        Ok(BlockNode(Content::sequence(seq)).pack())
    }
}
