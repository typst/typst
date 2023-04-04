use std::any::TypeId;
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
/// - [equations]($func/equation) are the second most important
/// - [code]($func/raw) are the third most important
/// - [table]($func/table) are the fourth most important.
///
/// There can be a variety of content within a figure and only the first element
/// of the most important category will be used. For example, if a figure contains
/// an image and a table, the image will be used. This behaviour can be overridden
/// using the `kind` parameter. By setting it, you can force the figure to use a
/// specific type of content. Note however that if the figure does not contain said
/// element, or the `kind` is set to a string, you will need to manually specify
/// the supplement to be able to make an outline or reference it.
///
/// ```example
/// #figure(caption: [ Hello, world! ], kind: table)[
///   #table(
///    columns: (auto, 1fr),
///    image("molecular.jpg", width: 32pt),
///    [ A first picture ],
///    image("molecular.jpg", width: 32pt),
///    [ A second picture ],
///   )
/// ]
/// ```
///
/// If you use an element that is not supported by the figure, and set it as its `content` parameter,
/// to be able to make an outline or reference it, you will need to manually specify the supplement
/// and counter. Otherwise the figure will produce an error.
///
/// ## Counting and supplement
/// Based on the `kind` parameter or the detected content, the figure will chose
/// the appropriate counter and supplement. These can be overridden by using the
/// `kind` and `supplement` parameters respectively.
///
/// The overriding of these values is done as follows:
/// ```example
/// #figure(caption: [ Hello, world! ], kind: "hello", supplement: "Molecule")[
///   #image("molecular.jpg", width: 32pt)
/// ]
/// ```
///
/// The default counters are defined as follows:
/// - for (tables)[$func/table]: `counter(figure.where(kind: table))`
/// - for (equations)[$func/equation]: `counter(figure.where(kind: math.equation))`
/// - for (raw text)[$func/raw]: `counter(figure.where(kind: raw))`
/// - for (images)[$func/image]: `counter(figure.where(kind: image))`
/// - for a custom kind: `counter(figure.where(kind: kind))`
///
/// These are the counters you need to use if you want to change the
/// counting behaviour of figures.
///
/// ## Numbering
/// By default, the figure will be numbered using the `1` [numbering pattern]($func/numbering).
/// This can be overridden by using the `numbering` parameter.
///
/// ## Outline
/// By default, the figure will be outlined in the list of figures/tables/code. This can be disabled by
/// setting the `outlined` parameter to `false`.
///
/// ## Global figure counter
/// There is a global figure counter which can be accessed which counts all numbered figures in the document
/// regardless of its type. This counter can be accessed using the `counter(figure)` function.
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
    ///
    /// ## Custom figure type
    /// If you are using a custom figure type and would like to figure to be
    /// referenced, you will need to manually specify the supplement, using either
    /// a function or a string.
    ///
    /// ```example
    /// #figure(caption: "My custom figure", kind: "foo", supplement: "Bar")[
    ///   #block[ The inside of my custom figure! ]
    /// ]
    /// ```
    #[default(Smart::Auto)]
    pub supplement: Smart<Option<Supplement>>,

    /// Whether the figure should appear in the list of figures/tables/code.
    /// Defaults to `true`.
    #[default(true)]
    pub outlined: bool,

    /// How to number the figure. Accepts a
    /// [numbering pattern or function]($func/numbering).
    #[default(Some(NumberingPattern::from_str("1").unwrap().into()))]
    pub numbering: Option<Numbering>,

    /// The type of the figure. Setting this will override the automatic detection.
    ///
    /// This can be useful if you wish to create a custom figure type that is not
    /// an [image]($func/image), a [table]($func/table) or a [code]($func/raw). Or if
    /// you want to force the figure to use a specific type regardless of its content.
    ///
    /// You can set the kind to be an element, or a string. If you set it to be
    /// a string or an element that is not supported by the figure, you will need to
    /// manually specify the supplement if you wish to number the figure.
    #[default(Smart::Auto)]
    pub kind: Smart<ContentParam>,

    /// The vertical gap between the body and caption.
    #[default(Em::new(0.65).into())]
    pub gap: Length,

    /// The detailed numbering information for the figure.
    #[synthesized]
    #[internal]
    pub element: Option<FigureKind>,
}

impl FigureElem {
    /// Determines the type of the figure by looking at the content, finding all
    /// [`Figurable`] elements and sorting them by priority then returning the highest.
    pub fn determine_type(&self, styles: StyleChain) -> Option<Content> {
        let potential_elems =
            self.body().query(Selector::Can(TypeId::of::<dyn Figurable>()));

        potential_elems.into_iter().max_by_key(|elem| {
            elem.with::<dyn Figurable>()
                .expect("should be figurable")
                .priority(styles)
        })
    }

    /// Finds the element with the given function in the figure's content.
    /// Returns `None` if no element with the given function is found.
    pub fn find_elem(&self, func: ElemFunc) -> Option<Content> {
        self.body().query(Selector::Elem(func, None)).first().cloned()
    }

    /// Builds the supplement and numbering of the figure.
    /// If there is no numbering, returns [`None`].
    ///
    /// # Errors
    /// If a numbering is specified but the [`Self::element`] is `None`.
    pub fn show_supplement_and_numbering(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        external_supp: Option<Content>,
    ) -> SourceResult<Option<Content>> {
        if let Some(numbering) = self.numbering(styles) {
            let element = self.element().expect("missing element");

            let mut name = external_supp.unwrap_or(element.supplement);

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

    /// Builds the caption for the figure.
    /// If there is a numbering, will also try to show the supplement and the numbering.
    ///
    /// # Errors
    /// If a numbering is specified but the [`Self::element`] is `None`.
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
    fn synthesize(&mut self, vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        self.push_numbering(self.numbering(styles));

        // We get the numbering or `None`.
        let numbering = self.numbering(styles);

        // We get the content or `None`.
        let content = match self.kind(styles) {
            Smart::Auto => match self.determine_type(styles) {
                Some(ty) => Some(ty),
                None => bail!(
                    self.span(),
                    "unable to determine figure type, use `kind` to manually specify it"
                ),
            },
            Smart::Custom(ContentParam::Elem(ty)) => self.find_elem(ty),
            Smart::Custom(ContentParam::Name(_)) => None,
        };

        if self.kind(styles).is_auto() {
            if let Some(content) = &content {
                self.push_kind(Smart::Custom(ContentParam::Elem(content.func())));
            }
        }

        // The list of choices is the following:
        // 1. If there is a detected content, we use the counter `counter(figure.where(kind: detected_content))`
        // 2. If there is a name/elem, we use the counter `counter(figure.where(kind: name/elem))`
        // 4. We return None.
        let counter = if let Some(content) = &content {
            Some(Counter::new(CounterKey::Selector(Selector::Elem(
                Self::func(),
                Some(dict! {
                    "kind" => Value::from(content.func()),
                }),
            ))))
        } else if let Smart::Custom(content) = self.kind(styles) {
            Some(Counter::new(CounterKey::Selector(Selector::Elem(
                Self::func(),
                Some(dict! {
                    "kind" => Value::from(content),
                }),
            ))))
        } else {
            None
        };

        // We get the supplement or `None`.
        // The supplement must either be set manually of the content identification
        // must have succeeded.
        let supplement = match self.supplement(styles) {
            Smart::Auto => {
                content.as_ref().and_then(|c| c.with::<dyn LocalName>()).map(|c| {
                    Supplement::Content(TextElem::packed(
                        c.local_name(TextElem::lang_in(styles)),
                    ))
                })
            }
            Smart::Custom(supp) => supp,
        };

        // We the user wishes to number their figure, we check whether there is a
        // counter and a supplement. If so, we push the element, which is just a
        // summary of the caption properties
        if let Some(numbering) = numbering {
            let Some(counter) = counter else {
                bail!(self.span(), "numbering a figure requires that is has a counter");
            };

            let Some(supplement) = supplement else {
                bail!(self.span(), "numbering a figure requires that is has a supplement");
            };

            let supplement = supplement
                .resolve(vt, [content.unwrap_or_else(|| self.body()).into()])?;

            self.push_element(Some(FigureKind { numbering, counter, supplement }))
        } else {
            self.push_element(None);
        }

        Ok(())
    }
}

impl Show for FigureElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        // We build the body of the figure.
        let mut realized = self.body();

        // We build the caption, if any.
        if self.caption(styles).is_some() {
            realized += VElem::weak(self.gap(styles).into()).pack();
            realized += self.show_caption(vt, styles)?;
        }

        // We wrap the contents in a block.
        Ok(BlockElem::new()
            .with_body(Some(realized))
            .with_breakable(false)
            .pack()
            .aligned(Axes::with_x(Some(Align::Center.into()))))
    }
}

impl Count for FigureElem {
    fn update(&self) -> Option<CounterUpdate> {
        // If the figure is numbered, step the counter by one.
        // This steps the `counter(figure)` which is global to all numbered figures.
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
        // If the figure is not numbered, we cannot reference it.
        // Otherwise we build the supplement and numbering scheme.
        let Some(desc) = self.show_supplement_and_numbering(vt, styles, supplement)? else {
            bail!(self.span(), "cannot reference unnumbered figure")
        };

        Ok(desc)
    }

    fn location(&self) -> Option<Location> {
        self.0.location()
    }

    fn outline(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Option<Content>> {
        // If the figure is not outlined, it is not referenced.
        if !self.outlined(styles) {
            return Ok(None);
        }

        self.show_caption(vt, styles).map(Some)
    }
}

/// The `kind` parameter of [`FigureElem`].
#[derive(Debug, Clone)]
pub enum ContentParam {
    /// The content is an element function.
    Elem(ElemFunc),

    /// The content is a name.
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

/// The state needed to build the numbering of a figure.
pub struct FigureKind {
    /// The numbering scheme.
    numbering: Numbering,

    /// The counter to use.
    counter: Counter,

    /// The supplement to use.
    supplement: Content,
}

cast_to_value! {
    v: FigureKind => dict! {
        "numbering" => Value::from(v.numbering),
        "counter" => Value::from(v.counter),
        "supplement" => Value::from(v.supplement),
    }.into()
}

cast_from_value! {
    FigureKind,
    v: Dict => {
        let numbering = v
            .at("numbering")
            .cloned()
            .map(Numbering::cast)??;

        let counter = v
            .at("counter")
            .cloned()
            .map(Counter::cast)??;

        let supplement = v
            .at("supplement")
            .cloned()
            .map(Content::cast)??;

        Self { numbering, counter, supplement }
    }
}

/// An element that can be autodetected in a figure.
/// This trait is used to determine the type of a figure, its counter, its numbering pattern
/// and the supplement to use for referencing it and creating the caption.
/// The element chosen as the figure's content is the one with the highest priority.
pub trait Figurable {
    /// The priority of this element.
    fn priority(&self, styles: StyleChain) -> isize;
}
