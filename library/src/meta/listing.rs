use super::{Counter, CounterKey, HeadingElem, LocalName};
use crate::layout::{BoxElem, HElem, ParbreakElem, RepeatElem};
use crate::meta::FigureElem;
use crate::prelude::*;
use crate::text::{LinebreakElem, SpaceElem, TextElem};
use crate::visualize::ImageElem;

/// A figure listing / table of figures / table of tables / table of code.
///
/// This function generates a list of all figures in the document.
/// The [figure]($func/figure) numbering will be reproduced within the listing.
///
/// ## Example
/// ```example
/// #listing(of: table, title: "Table of Tables")
///
/// #figure(caption: "A nice figure!")[
///   #table(
///     columns: (auto, 1fr),
///     "A", "Ampere",
///     "V", "Volt",
///     "Hz", "Hertz",
///     "K", "Kelvin",
///   )
/// ]
/// ```
///
/// Display: Listing
/// Category: meta
#[element(Show, LocalName)]
pub struct ListingElem {
    /// The title of the outline.
    ///
    /// - When set to `{auto}`, an appropriate title for the [text
    ///   language]($func/text.lang) will be used. This is the default.
    /// - When set to `{none}`, the outline will not have a title.
    /// - A custom title can be set by passing content.
    #[default(Some(Smart::Auto))]
    pub title: Option<Smart<Content>>,

    /// The type of element to include in the listing.
    /// - When set to `{figure}`, all figures regardless of their content are included.
    /// - When set to `{image}`, all figures are included.
    /// - When set to `{table}`, all tables are included.
    /// - When set to `{raw}`, all code blocks are included.
    #[default(ImageElem::func())]
    pub of: ElemFunc,

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

impl Show for ListingElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut contents = vec![ParbreakElem::new().pack()];
        if let Some(title) = self.title(styles) {
            let title = title.clone().unwrap_or_else(|| {
                TextElem::packed(self.local_name(TextElem::lang_in(styles)))
            });

            contents.push(
                HeadingElem::new(title)
                    .with_level(NonZeroUsize::ONE)
                    .with_numbering(None)
                    .with_outlined(false)
                    .pack(),
            );
        }

        let of = self.of(styles);

        let elems = vt.introspector.query(Selector::Elem(FigureElem::func(), None));

        for elem in &elems {
            let figure = elem.to::<FigureElem>().expect("expected a figure");
            let location = figure.0.location().unwrap();
            if !figure.listed(StyleChain::default()) {
                continue;
            }

            // Ignore figures that are not of the desired type
            // Unless the filtered type is `figure` which includes all figures.
            if of != FigureElem::func()
                && figure.determine_type(styles).func() != Some(of)
            {
                continue;
            }

            // Get the figure caption.
            let start = figure.show_caption(vt, styles)?;

            // Add the numbering and figure name.
            contents.push(start.linked(Destination::Location(location)));

            // Add filler symbols between the figure name and page number.
            if let Some(filler) = self.fill(styles) {
                contents.push(SpaceElem::new().pack());
                contents.push(
                    BoxElem::new()
                        .with_body(Some(filler.clone()))
                        .with_width(Fr::one().into())
                        .pack(),
                );
                contents.push(SpaceElem::new().pack());
            } else {
                contents.push(HElem::new(Fr::one().into()).pack());
            }

            // Add the page number and linebreak.
            let page = Counter::new(CounterKey::Page)
                // query the page counter state at location of heading
                .at(vt, location)?
                .first();
            let end = TextElem::packed(eco_format!("{page}"));
            contents.push(end.linked(Destination::Location(location)));
            contents.push(LinebreakElem::new().pack());
        }

        contents.push(ParbreakElem::new().pack());

        Ok(Content::sequence(contents))
    }
}

impl LocalName for ListingElem {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::ENGLISH | _ => "Contents",
        }
    }
}
