use std::str::FromStr;

use super::{
    Count, Counter, CounterUpdate, LocalName, Numbering, NumberingPattern, Supplement,
};
use crate::prelude::*;
use crate::text::TextElem;
use crate::{
    layout::{BlockElem, VElem},
    meta::RefSupplement,
};

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
#[element(Locatable, Synthesize, Count, Show, LocalName, RefSupplement)]
pub struct FigureElem {
    /// The content of the figure. Often, an [image]($func/image).
    #[required]
    pub body: Content,

    /// The supplement of the figure, shows before the numbering.
    /// And if you reference a figure, this will be the default supplement of it.
    pub supplement: Smart<Option<Supplement>>,

    /// The figure's caption.
    pub caption: Option<Content>,

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

        if let Some(mut caption) = self.caption(styles) {
            if let Some(numbering) = self.numbering(styles) {
                let mut supplement = self.ref_supplement(vt, styles)?;
                if !supplement.is_empty() {
                    supplement += TextElem::packed('\u{a0}');
                }
                caption = supplement
                    + Counter::of(Self::func())
                        .display(Some(numbering), false)
                        .spanned(self.span())
                    + TextElem::packed(": ")
                    + caption;
            }

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
        self.numbering(StyleChain::default())
            .is_some()
            .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl RefSupplement for FigureElem {
    fn ref_supplement(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        Ok(match self.supplement(styles) {
            Smart::Auto => TextElem::packed(self.local_name(TextElem::lang_in(styles))),
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(Supplement::Content(content))) => content.clone(),
            Smart::Custom(Some(Supplement::Func(func))) => {
                func.call_vt(vt, []).map(Value::display)?
            }
        })
    }
}

impl LocalName for FigureElem {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::GERMAN => "Abbildung",
            Lang::ENGLISH | _ => "Figure",
        }
    }
}
