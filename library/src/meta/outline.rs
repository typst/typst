use std::any::TypeId;

use ecow::eco_vec;

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
#[element(Show, Synthesize, LocalName)]
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
    #[default(None)]
    pub target: Option<Selector>,

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

impl Synthesize for OutlineElem {
    fn synthesize(&mut self, _vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        // if no target is set, we default to outlined headings.
        if let Some(target) = self.target(styles) {
            self.push_target(Some(Selector::All(eco_vec![
                target,
                Selector::Can(TypeId::of::<dyn Refable>())
            ])));
        } else {
            self.push_target(Some(Selector::Elem(
                HeadingElem::func(),
                Some(dict! { "outlined" => true }),
            )));
        }

        Ok(())
    }
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

        let mut ancestors: Vec<&HeadingElem> = vec![];
        let elems = vt
            .introspector
            .query(self.target(styles).expect("expected target to be set"));

        for elem in &elems {
            let refable = elem.with::<dyn Refable>().unwrap();
            let heading = elem.to::<HeadingElem>();
            let location = elem.location().expect("missing location");

            if depth < refable.level(styles) {
                continue;
            }

            // Deals with the ancestors of the current heading.
            // This is only applicable for headings.
            if let Some(heading) = heading {
                while ancestors.last().map_or(false, |last| {
                    last.level(StyleChain::default())
                        >= heading.level(StyleChain::default())
                }) {
                    ancestors.pop();
                }
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

            let Some(outline) = refable.outline(vt, styles)? else {
                continue;
            };

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

            if let Some(heading) = heading {
                ancestors.push(heading);
            }
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
            Lang::ENGLISH | _ => "Contents",
        }
    }
}
