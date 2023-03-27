use std::str::FromStr;

use super::{
    AnchorElem, Count, Counter, CounterUpdate, ErrorElem, Numbering, NumberingPattern,
};
use crate::layout::{BlockElem, VElem};
use crate::prelude::*;
use crate::text::TextElem;

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
#[element(Locatable, Synthesize, Count, Show)]
pub struct FigureElem {
    /// The content of the figure. Often, an [image]($func/image).
    #[required]
    pub body: Content,

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
        let numbering = self.numbering(styles);
        let supplement =
            TextElem::packed(eco_format!("{}\u{a0}", self.local_name(styles)));

        // Build the content
        let mut realized = self.body();

        // Build the caption
        if let Some(caption) = self.caption(styles) {
            realized += VElem::weak(self.gap(styles).into()).pack();

            if let Some(numbering) = &numbering {
                realized += supplement.clone()
                    + Counter::of(Self::func())
                        .at(vt, self.0.location().unwrap())?
                        .display(vt, numbering)?
                        .spanned(self.span())
                    + TextElem::packed(": ");
            }

            realized += caption;
        }

        // Collect the content & caption into an unbreakable block
        let block = BlockElem::new()
            .with_body(Some(realized))
            .with_breakable(false)
            .pack()
            .aligned(Axes::with_x(Some(Align::Center.into())));

        // Build the reference name
        let ref_name = match numbering.map(Numbering::trimmed) {
            Some(numbering) => Some(
                supplement
                    + Counter::of(Self::func())
                        .at(vt, self.0.location().unwrap())?
                        .display(vt, &numbering)?
                        .spanned(self.span()),
            ),
            None => None,
        };

        let ref_name = ref_name.unwrap_or_else(|| {
            ErrorElem::from(error!(
                self.span(),
                "cannot reference figure without numbering"
            ))
            .pack()
        });

        Ok(AnchorElem::new(ref_name, block).pack().spanned(self.span()))
    }
}

impl Count for FigureElem {
    fn update(&self) -> Option<CounterUpdate> {
        self.numbering(StyleChain::default())
            .is_some()
            .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl FigureElem {
    fn local_name(&self, styles: StyleChain) -> &'static str {
        match TextElem::lang_in(styles) {
            Lang::GERMAN => "Abbildung",
            Lang::GREEK => "Εικόνα",
            Lang::ENGLISH | _ => "Figure",
        }
    }
}
