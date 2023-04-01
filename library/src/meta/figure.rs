use std::str::FromStr;

use ecow::eco_vec;

use super::{
    Count, Counter, CounterKey, CounterUpdate, LocalName, Numbering, NumberingPattern,
};
use crate::layout::{BlockElem, TableElem, VElem};
use crate::meta::{ReferenceInfo, Supplement};
use crate::prelude::*;
use crate::text::{RawElem, TextElem};
use crate::visualize::ImageElem;

/// A figure with an optional caption.
///
/// ## Example
/// ```example
/// = Pipeline
/// @lab shows the central step of
/// our molecular testing pipeline.
///
/// #figure(
///   image("molecular.jpg", width: 80%),
///   caption: [
///     The molecular testing pipeline.
///   ],
/// ) <lab>
/// ```
///
/// Display: Figure
/// Category: meta
#[element(Locatable, Synthesize, Count, Show, LocalName, ReferenceInfo)]
pub struct FigureElem {
    /// The content of the figure. Often, an [image]($func/image).
    #[required]
    pub body: Content,

    /// The figure's caption.
    pub caption: Option<Content>,

    /// The figure's supplement, if not provided, the figure will attempt to
    /// automatically detect the counter from the content.
    #[default(Smart::Auto)]
    pub supplement: Smart<Option<Supplement>>,

    /// Whether the figure should appear in the list of figures/tables/code.
    #[default(true)]
    pub listed: bool,

    /// How to number the figure. Accepts a
    /// [numbering pattern or function]($func/numbering).
    #[default(Some(NumberingPattern::from_str("1").unwrap().into()))]
    pub numbering: Option<Numbering>,

    /// The vertical gap between the body and caption.
    #[default(Em::new(0.65).into())]
    pub gap: Length,
}

impl FigureElem {
    /// Determines the type of the figure based on its content.
    pub fn determine_type(&self) -> FigureType {
        let elems = eco_vec![
            Selector::Elem(ImageElem::func(), None),
            Selector::Elem(RawElem::func(), None),
            Selector::Elem(TableElem::func(), None),
        ];

        let query = self.body().query(Selector::Any(elems));

        // we query in the order of the highest priority to the lowest
        if let Some(image) = query.iter().find(|c| c.is::<ImageElem>()) {
            FigureType::Image(image.to::<ImageElem>().expect("expected an image").clone())
        } else if let Some(raw) = query.iter().find(|c| c.is::<RawElem>()) {
            FigureType::Raw(raw.to::<RawElem>().expect("expected an image").clone())
        } else if let Some(table) = query.iter().find(|c| c.is::<TableElem>()) {
            FigureType::Table(table.to::<TableElem>().expect("expected an image").clone())
        } else {
            FigureType::Other
        }
    }

    /// Creates the content of the figure's caption.
    pub fn show_caption(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let ty = self.determine_type();

        let mut caption = Content::empty();
        if let Some(caption_elem) = self.caption(styles) {
            if let Some(numbering) = self.numbering(styles) {
                let mut name =
                    ty.resolve_supplement(vt, self, styles)?.ok_or_else(|| {
                        vec![error!(self.span(), "Could not determine the figure type")]
                    })?;

                let counter = ty.counter().ok_or_else(|| {
                    vec![error!(self.span(), "Could not determine the figure type")]
                })?;

                if !name.is_empty() {
                    name += TextElem::packed("\u{a0}");
                }

                caption = name
                    + counter
                        .at(vt, self.0.location().expect("missing location"))?
                        .display(vt, &numbering)?
                        .spanned(self.span())
                    + TextElem::packed(": ")
                    + caption_elem;
            }
        }

        Ok(caption)
    }
}

impl Synthesize for FigureElem {
    fn synthesize(&mut self, styles: StyleChain) {
        self.push_numbering(self.numbering(styles));
    }
}

impl Show for FigureElem {
    #[track_caller]
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();

        if self.caption(styles).is_some() {
            let counter = self.determine_type().counter().ok_or_else(|| {
                vec![error!(self.span(), "Could not determine the figure type")]
            })?;

            realized += counter.clone().update(CounterUpdate::Step(NonZeroUsize::ONE));
            realized += VElem::weak(self.gap(styles).into()).pack();
            realized += self.show_caption(vt, styles)?;
        }

        Ok(BlockElem::new()
            .with_body(Some(realized))
            .with_breakable(false)
            .pack()
            .aligned(Axes::with_x(Some(Align::Center.into()))))
    }
}

impl Count for FigureElem {
    fn update(&self) -> Option<CounterUpdate> {
        // if the figure is numbered and is listed.
        (self.numbering(StyleChain::default()).is_some()
            && self.listed(StyleChain::default()))
        .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl LocalName for FigureElem {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::CHINESE => "图",
            Lang::GERMAN => "Abbildung",
            Lang::ITALIAN => "Figura",
            Lang::RUSSIAN => "Рисунок",
            Lang::ENGLISH | Lang::FRENCH | _ => "Figure",
        }
    }
}

impl ReferenceInfo for FigureElem {
    fn counter(&self, _: StyleChain) -> Option<Counter> {
        self.determine_type().counter()
    }

    fn numbering(&self, styles: StyleChain) -> Option<Numbering> {
        self.numbering(styles)
    }

    fn supplement(&self, styles: StyleChain) -> Option<Supplement> {
        self.determine_type().supplement(self, styles)
    }
}

/// The type of a figure
///
/// Priority list:
/// 1. `counter` and `supplement` explicitly set
/// 2. contains an image element
/// 3. contains a raw element
/// 4. contains a table element
/// 5. could not determine content
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum FigureType {
    /// A figure containing one (or more) images
    Image(ImageElem),

    /// A figure containing a table
    Table(TableElem),

    /// A figure containing a snippet of code
    Raw(RawElem),

    /// Could not determine the content of the figure.
    /// Unless the figure has `counter` and `supplement` explicitly set,
    /// this will be treated as an error.
    Other,
}

impl FigureType {
    /// Is this figure type unknown?
    pub fn is_other(&self) -> bool {
        matches!(self, Self::Other)
    }

    /// Gets the function of the element associated with this figure type.
    pub fn func(&self) -> Option<ElemFunc> {
        match self {
            FigureType::Image(_) => Some(ImageElem::func()),
            FigureType::Table(_) => Some(TableElem::func()),
            FigureType::Raw(_) => Some(RawElem::func()),
            FigureType::Other => None,
        }
    }

    /// Gets the counter associated with this figure type.
    pub fn counter(&self) -> Option<Counter> {
        match self {
            FigureType::Image(_) => {
                Some(Counter::new(CounterKey::Str("figure_images".into())))
            }
            FigureType::Table(_) => {
                Some(Counter::new(CounterKey::Str("figure_tables".into())))
            }
            FigureType::Raw(_) => {
                Some(Counter::new(CounterKey::Str("figure_raw_texts".into())))
            }
            FigureType::Other => None,
        }
    }

    /// Gets the supplement of this figure type.
    pub fn supplement(
        &self,
        figure: &FigureElem,
        styles: StyleChain,
    ) -> Option<Supplement> {
        let lang = TextElem::lang_in(styles);
        match figure.supplement(styles) {
            Smart::Auto => Some(Supplement::Content(TextElem::packed(match self {
                FigureType::Raw(raw) => raw.local_name(lang),
                FigureType::Table(table) => table.local_name(lang),
                FigureType::Image(_) | FigureType::Other => figure.local_name(lang),
            }))),
            Smart::Custom(None) => None,
            Smart::Custom(Some(supplement)) => Some(supplement),
        }
    }

    /// Resolves the supplement of this figure type.
    pub fn resolve_supplement(
        &self,
        vt: &mut Vt,
        figure: &FigureElem,
        styles: StyleChain,
    ) -> SourceResult<Option<Content>> {
        Ok(match self.supplement(figure, styles) {
            Some(Supplement::Content(content)) => Some(content),
            Some(Supplement::Func(func)) => {
                Some(func.call_vt(vt, [figure.clone().into()])?.display())
            }
            None => None,
        })
    }
}
