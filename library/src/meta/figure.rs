use std::str::FromStr;

use super::{
    Count, Counter, CounterKey, CounterUpdate, LocalName, Numbering, NumberingPattern,
};
use crate::layout::{BlockElem, VElem};
use crate::meta::{Refable, Supplement};
use crate::prelude::*;
use crate::text::TextElem;

/// A figure with an optional caption.
///
/// ## Content detection
/// By default, the figure will attempt to automatically detect the content
/// and use a priority list to detect which content is likely
/// to be the most important. The priority list is as follows:
/// - [image]($func/image) are the most important
/// - [code]($func/code) are the second most important
/// - [table]($func/table) are the third most important.
///
/// There can be a variety of content within a figure and only the first element
/// of the most important category will be used. For example, if a figure contains
/// an image and a table, the image will be used. This behaviour can be overridden
/// using the `of` parameter. By setting it, you can force the figure to use a
/// specific type of content. Note however, that the figure must contain an element
/// of the given type.
///
/// ```example
/// #figure(caption: [ Hello, world! ], of: table)[
///   #table(
///    columns: (auto, 1fr)
///    image("molecular.jpg", width: 32pt),
///    [ A first picture ],
///    image("molecular.jpg", width: 32pt),
///    [ A second picture ],
///   )
/// ]
/// ```
///
/// If you use an element that is not supported by the figure, and set it as its `of` parameter,
/// to be able to make an outline or reference it, you will need to manually specify the supplement
/// and counter. Otherwise the figure will produce an error.
///
/// ## Counter and supplement
/// Based on the `of` parameter or the detected content, the figure will chose
/// the appropriate counter and supplement. These can be overridden by using the
/// `counter` and `supplement` parameters respectively.
///
/// The overriding of these values is done as follows:
/// ```example
/// #figure(caption: [ Hello, world! ], counter: counter("my_counter"), supplement: "Molecule")[
///   #image("molecular.jpg", width: 32pt)
/// ]
/// ```
///
/// The default counters are defined as follows:
/// - for (tables)[$func/table]: `counter(figure.where(of: table))`
/// - for (raw text)[$func/raw]: `counter(figure.where(of: raw))`
/// - for (images)[$func/image]: `counter(figure.where(of: image))`
///
/// These are the counters you need to use if you want to change the
/// counting behaviour of figures.
///
/// ## Numbering
/// By default, the figure will be numbered using the `1` [numbering pattern]($func/numbering).
/// This can be overridden by using the `numbering` parameter.
///
/// ## Listing
/// By default, the figure will be listed in the list of figures/tables/code. This can be disabled by
/// setting the `listed` parameter to `false`.
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
#[element(Locatable, Synthesize, Count, Show, Refable)]
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
    pub outlined: bool,

    /// How to number the figure. Accepts a
    /// [numbering pattern or function]($func/numbering).
    #[default(Some(NumberingPattern::from_str("1").unwrap().into()))]
    pub numbering: Option<Numbering>,

    /// The type of the figure.
    /// Setting this will override the automatic detection.
    #[default(Smart::Auto)]
    pub contents: Smart<ContentParam>,

    /// The vertical gap between the body and caption.
    #[default(Em::new(0.65).into())]
    pub gap: Length,

    /// The element to use for the figure's properties.
    #[synthesized]
    #[internal]
    element: Option<FigureContent>,
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
        self.body().query(|content| content.func() == func).first().cloned()
    }

    pub fn show_supplement_and_numbering(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        external_supp: Option<Content>,
    ) -> SourceResult<Option<Content>> {
        if let Some(numbering) = self.numbering(styles) {
            let element = self.element().expect("missing element");

            let mut name = if let Some(supp) = external_supp {
                supp
            } else {
                element.supplement.resolve(vt, [element.content.into()])?
            };

            let counter = element.counter;

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
        self.push_numbering(self.numbering(styles));

        // we get the numbering or `None`.
        let numbering = self.numbering(styles);

        // we get the content or `None`.
        let content = match self.contents(styles) {
            Smart::Auto => match self.determine_type(styles ){
                Some(ty) => Some(ty),
                None => bail!(self.span(), "unable to determine figure type, use `contents` to manually specify it"),
            },
            Smart::Custom(ContentParam::Elem(ty)) => self.find_elem(ty),
            Smart::Custom(ContentParam::Name(_)) => None,
        };

        if self.contents(styles).is_auto() {
            if let Some(content) = &content {
                self.push_contents(Smart::Custom(ContentParam::Elem(content.func())));
            }
        }

        // we get the counter or `None`.
        let counter = if let Some(content) = &content {
            Some((
                Counter::new(CounterKey::Selector(Selector::Elem(
                    Self::func(),
                    Some(dict! {
                        "contents" => Value::from(content.func()),
                    }),
                ))),
                false,
            ))
        } else if let Smart::Custom(ContentParam::Name(name)) = self.contents(styles) {
            Some((
                Counter::new(CounterKey::Selector(Selector::Elem(
                    Self::func(),
                    Some(dict! {
                        "contents" => Value::from(name),
                    }),
                ))),
                false,
            ))
        } else if let Smart::Custom(ContentParam::Elem(func)) = self.contents(styles) {
            Some((
                Counter::new(CounterKey::Selector(Selector::Elem(
                    Self::func(),
                    Some(dict! {
                        "contents" => Value::from(func),
                    }),
                ))),
                false,
            ))
        } else {
            None
        };

        // we get the supplement or `None`.
        let supplement = match self.supplement(styles) {
            Smart::Auto => {
                if let Some(figurable) =
                    content.as_ref().and_then(|c| c.with::<dyn Figurable>())
                {
                    Some(figurable.supplement(styles))
                } else {
                    None
                }
            }
            Smart::Custom(supp) => supp,
        };

        if let Some(numbering) = numbering {
            let Some((counter, update_counter)) = counter else {
                bail!(self.span(), "numbering a figure requires that is has a counter");
            };

            let Some(supplement) = supplement else {
                bail!(self.span(), "numbering a figure requires that is has a supplement");
            };

            self.push_element(Some(FigureContent {
                numbering,
                counter,
                update_counter,
                supplement,
                content: content.unwrap_or_else(|| self.body()),
            }))
        } else {
            self.push_element(None);
        }

        Ok(())
    }
}

impl Show for FigureElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let counter_update = if let Some(element) = self.element() {
            element
                .update_counter
                .then(|| element.counter.update(CounterUpdate::Step(NonZeroUsize::ONE)))
                .unwrap_or_else(Content::empty)
        } else {
            Content::empty()
        };

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
        // if the figure is not outlined, it is not referenced.
        if !self.outlined(styles) {
            return Ok(None);
        }

        self.show_caption(vt, styles).map(Some)
    }
}

#[derive(Debug, Clone)]
pub enum ContentParam {
    Elem(ElemFunc),
    Name(EcoString),
}

cast_from_value! {
    ContentParam,
    v: ElemFunc => Self::Elem(v),
    v: EcoString => Self::Name(v),
}

cast_to_value! {
    v: ContentParam => match v {
        ContentParam::Elem(v) => v.into(),
        ContentParam::Name(v) => v.into(),
    }
}

struct FigureContent {
    numbering: Numbering,
    counter: Counter,
    update_counter: bool,
    supplement: Supplement,
    content: Content,
}

cast_to_value! {
    v: FigureContent => dict! {
        "numbering" => Value::from(v.numbering),
        "counter" => Value::from(v.counter),
        "update_counter" => Value::from(v.update_counter),
        "supplement" => Value::from(v.supplement),
        "content" => Value::from(v.content),
    }.into()
}

cast_from_value! {
    FigureContent,
    v: Dict => {
        let numbering = v
            .at("numbering")
            .cloned()
            .map(Numbering::cast)??;

        let counter = v
            .at("counter")
            .cloned()
            .map(Counter::cast)??;

        let update_counter = v
            .at("update_counter")
            .cloned()
            .map(bool::cast)??;

        let supplement = v
            .at("supplement")
            .cloned()
            .map(Supplement::cast)??;

        let content = v
            .at("content")
            .cloned()
            .map(Content::cast)??;

        Self { numbering, update_counter, counter, supplement, content }
    }
}

/// An element that can be placed in a figure.
/// This trait is used to determine the type of a figure, it counter, its numbering pattern
/// and the supplement to use for referencing it and creating the citation.
/// The element chosen as the figure's content is the one with the highest priority.
pub trait Figurable {
    /// The supplement to use for referencing the figure.
    fn supplement(&self, styles: StyleChain) -> Supplement;

    /// The priority of this element.
    fn priority(&self, styles: StyleChain) -> isize;
}
