use super::HeadingNode;
use crate::layout::{
    BoxNode, HNode, HideNode, ParbreakNode, RepeatNode, Sizing, Spacing,
};
use crate::prelude::*;
use crate::text::{LinebreakNode, SpaceNode, TextNode};

/// # Outline
/// A section outline / table of contents.
///
/// This function generates a list of all headings in the document, up to a
/// given depth. The [heading]($func/heading) numbering will be reproduced
/// within the outline.
///
/// ## Example
/// ```example
/// #outline()
///
/// = Introduction
/// #lorem(5)
///
/// = Prior work
/// #lorem(10)
/// ```
///
/// ## Category
/// meta
#[func]
#[capable(Prepare, Show)]
#[derive(Debug, Hash)]
pub struct OutlineNode;

#[node]
impl OutlineNode {
    /// The title of the outline.
    ///
    /// - When set to `{auto}`, an appropriate title for the [text
    ///   language]($func/text.lang) will be used. This is the default.
    /// - When set to `{none}`, the outline will not have a title.
    /// - A custom title can be set by passing content.
    #[property(referenced)]
    pub const TITLE: Option<Smart<Content>> = Some(Smart::Auto);

    /// The maximum depth up to which headings are included in the outline. When
    /// this argument is `{none}`, all headings are included.
    pub const DEPTH: Option<NonZeroUsize> = None;

    /// Whether to indent the subheadings to align the start of their numbering
    /// with the title of their parents. This will only have an effect if a
    /// [heading numbering]($func/heading.numbering) is set.
    ///
    /// ```example
    /// #set heading(numbering: "1.a.")
    ///
    /// #outline(indent: true)
    ///
    /// = About ACME Corp.
    ///
    /// == History
    /// #lorem(10)
    ///
    /// == Products
    /// #lorem(10)
    /// ```
    pub const INDENT: bool = false;

    /// Content to fill the space between the title and the page number. Can be
    /// set to `none` to disable filling. The default is `{repeat[.]}`.
    ///
    /// ```example
    /// #outline(fill: line(length: 100%))
    ///
    /// = A New Beginning
    /// ```
    #[property(referenced)]
    pub const FILL: Option<Content> = Some(RepeatNode(TextNode::packed(".")).pack());

    fn construct(_: &Vm, _: &mut Args) -> SourceResult<Content> {
        Ok(Self.pack())
    }
}

impl Prepare for OutlineNode {
    fn prepare(
        &self,
        vt: &mut Vt,
        mut this: Content,
        _: StyleChain,
    ) -> SourceResult<Content> {
        let headings = vt
            .locate(Selector::node::<HeadingNode>())
            .into_iter()
            .map(|(_, node)| node)
            .filter(|node| node.field("outlined").unwrap() == Value::Bool(true))
            .map(|node| Value::Content(node.clone()))
            .collect();

        this.push_field("headings", Value::Array(Array::from_vec(headings)));
        Ok(this)
    }
}

impl Show for OutlineNode {
    fn show(
        &self,
        vt: &mut Vt,
        _: &Content,
        styles: StyleChain,
    ) -> SourceResult<Content> {
        let mut seq = vec![ParbreakNode.pack()];
        if let Some(title) = styles.get(Self::TITLE) {
            let body = title.clone().unwrap_or_else(|| {
                TextNode::packed(match styles.get(TextNode::LANG) {
                    Lang::GERMAN => "Inhaltsverzeichnis",
                    Lang::ENGLISH | _ => "Contents",
                })
            });

            seq.push(
                HeadingNode { title: body, level: NonZeroUsize::new(1).unwrap() }
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
                let hidden: Vec<_> = ancestors
                    .iter()
                    .map(|node| node.field("numbers").unwrap())
                    .filter(|numbers| *numbers != Value::None)
                    .map(|numbers| numbers.display() + SpaceNode.pack())
                    .collect();

                if !hidden.is_empty() {
                    seq.push(HideNode(Content::sequence(hidden)).pack());
                    seq.push(SpaceNode.pack());
                }
            }

            // Format the numbering.
            let mut start = heading.title.clone();
            let numbers = node.field("numbers").unwrap();
            if numbers != Value::None {
                start = numbers.display() + SpaceNode.pack() + start;
            };

            // Add the numbering and section name.
            seq.push(start.linked(Destination::Internal(loc)));

            // Add filler symbols between the section name and page number.
            if let Some(filler) = styles.get(Self::FILL) {
                seq.push(SpaceNode.pack());
                seq.push(
                    BoxNode {
                        body: filler.clone(),
                        width: Sizing::Fr(Fr::one()),
                        height: Smart::Auto,
                        baseline: Rel::zero(),
                    }
                    .pack(),
                );
                seq.push(SpaceNode.pack());
            } else {
                let amount = Spacing::Fr(Fr::one());
                seq.push(HNode { amount, weak: false }.pack());
            }

            // Add the page number and linebreak.
            let end = TextNode::packed(format_eco!("{}", loc.page));
            seq.push(end.linked(Destination::Internal(loc)));
            seq.push(LinebreakNode { justify: false }.pack());

            ancestors.push(node);
        }

        seq.push(ParbreakNode.pack());

        Ok(Content::sequence(seq))
    }
}
