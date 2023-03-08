use super::HeadingNode;
use crate::layout::{BoxNode, HNode, HideNode, ParbreakNode, RepeatNode};
use crate::prelude::*;
use crate::text::{LinebreakNode, SpaceNode, TextNode};

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
/// Display: Outline
/// Category: meta
#[node(Prepare, Show)]
pub struct OutlineNode {
    /// The title of the outline.
    ///
    /// - When set to `{auto}`, an appropriate title for the [text
    ///   language]($func/text.lang) will be used. This is the default.
    /// - When set to `{none}`, the outline will not have a title.
    /// - A custom title can be set by passing content.
    #[settable]
    #[default(Some(Smart::Auto))]
    pub title: Option<Smart<Content>>,

    /// The maximum depth up to which headings are included in the outline. When
    /// this argument is `{none}`, all headings are included.
    #[settable]
    #[default]
    pub depth: Option<NonZeroUsize>,

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
    #[settable]
    #[default(false)]
    pub indent: bool,

    /// Content to fill the space between the title and the page number. Can be
    /// set to `none` to disable filling. The default is `{repeat[.]}`.
    ///
    /// ```example
    /// #outline(fill: line(length: 100%))
    ///
    /// = A New Beginning
    /// ```
    #[settable]
    #[default(Some(RepeatNode::new(TextNode::packed(".")).pack()))]
    pub fill: Option<Content>,
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
            .filter(|node| *node.field("outlined").unwrap() == Value::Bool(true))
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
        let mut seq = vec![ParbreakNode::new().pack()];
        if let Some(title) = Self::title_in(styles) {
            let title = title.clone().unwrap_or_else(|| {
                TextNode::packed(match TextNode::lang_in(styles) {
                    Lang::GERMAN => "Inhaltsverzeichnis",
                    Lang::ENGLISH | _ => "Contents",
                })
            });

            seq.push(
                HeadingNode::new(title)
                    .pack()
                    .styled(HeadingNode::set_numbering(None))
                    .styled(HeadingNode::set_outlined(false)),
            );
        }

        let indent = Self::indent_in(styles);
        let depth = Self::depth_in(styles);

        let mut ancestors: Vec<&Content> = vec![];
        for (_, node) in vt.locate(Selector::node::<HeadingNode>()) {
            if *node.field("outlined").unwrap() != Value::Bool(true) {
                continue;
            }

            let heading = node.to::<HeadingNode>().unwrap();
            if let Some(depth) = depth {
                if depth < heading.level() {
                    continue;
                }
            }

            while ancestors.last().map_or(false, |last| {
                last.to::<HeadingNode>().unwrap().level() >= heading.level()
            }) {
                ancestors.pop();
            }

            // Adjust the link destination a bit to the topleft so that the
            // heading is fully visible.
            let mut loc = node.field("loc").unwrap().clone().cast::<Location>().unwrap();
            loc.pos -= Point::splat(Abs::pt(10.0));

            // Add hidden ancestors numberings to realize the indent.
            if indent {
                let hidden: Vec<_> = ancestors
                    .iter()
                    .map(|node| node.field("numbers").unwrap())
                    .filter(|&numbers| *numbers != Value::None)
                    .map(|numbers| numbers.clone().display() + SpaceNode::new().pack())
                    .collect();

                if !hidden.is_empty() {
                    seq.push(HideNode::new(Content::sequence(hidden)).pack());
                    seq.push(SpaceNode::new().pack());
                }
            }

            // Format the numbering.
            let mut start = heading.body();
            let numbers = node.field("numbers").unwrap();
            if *numbers != Value::None {
                start = numbers.clone().display() + SpaceNode::new().pack() + start;
            };

            // Add the numbering and section name.
            seq.push(start.linked(Destination::Internal(loc)));

            // Add filler symbols between the section name and page number.
            if let Some(filler) = Self::fill_in(styles) {
                seq.push(SpaceNode::new().pack());
                seq.push(
                    BoxNode::new()
                        .with_body(Some(filler.clone()))
                        .with_width(Fr::one().into())
                        .pack(),
                );
                seq.push(SpaceNode::new().pack());
            } else {
                seq.push(HNode::new(Fr::one().into()).pack());
            }

            // Add the page number and linebreak.
            let end = TextNode::packed(eco_format!("{}", loc.page));
            seq.push(end.linked(Destination::Internal(loc)));
            seq.push(LinebreakNode::new().pack());

            ancestors.push(node);
        }

        seq.push(ParbreakNode::new().pack());

        Ok(Content::sequence(seq))
    }
}
