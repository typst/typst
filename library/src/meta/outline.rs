use super::{Counter, CounterKey, HeadingElem, LocalName, Refable};
use crate::layout::{BoxElem, HElem, HideElem, ParbreakElem, RepeatElem};
use crate::prelude::*;
use crate::text::{LinebreakElem, SpaceElem, TextElem};

/// A section outline / table of contents / table of figures / table of tables / etc.
///
/// This function generates a list of all headings in the document, up to a
/// given depth. The [heading]($func/heading) numbering will be reproduced
/// within the outline.
///
/// Alternatively, by setting the `target` parameter, the outline can be used to
/// generate a list of all figures, tables, code blocks, etc. When the `target` parameter
/// is set, the `depth` parameter is ignored unless it is set to `heading`.
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
/// ## Example: List of figures
/// ```example
/// #outline(target: figure.where(kind: image), title: "Table of Figures")
///
/// #figure(caption: "A nice figure!")[
///  #image("/tiger.jpg")
/// ]
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

    /// The type of element to include in the outline.
    #[default(Selector::Elem(HeadingElem::func(), Some(dict! { "outlined" => true })))]
    pub target: Selector,

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
        // Build the outline title.
        if let Some(title) = self.title(styles) {
            let title = title.unwrap_or_else(|| {
                TextElem::packed(self.local_name(TextElem::lang_in(styles)))
                    .spanned(self.span())
            });

            seq.push(
                HeadingElem::new(title)
                    .with_level(NonZeroUsize::ONE)
                    .with_numbering(None)
                    .with_outlined(false)
                    .pack(),
            );
        }

        let indent = self.indent(styles);
        let depth = self.depth(styles).map_or(usize::MAX, NonZeroUsize::get);

        let mut ancestors: Vec<&Content> = vec![];
        let elems = vt.introspector.query(self.target(styles));

        for elem in &elems {
            let Some(refable) = elem.with::<dyn Refable>() else {
                bail!(elem.span(), "outlined elements must be referenceable");
            };

            let location = elem.location().expect("missing location");

            if depth < refable.level(styles) {
                continue;
            }

            let Some(outline) = refable.outline(vt, styles)? else {
                continue;
            };

            // Deals with the ancestors of the current element.
            // This is only applicable for elements with a hierarchy/level.
            while ancestors
                .last()
                .and_then(|ancestor| ancestor.with::<dyn Refable>())
                .map_or(false, |last| {
                    last.level(StyleChain::default())
                        >= refable.level(StyleChain::default())
                })
            {
                ancestors.pop();
            }

            // Add hidden ancestors numberings to realize the indent.
            if indent {
                let mut hidden = Content::empty();
                for ancestor in &ancestors {
                    let ancestor_refable = ancestor.with::<dyn Refable>().unwrap();

                    if let Some(numbering) =
                        ancestor_refable.numbering(StyleChain::default())
                    {
                        let numbers = ancestor_refable
                            .counter(styles)
                            .at(vt, ancestor.location().unwrap())?
                            .display(vt, &numbering)?;

                        hidden += numbers + SpaceElem::new().pack();
                    };
                }

                if !ancestors.is_empty() {
                    seq.push(HideElem::new(hidden).pack());
                    seq.push(SpaceElem::new().pack());
                }
            }

            // Add the outline of the element.
            seq.push(outline.linked(Destination::Location(location)));

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

            ancestors.push(elem);
        }

        seq.push(ParbreakElem::new().pack());

        Ok(Content::sequence(seq))
    }
}

impl LocalName for OutlineElem {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::CHINESE => "目录",
            Lang::FRENCH => "Table des matières",
            Lang::GERMAN => "Inhaltsverzeichnis",
            Lang::ITALIAN => "Indice",
            Lang::PORTUGUESE => "Sumário",
            Lang::RUSSIAN => "Содержание",
            Lang::UKRAINIAN => "Зміст",
            Lang::ENGLISH | _ => "Contents",
        }
    }
}
