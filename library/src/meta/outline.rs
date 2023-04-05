use super::{Counter, CounterKey, HeadingElem, LocalName, Refable};
use crate::layout::{BoxElem, HElem, HideElem, ParbreakElem, RepeatElem};
use crate::prelude::*;
use crate::text::{LinebreakElem, SpaceElem, TextElem};

/// A table of contents, figures, or other elements.
///
/// This function generates a list of all occurances of an element in the
/// document, up to a given depth. The element's numbering and page number will
/// be displayed in the outline alongside its title or caption. By default this
/// generates a table of contents.
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
/// ## Alternative outlines
/// By setting the `target` parameter, the outline can be used to generate a
/// list of other kinds of elements than headings. In the example below, we list
/// all figures containing images by setting `target` to `{figure.where(kind:
/// image)}`. We could have also set it to just `figure`, but then the list
/// would also include figures containing tables or other material. For more
/// details on the `where` selector, [see here]($type/content.where).
///
/// ```example
/// #outline(
///   title: [List of Figures],
///   target: figure.where(kind: image),
/// )
///
/// #figure(
///   image("tiger.jpg"),
///   caption: [A nice figure!],
/// )
/// ```
///
/// Display: Outline
/// Category: meta
#[element(Show, LocalName)]
pub struct OutlineElem {
    /// The title of the outline.
    ///
    /// - When set to `{auto}`, an appropriate title for the
    ///   [text language]($func/text.lang) will be used. This is the default.
    /// - When set to `{none}`, the outline will not have a title.
    /// - A custom title can be set by passing content.
    #[default(Some(Smart::Auto))]
    pub title: Option<Smart<Content>>,

    /// The type of element to include in the outline.
    ///
    /// To list figures containing a specific kind of element, like a table, you
    /// can write `{figure.where(kind: table)}`.
    ///
    /// ```example
    /// #outline(
    ///   title: [List of Tables],
    ///   target: figure.where(kind: table),
    /// )
    ///
    /// #figure(
    ///   table(
    ///     columns: 4,
    ///     [t], [1], [2], [3],
    ///     [y], [0.3], [0.7], [0.5],
    ///   ),
    ///   caption: [Experiment results],
    /// )
    /// ```
    #[default(Selector::Elem(HeadingElem::func(), Some(dict! { "outlined" => true })))]
    pub target: Selector,

    /// The maximum level up to which elements are included in the outline. When
    /// this argument is `{none}`, all elements are included.
    ///
    /// ```example
    /// #set heading(numbering: "1.")
    /// #outline(depth: 2)
    ///
    /// = Yes
    /// Top-level section.
    ///
    /// == Still
    /// Subsection.
    ///
    /// === Nope
    /// Not included.
    /// ```
    pub depth: Option<NonZeroUsize>,

    /// Whether to indent the sub-elements to align the start of their numbering
    /// with the title of their parents. This will only have an effect if a
    /// [heading numbering]($func/heading.numbering) is set.
    ///
    /// ```example
    /// #set heading(numbering: "1.a.")
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
            if depth < refable.level(StyleChain::default()) {
                continue;
            }

            let Some(outline) = refable.outline(vt, StyleChain::default())? else {
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
                            .counter(StyleChain::default())
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
