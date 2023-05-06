use std::str::FromStr;

use super::{
    Count, Counter, CounterKey, CounterUpdate, LocalName, Numbering, NumberingPattern,
};
use crate::layout::{BlockElem, VElem};
use crate::meta::{Refable, Supplement};
use crate::prelude::*;
use crate::text::TextElem;
use crate::visualize::ImageElem;

/// A figure with an optional caption.
///
/// Automatically detects its contents to select the correct counting track.
/// For example, figures containing images will be numbered separately from
/// figures containing tables.
///
/// ## Examples
/// The example below shows a basic figure with an image:
/// ```example
/// @glacier shows a glacier. Glaciers
/// are complex systems.
///
/// #figure(
///   image("glacier.jpg", width: 80%),
///   caption: [A curious figure.],
/// ) <glacier>
/// ```
///
/// You can also insert [tables]($func/table) into figures to give them a
/// caption. The figure will detect this and automatically use a separate
/// counter.
///
/// ```example
/// #figure(
///   table(
///     columns: 4,
///     [t], [1], [2], [3],
///     [y], [0.3s], [0.4s], [0.8s],
///   ),
///   caption: [Timing results],
/// )
/// ```
///
/// This behaviour can be overridden by explicitly specifying the figure's
/// `kind`. All figures of the same kind share a common counter.
///
/// ## Modifying the appearance
/// You can completely customize the look of your figures with a [show
/// rule]($styling/#show-rules). In the example below, we show the figure's
/// caption above its body and display its supplement and counter after the
/// caption.
///
/// ```example
/// #show figure: it => align(center)[
///   #it.caption |
///   #emph[
///     #it.supplement
///     #it.counter.display(it.numbering)
///   ]
///   #v(10pt, weak: true)
///   #it.body
/// ]
///
/// #figure(
///   image("molecular.jpg", width: 80%),
///   caption: [
///     The molecular testing pipeline.
///   ],
/// )
/// ```
///
/// Display: Figure
/// Category: meta
#[element(Locatable, Synthesize, Count, Show, Finalize, Refable)]
pub struct FigureElem {
    /// The content of the figure. Often, an [image]($func/image).
    #[required]
    pub body: Content,

    /// The figure's caption.
    pub caption: Option<Content>,

    /// The kind of the figure this is.
    ///
    /// If set to `{auto}`, the figure will try to automatically determine its
    /// kind. All figures of the same kind share a common counter.
    ///
    /// Setting this to something other than `{auto}` will override the
    /// automatic detection. This can be useful if
    /// - you wish to create a custom figure type that is not an
    ///   [image]($func/image), a [table]($func/table) or [code]($func/raw),
    /// - you want to force the figure to use a counter regardless of its
    ///   content.
    ///
    /// You can set the kind to be an element function or a string. If you set
    /// it to an element function that is not supported by the figure, you will
    /// need to manually specify the figure's supplement.
    ///
    /// The figure's automatic detection is based on a priority list to select
    /// the element that is likely to be the most important one. If the figure's
    /// body contains multiple valid elements, the one with the highest priority
    /// is selected. The priority list is as follows:
    /// - [image]($func/image) is the most important,
    /// - [code]($func/raw) is the second most important,
    /// - [table]($func/table) is the least important one.
    ///
    /// ```example
    /// #figure(
    ///   circle(radius: 10pt),
    ///   caption: [A curious atom.],
    ///   kind: "atom",
    ///   supplement: [Atom],
    /// )
    /// ```
    #[default(Smart::Auto)]
    pub kind: Smart<FigureKind>,

    /// The figure's supplement.
    ///
    /// If set to `{auto}`, the figure will try to automatically determine the
    /// correct supplement based on the `kind` and the active [text
    /// language]($func/text.lang). If you are using a custom figure type, you
    /// will need to manually specify the supplement.
    ///
    /// This can also be set to a function that receives the figure's body to
    /// select the supplement based on the figure's contents.
    ///
    /// ```example
    /// #figure(
    ///   [The contents of my figure!],
    ///   caption: [My custom figure],
    ///   supplement: [Bar],
    ///   kind: "foo",
    /// )
    /// ```
    #[default(Smart::Auto)]
    pub supplement: Smart<Supplement>,

    /// How to number the figure. Accepts a
    /// [numbering pattern or function]($func/numbering).
    ///
    /// Defaults to `{"1"}`.
    #[default(Some(NumberingPattern::from_str("1").unwrap().into()))]
    pub numbering: Option<Numbering>,

    /// The vertical gap between the body and caption.
    #[default(Em::new(0.65).into())]
    pub gap: Length,

    /// Whether the figure should appear in an [`outline`]($func/outline)
    /// of figures.
    ///
    /// Defaults to `{true}`.
    #[default(true)]
    pub outlined: bool,

    /// Convenience field to get access to the counter for this figure.
    ///
    /// The counter only depends on the `kind`:
    /// - For (tables)[$func/table]: `{counter(figure.where(kind: table))}`
    /// - For (images)[$func/image]: `{counter(figure.where(kind: image))}`
    /// - For a custom kind: `{counter(figure.where(kind: kind))}`
    ///
    /// These are the counters you'll need to modify if you want to skip a
    /// number or reset the counter.
    #[synthesized]
    pub counter: Option<Counter>,
}

impl Synthesize for FigureElem {
    fn synthesize(&mut self, vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        // Determine the figure's kind.
        let kind = match self.kind(styles) {
            Smart::Auto => self
                .find_figurable(styles)
                .map(|elem| FigureKind::Elem(elem.func()))
                .unwrap_or_else(|| FigureKind::Elem(ImageElem::func())),
            Smart::Custom(kind) => kind,
        };

        let content = match &kind {
            FigureKind::Elem(func) => self.find_of_elem(*func),
            FigureKind::Name(_) => None,
        }
        .unwrap_or_else(|| self.body());

        let numbering = self.numbering(styles);

        // We get the supplement or `None`. The supplement must either be set
        // manually or the content identification must have succeeded.
        let supplement = match self.supplement(styles) {
            Smart::Auto => match &kind {
                FigureKind::Elem(func) => {
                    let elem = Content::new(*func).with::<dyn LocalName>().map(|c| {
                        TextElem::packed(c.local_name(
                            TextElem::lang_in(styles),
                            TextElem::region_in(styles),
                        ))
                    });

                    if numbering.is_some() {
                        Some(elem
                            .ok_or("unable to determine the figure's `supplement`, please specify it manually")
                            .at(self.span())?)
                    } else {
                        elem
                    }
                }
                FigureKind::Name(_) => {
                    if numbering.is_some() {
                        bail!(self.span(), "please specify the figure's supplement")
                    } else {
                        None
                    }
                }
            },
            Smart::Custom(supp) => Some(supp.resolve(vt, [content.into()])?),
        };

        // Construct the figure's counter.
        let counter = Counter::new(CounterKey::Selector(Selector::Elem(
            Self::func(),
            Some(dict! {
                "kind" => kind.clone(),
            }),
        )));

        self.push_caption(self.caption(styles));
        self.push_kind(Smart::Custom(kind));
        self.push_supplement(Smart::Custom(Supplement::Content(
            supplement.unwrap_or_default(),
        )));
        self.push_numbering(numbering);
        self.push_outlined(self.outlined(styles));
        self.push_counter(Some(counter));

        Ok(())
    }
}

impl Show for FigureElem {
    #[tracing::instrument(name = "FigureElem::show", skip_all)]
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        // We build the body of the figure.
        let mut realized = self.body();

        // We build the caption, if any.
        if self.caption(styles).is_some() {
            realized += VElem::weak(self.gap(styles).into()).pack();
            realized += self.show_caption(vt)?;
        }

        // We wrap the contents in a block.
        Ok(BlockElem::new()
            .with_body(Some(realized))
            .pack()
            .aligned(Axes::with_x(Some(Align::Center.into()))))
    }
}

impl Finalize for FigureElem {
    fn finalize(&self, realized: Content, _: StyleChain) -> Content {
        // Allow breakable figures with `show figure: set block(breakable: true)`.
        realized
            .styled(BlockElem::set_breakable(false))
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
        supplement: Option<Content>,
        _: Lang,
        _: Option<Region>,
    ) -> SourceResult<Content> {
        // If the figure is not numbered, we cannot reference it.
        // Otherwise we build the supplement and numbering scheme.
        let Some(desc) = self.show_supplement_and_numbering(vt, supplement)? else {
            bail!(self.span(), "cannot reference unnumbered figure")
        };

        Ok(desc)
    }

    fn outline(
        &self,
        vt: &mut Vt,
        _: Lang,
        _: Option<Region>,
    ) -> SourceResult<Option<Content>> {
        // If the figure is not outlined, it is not referenced.
        if !self.outlined(StyleChain::default()) {
            return Ok(None);
        }

        self.show_caption(vt).map(Some)
    }

    fn numbering(&self) -> Option<Numbering> {
        self.numbering(StyleChain::default())
    }

    fn counter(&self) -> Counter {
        self.counter().unwrap_or_else(|| Counter::of(Self::func()))
    }
}

impl FigureElem {
    /// Determines the type of the figure by looking at the content, finding all
    /// [`Figurable`] elements and sorting them by priority then returning the highest.
    pub fn find_figurable(&self, styles: StyleChain) -> Option<Content> {
        self.body()
            .query(Selector::can::<dyn Figurable>())
            .into_iter()
            .max_by_key(|elem| elem.with::<dyn Figurable>().unwrap().priority(styles))
            .cloned()
    }

    /// Finds the element with the given function in the figure's content.
    /// Returns `None` if no element with the given function is found.
    pub fn find_of_elem(&self, func: ElemFunc) -> Option<Content> {
        self.body()
            .query(Selector::Elem(func, None))
            .into_iter()
            .next()
            .cloned()
    }

    /// Builds the supplement and numbering of the figure.
    /// If there is no numbering, returns [`None`].
    ///
    /// # Errors
    /// If a numbering is specified but the [`Self::data()`] is `None`.
    pub fn show_supplement_and_numbering(
        &self,
        vt: &mut Vt,
        external_supplement: Option<Content>,
    ) -> SourceResult<Option<Content>> {
        if let (Some(numbering), Some(supplement), Some(counter)) = (
            self.numbering(StyleChain::default()),
            self.supplement(StyleChain::default())
                .as_custom()
                .and_then(|s| s.as_content()),
            self.counter(),
        ) {
            let mut name = external_supplement.unwrap_or(supplement);
            if !name.is_empty() {
                name += TextElem::packed("\u{a0}");
            }

            let number = counter
                .at(vt, self.0.location().unwrap())?
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
    pub fn show_caption(&self, vt: &mut Vt) -> SourceResult<Content> {
        let Some(mut caption) = self.caption(StyleChain::default()) else {
            return Ok(Content::empty());
        };

        if let Some(sup_and_num) = self.show_supplement_and_numbering(vt, None)? {
            caption = sup_and_num + TextElem::packed(": ") + caption;
        }

        Ok(caption)
    }
}

/// The `kind` parameter of a [`FigureElem`].
#[derive(Debug, Clone)]
pub enum FigureKind {
    /// The kind is an element function.
    Elem(ElemFunc),
    /// The kind is a name.
    Name(EcoString),
}

cast_from_value! {
    FigureKind,
    v: ElemFunc => Self::Elem(v),
    v: EcoString => Self::Name(v),
}

cast_to_value! {
    v: FigureKind => match v {
        FigureKind::Elem(v) => v.into(),
        FigureKind::Name(v) => v.into(),
    }
}

/// An element that can be auto-detected in a figure.
///
/// This trait is used to determine the type of a figure. The element chosen as
/// the figure's content is the figurable descendant with the highest priority.
pub trait Figurable: LocalName {
    /// The priority of this element.
    fn priority(&self, styles: StyleChain) -> isize;
}
