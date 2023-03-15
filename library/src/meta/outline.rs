use super::{HeadingNode, LocalName};
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
#[node(Synthesize, Show, LocalName)]
pub struct OutlineNode {
    /// The title of the outline.
    ///
    /// - When set to `{auto}`, an appropriate title for the [text
    ///   language]($func/text.lang) will be used. This is the default.
    /// - When set to `{none}`, the outline will not have a title.
    /// - A custom title can be set by passing content.
    #[default(Some(Smart::Auto))]
    pub title: Option<Smart<Content>>,

    /// The maximum depth up to which headings are included in the outline. When
    /// this argument is `{none}`, all headings are included.
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
    #[default(Some(RepeatNode::new(TextNode::packed(".")).pack()))]
    pub fill: Option<Content>,

    /// All outlined headings in the document.
    #[synthesized]
    pub headings: Vec<HeadingNode>,
}

impl Synthesize for OutlineNode {
    fn synthesize(&mut self, vt: &Vt, _: StyleChain) {
        let headings = vt
            .locate_node::<HeadingNode>()
            .filter(|node| node.outlined(StyleChain::default()))
            .cloned()
            .collect();

        self.push_headings(headings);
    }
}

impl Show for OutlineNode {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut seq = vec![ParbreakNode::new().pack()];
        if let Some(title) = self.title(styles) {
            let title = title.clone().unwrap_or_else(|| {
                TextNode::packed(self.local_name(TextNode::lang_in(styles)))
            });

            seq.push(
                HeadingNode::new(title)
                    .with_level(NonZeroUsize::new(1).unwrap())
                    .with_numbering(None)
                    .with_outlined(false)
                    .pack(),
            );
        }

        let indent = self.indent(styles);
        let depth = self.depth(styles);

        let mut ancestors: Vec<&HeadingNode> = vec![];
        for heading in self.headings().iter() {
            if !heading.outlined(StyleChain::default()) {
                continue;
            }

            if let Some(depth) = depth {
                if depth < heading.level(StyleChain::default()) {
                    continue;
                }
            }

            while ancestors.last().map_or(false, |last| {
                last.level(StyleChain::default()) >= heading.level(StyleChain::default())
            }) {
                ancestors.pop();
            }

            // Adjust the link destination a bit to the topleft so that the
            // heading is fully visible.
            let mut loc = heading.0.expect_field::<Location>("location");
            loc.pos -= Point::splat(Abs::pt(10.0));

            // Add hidden ancestors numberings to realize the indent.
            if indent {
                let mut hidden = Content::empty();
                for ancestor in &ancestors {
                    if let Some(numbering) = ancestor.numbering(StyleChain::default()) {
                        let numbers = ancestor.numbers().unwrap();
                        hidden += numbering.apply(vt.world(), &numbers)?.display()
                            + SpaceNode::new().pack();
                    };
                }

                if !ancestors.is_empty() {
                    seq.push(HideNode::new(hidden).pack());
                    seq.push(SpaceNode::new().pack());
                }
            }

            // Format the numbering.
            let mut start = heading.body();
            if let Some(numbering) = heading.numbering(StyleChain::default()) {
                let numbers = heading.numbers().unwrap();
                start = numbering.apply(vt.world(), &numbers)?.display()
                    + SpaceNode::new().pack()
                    + start;
            };

            // Add the numbering and section name.
            seq.push(start.linked(Destination::Internal(loc)));

            // Add filler symbols between the section name and page number.
            if let Some(filler) = self.fill(styles) {
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
            ancestors.push(heading);
        }

        seq.push(ParbreakNode::new().pack());

        Ok(Content::sequence(seq))
    }
}

impl LocalName for OutlineNode {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::GERMAN => "Inhaltsverzeichnis",
            Lang::ENGLISH | _ => "Contents",
        }
    }
}
