use std::str::FromStr;

use super::{Count, Counter, CounterUpdate, LocalName, Numbering, NumberingPattern};
use crate::layout::{BlockElem, VElem};
use crate::meta::{Refable, Supplement};
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
#[element(Locatable, Synthesize, Count, Show, LocalName, Refable)]
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

    /// The counter to use for the figure.
    #[default(Smart::Auto)]
    pub counter: Smart<Counter>,

    /// Whether the figure should appear in the list of figures/tables/code.
    #[default(true)]
    pub listed: bool,

    /// How to number the figure. Accepts a
    /// [numbering pattern or function]($func/numbering).
    #[default(Some(NumberingPattern::from_str("1").unwrap().into()))]
    pub numbering: Option<Numbering>,

    /// The type of the figure.
    /// Setting this will override the automatic detection.
    #[default(Smart::Auto)]
    pub of: Smart<ElemFunc>,

    /// The element to use for the figure's properties.
    #[synthesized]
    pub element: Option<Content>,

    /// The vertical gap between the body and caption.
    #[default(Em::new(0.65).into())]
    pub gap: Length,
}

impl FigureElem {
    /// Determines the type of the figure by looking at the content, finding all
    /// [`Figurable`] elements and sorting them by priority then returning the highest.
    pub fn determine_type(&self, styles: StyleChain) -> Option<Content> {
        let potential_elems = self.body().query(|content| {
            content.can::<dyn Figurable>() && content.can::<dyn LocalName>()
        });

        potential_elems.into_iter().max_by_key(|elem| {
            elem.with::<dyn Figurable>()
                .expect("should be figurable")
                .priority(styles)
        })
    }

    /// Finds the element with the given function in the figure's content.
    /// Returns `None` if no element with the given function is found.
    pub fn find_elem(&self, func: ElemFunc) -> Option<Content> {
        let potential_elems = self.body().query(|content| {
            content.can::<dyn Figurable>() && content.can::<dyn LocalName>()
        });

        potential_elems.into_iter().find(|elem| elem.func() == func)
    }

    pub fn resolve_of(&self, styles: StyleChain) -> ElemFunc {
        match self.of(styles) {
            Smart::Custom(func) => func,
            Smart::Auto => unreachable!("should be synthesized"),
        }
    }

    pub fn resolve_element(&self) -> Content {
        self.element().expect("should be synthesized")
    }

    pub fn resolve_counter(&self, styles: StyleChain) -> Counter {
        match self.counter(styles) {
            Smart::Auto => self
                .resolve_element()
                .with::<dyn Figurable>()
                .expect("should be figurable")
                .counter(styles),
            Smart::Custom(other) => other,
        }
    }

    pub fn resolve_supplement(&self, styles: StyleChain) -> Option<Supplement> {
        match self.supplement(styles) {
            Smart::Auto => Some(
                self.resolve_element()
                    .with::<dyn Figurable>()
                    .expect("should be figurable")
                    .supplement(styles),
            ),
            Smart::Custom(other) => other,
        }
    }

    pub fn show_supplement_and_numbering(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        external_supp: Option<Content>,
    ) -> SourceResult<Option<Content>> {
        let elem = self.resolve_element();
        if let Some(numbering) = self.numbering(styles) {
            let mut name = if let Some(supplement) = external_supp {
                supplement
            } else {
                self.resolve_supplement(styles)
                    .map_or(Ok(None), |supplement| {
                        supplement.resolve(vt, [elem.into()]).map(Some)
                    })?
                    .unwrap_or_else(Content::empty)
            };

            let counter = self.resolve_counter(styles);

            if !name.is_empty() {
                name += TextElem::packed("\u{a0}");
            }

            let number = counter
                .at(vt, self.0.location().expect("missing location"))?
                .display(vt, &numbering)?
                .spanned(self.span());

            Ok(Some(name + number))
        } else {
            Ok(None)
        }
    }

    pub fn show_caption(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let Some(mut caption) = self.caption(styles) else {
            return Ok(Content::empty());
        };

        if let Some(sup_and_num) = self.show_supplement_and_numbering(vt, styles, None)? {
            caption = sup_and_num + TextElem::packed(": ") + caption;
        }

        Ok(caption)
    }
}

impl Synthesize for FigureElem {
    fn synthesize(&mut self, styles: StyleChain) -> SourceResult<()> {
        Self::func();
        self.push_numbering(self.numbering(styles));
        self.push_counter(self.counter(styles));

        let type_ = match self.of(styles) {
            Smart::Auto => {
                let Some(type_) = self.determine_type(styles) else {
                    bail!(self.span(), "unable to determine figure type")
                };

                type_
            }
            Smart::Custom(func) => {
                let Some(type_) = self.find_elem(func) else {
                    bail!(self.span(), "unable to find figure child of type: {}", func.name())
                };

                type_
            }
        };

        self.push_of(Smart::Custom(type_.func()));
        self.push_element(Some(type_));

        Ok(())
    }
}

impl Show for FigureElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let counter = self.resolve_counter(styles);
        let counter_update = (self.numbering(styles).is_some()
            && counter != Counter::of(Self::func()))
        .then(|| counter.update(CounterUpdate::Step(NonZeroUsize::ONE)))
        .unwrap_or_else(Content::empty);

        let mut realized = self.body();

        if self.caption(styles).is_some() {
            realized += VElem::weak(self.gap(styles).into()).pack();
            realized += self.show_caption(vt, styles)?;
        }

        Ok(counter_update
            + BlockElem::new()
                .with_body(Some(realized))
                .with_breakable(false)
                .pack()
                .aligned(Axes::with_x(Some(Align::Center.into()))))
    }
}

impl Count for FigureElem {
    fn update(&self) -> Option<CounterUpdate> {
        // if the figure is numbered.
        self.numbering(StyleChain::default())
            .is_some()
            .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl LocalName for FigureElem {
    fn local_name(&self, lang: Lang) -> &'static str {
        self.element()
            .expect("missing element")
            .with::<dyn LocalName>()
            .expect("missing local name")
            .local_name(lang)
    }
}

impl Refable for FigureElem {
    fn reference(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        supplement: Option<Content>,
    ) -> SourceResult<Content> {
        let Some(desc) = self.show_supplement_and_numbering(vt, styles, supplement)? else {
            bail!(self.span(), "cannot reference unnumbered figure")
        };

        Ok(desc)
    }

    fn location(&self) -> Option<Location> {
        self.0.location()
    }

    fn outline(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Option<Content>> {
        // if the figure is not listed, it is not referenced.
        if !self.listed(styles) {
            return Ok(None);
        }

        self.show_caption(vt, styles).map(Some)
    }
}

/// An element that can be placed in a figure.
/// This trait is used to determine the type of a figure, it counter, its numbering pattern
/// and the supplement to use for referencing it and creating the citation.
/// The element chosen as the figure's content is the one with the highest priority.
pub trait Figurable {
    /// The type of the figure's content.
    fn counter(&self, styles: StyleChain) -> Counter;

    /// The supplement to use for referencing the figure.
    fn supplement(&self, styles: StyleChain) -> Supplement;

    /// The priority of this element.
    fn priority(&self, styles: StyleChain) -> isize;
}
