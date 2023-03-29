use std::str::FromStr;

use super::{
    Count, Counter, CounterUpdate, LocalName, Numbering, NumberingPattern, ReferenceInfo,
    Supplement,
};
use crate::layout::{BlockElem, VElem};
use crate::prelude::*;
use crate::text::TextElem;

/// A caption in figure.
///
/// Display: Caption
/// Category: meta
#[element(Locatable)]
pub struct CaptionElem {
    /// Caption content.
    #[required]
    pub content: Content,

    /// The supplement/prefix of the caption, will be used in reference too.
    pub supplement: Smart<Option<Supplement>>,

    /// Counter of this caption, if do not provide, the default one will be used.
    pub counter: Option<Counter>,

    /// The separator between "Figure 1", and caption, default will be ": "
    pub sep: Option<Content>,
}

cast_from_value! {
    CaptionElem,
    v: Content => v.to::<Self>().map(|c| c.clone()).unwrap_or_else(|| CaptionElem::new(v))
}

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
    pub caption: Option<CaptionElem>,

    /// How to number the figure. Accepts a
    /// [numbering pattern or function]($func/numbering).
    #[default(Some(NumberingPattern::from_str("1").unwrap().into()))]
    pub numbering: Option<Numbering>,

    /// The vertical gap between the body and caption.
    #[default(Em::new(0.65).into())]
    pub gap: Length,
}

impl Synthesize for FigureElem {
    fn synthesize(&mut self, styles: StyleChain) {
        self.push_numbering(self.numbering(styles));
    }
}

impl Show for FigureElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();

        if let Some(caption_elem) = self.caption(styles) {
            let mut caption = Content::empty();

            if let Some(numbering) = self.numbering(styles) {
                caption += self.resolve_supplement(vt, styles, self.clone().pack())?;

                if !caption.is_empty() {
                    caption += TextElem::packed('\u{a0}');
                }

                let counter = self.counter(styles);

                caption +=
                    counter.clone().display(Some(numbering), false).spanned(self.span())
                        + caption_elem.sep(styles).unwrap_or(TextElem::packed(": "));

                if counter != Counter::of(Self::func()) {
                    caption += counter.update(CounterUpdate::Step(NonZeroUsize::ONE))
                }
            }

            caption += caption_elem.content();

            realized += VElem::weak(self.gap(styles).into()).pack();
            realized += caption;
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
        (self.counter(StyleChain::default()) == Counter::of(Self::func())
            && self.numbering(StyleChain::default()).is_some())
        .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl ReferenceInfo for FigureElem {
    fn counter(&self, styles: StyleChain) -> Counter {
        self.caption(styles)
            .and_then(|caption| caption.counter(styles))
            .unwrap_or(Counter::of(Self::func()))
    }

    fn supplement(&self, styles: StyleChain) -> Smart<Option<Supplement>> {
        self.caption(styles)
            .map(|caption| caption.supplement(styles))
            .unwrap_or(Smart::Auto)
    }
}

impl LocalName for FigureElem {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::GERMAN => "Abbildung",
            Lang::ITALIAN => "Figura",
            Lang::ENGLISH | _ => "Figure",
        }
    }
}
