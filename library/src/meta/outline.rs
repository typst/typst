use super::{Counter, CounterKey, HeadingElem, LocalName};
use crate::layout::{BoxElem, HElem, HideElem, ParbreakElem, RepeatElem};
use crate::prelude::*;
use crate::text::{LinebreakElem, SpaceElem, TextElem};

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
#[element(Show, LocalName)]
pub struct OutlineElem {
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
    #[default(Some(RepeatElem::new(TextElem::packed(".")).pack()))]
    pub fill: Option<Content>,
}

impl Show for OutlineElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut seq = vec![ParbreakElem::new().pack()];
        if let Some(title) = self.title(styles) {
            let title = title
                .clone()
                .unwrap_or_else(|| self.local_name_content(styles).spanned(self.span()));

            seq.push(
                HeadingElem::new(title)
                    .with_level(NonZeroUsize::ONE)
                    .with_numbering(None)
                    .with_outlined(false)
                    .pack(),
            );
        }

        let indent = self.indent(styles);
        let depth = self.depth(styles);

        let mut ancestors: Vec<&HeadingElem> = vec![];
        let elems = vt.introspector.query(Selector::Elem(
            HeadingElem::func(),
            Some(dict! { "outlined" => true }),
        ));

        for elem in &elems {
            let heading = elem.to::<HeadingElem>().unwrap();
            let location = heading.0.location().unwrap();
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

            // Add hidden ancestors numberings to realize the indent.
            if indent {
                let mut hidden = Content::empty();
                for ancestor in &ancestors {
                    if let Some(numbering) = ancestor.numbering(StyleChain::default()) {
                        let numbers = Counter::of(HeadingElem::func())
                            .at(vt, ancestor.0.location().unwrap())?
                            .display(vt, &numbering)?;
                        hidden += numbers + SpaceElem::new().pack();
                    };
                }

                if !ancestors.is_empty() {
                    seq.push(HideElem::new(hidden).pack());
                    seq.push(SpaceElem::new().pack());
                }
            }

            // Format the numbering.
            let mut start = heading.body();
            if let Some(numbering) = heading.numbering(StyleChain::default()) {
                let numbers = Counter::of(HeadingElem::func())
                    .at(vt, location)?
                    .display(vt, &numbering)?;
                start = numbers + SpaceElem::new().pack() + start;
            };

            // Add the numbering and section name.
            seq.push(start.linked(Destination::Location(location)));

            // Add filler symbols between the section name and page number.
            if let Some(filler) = self.fill(styles) {
                seq.push(SpaceElem::new().pack());
                seq.push(
                    BoxElem::new()
                        .with_body(Some(filler.clone()))
                        .with_width(Fr::one().into())
                        .pack(),
                );
                seq.push(SpaceElem::new().pack());
            } else {
                seq.push(HElem::new(Fr::one().into()).pack());
            }

            // Add the page number and linebreak.
            let page = Counter::new(CounterKey::Page)
                // query the page counter state at location of heading
                .at(vt, location)?
                .first();
            let end = TextElem::packed(eco_format!("{page}"));
            seq.push(end.linked(Destination::Location(location)));
            seq.push(LinebreakElem::new().pack());
            ancestors.push(heading);
        }

        seq.push(ParbreakElem::new().pack());

        Ok(Content::sequence(seq))
    }
}

impl LocalName for OutlineElem {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::GERMAN => "Inhaltsverzeichnis",
            Lang::ITALIAN => "Indice",
            Lang::ENGLISH | _ => "Contents",
        }
    }
}
