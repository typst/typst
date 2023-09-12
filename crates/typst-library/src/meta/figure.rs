use std::str::FromStr;

use super::{
    Count, Counter, CounterKey, CounterUpdate, LocalName, Numbering, NumberingPattern,
};
use crate::layout::{BlockElem, PlaceElem, VElem};
use crate::meta::{Outlinable, Refable, Supplement};
use crate::prelude::*;
use crate::text::TextElem;
use crate::visualize::ImageElem;

/// A figure with an optional caption.
///
/// Automatically detects its contents to select the correct counting track.
/// For example, figures containing images will be numbered separately from
/// figures containing tables.
///
/// # Examples
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
/// You can also insert [tables]($table) into figures to give them a caption.
/// The figure will detect this and automatically use a separate counter.
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
/// # Modifying the appearance { #modifying-appearance }
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
/// If your figure is too large and its contents are breakable across pages
/// (e.g. if it contains a large table), then you can make the figure breakable
/// across pages as well by using `[#show figure: set block(breakable: true)]`
/// (see the [block]($block) documentation for more information).
#[elem(scope, Locatable, Synthesize, Count, Show, Finalize, Refable, Outlinable)]
pub struct FigureElem {
    /// The content of the figure. Often, an [image]($image).
    #[required]
    pub body: Content,

    /// The figure's placement on the page.
    ///
    /// - `{none}`: The figure stays in-flow exactly where it was specified
    ///   like other content.
    /// - `{auto}`: The figure picks `{top}` or `{bottom}` depending on which
    ///   is closer.
    /// - `{top}`: The figure floats to the top of the page.
    /// - `{bottom}`: The figure floats to the bottom of the page.
    ///
    /// ```example
    /// #set page(height: 200pt)
    ///
    /// = Introduction
    /// #figure(
    ///   placement: bottom,
    ///   caption: [A glacier],
    ///   image("glacier.jpg", width: 60%),
    /// )
    /// #lorem(60)
    /// ```
    pub placement: Option<Smart<VAlign>>,

    /// The figure's caption.
    pub caption: Option<Content>,

    /// The caption's position. Either `{top}` or `{bottom}`.
    ///
    /// ```example
    /// #figure(
    ///   table(columns: 2)[A][B],
    ///   caption: [I'm up here],
    ///   caption-pos: top,
    /// )
    ///
    /// #figure(
    ///   table(columns: 2)[A][B],
    ///   caption: [I'm down here],
    /// )
    /// ```
    #[default(VAlign::Bottom)]
    #[parse({
        let option: Option<Spanned<VAlign>> = args.named("caption-pos")?;
        if let Some(Spanned { v: align, span }) = option {
            if align == VAlign::Horizon {
                bail!(span, "expected `top` or `bottom`");
            }
        }
        option.map(|spanned| spanned.v)
    })]
    pub caption_pos: VAlign,

    /// The kind of figure this is.
    ///
    /// If set to `{auto}`, the figure will try to automatically determine its
    /// kind. All figures of the same kind share a common counter.
    ///
    /// Setting this to something other than `{auto}` will override the
    /// automatic detection. This can be useful if
    /// - you wish to create a custom figure type that is not an
    ///   [image]($image), a [table]($table) or [code]($raw),
    /// - you want to force the figure to use a specific counter regardless of
    ///   its content.
    ///
    /// You can set the kind to be an element function or a string. If you set
    /// it to an element function that is not supported by the figure, you will
    /// need to manually specify the figure's supplement.
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
    /// correct supplement based on the `kind` and the active
    /// [text language]($text.lang). If you are using a custom figure type, you
    /// will need to manually specify the supplement.
    ///
    /// If a function is specified, it is passed the first descendant of the
    /// specified `kind` (typically, the figure's body) and should return
    /// content.
    ///
    /// ```example
    /// #figure(
    ///   [The contents of my figure!],
    ///   caption: [My custom figure],
    ///   supplement: [Bar],
    ///   kind: "foo",
    /// )
    /// ```
    pub supplement: Smart<Option<Supplement>>,

    /// How to number the figure. Accepts a
    /// [numbering pattern or function]($numbering).
    #[default(Some(NumberingPattern::from_str("1").unwrap().into()))]
    pub numbering: Option<Numbering>,

    /// The vertical gap between the body and caption.
    #[default(Em::new(0.65).into())]
    pub gap: Length,

    /// Whether the figure should appear in an [`outline`]($outline) of figures.
    #[default(true)]
    pub outlined: bool,

    /// Convenience field to get access to the counter for this figure.
    ///
    /// The counter only depends on the `kind`:
    /// - For (tables)[@table]: `{counter(figure.where(kind: table))}`
    /// - For (images)[@image]: `{counter(figure.where(kind: image))}`
    /// - For a custom kind: `{counter(figure.where(kind: kind))}`
    ///
    /// These are the counters you'll need to modify if you want to skip a
    /// number or reset the counter.
    #[synthesized]
    pub counter: Option<Counter>,
}

#[scope]
impl FigureElem {
    #[elem]
    type FigureCaption;
}

impl Synthesize for FigureElem {
    fn synthesize(&mut self, vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        let numbering = self.numbering(styles);

        // Determine the figure's kind.
        let kind = self.kind(styles).unwrap_or_else(|| {
            self.body()
                .query_first(Selector::can::<dyn Figurable>())
                .cloned()
                .map(|elem| FigureKind::Elem(elem.func()))
                .unwrap_or_else(|| FigureKind::Elem(ImageElem::elem()))
        });

        // Resolve the supplement.
        let supplement = match self.supplement(styles) {
            Smart::Auto => {
                // Default to the local name for the kind, if available.
                let name = match &kind {
                    FigureKind::Elem(func) => {
                        let empty = Content::new(*func);
                        empty.with::<dyn LocalName>().map(|c| {
                            TextElem::packed(c.local_name(
                                TextElem::lang_in(styles),
                                TextElem::region_in(styles),
                            ))
                        })
                    }
                    FigureKind::Name(_) => None,
                };

                if numbering.is_some() && name.is_none() {
                    bail!(self.span(), "please specify the figure's supplement")
                }

                name.unwrap_or_default()
            }
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(supplement)) => {
                // Resolve the supplement with the first descendant of the kind or
                // just the body, if none was found.
                let descendant = match kind {
                    FigureKind::Elem(func) => {
                        self.body().query_first(Selector::Elem(func, None)).cloned()
                    }
                    FigureKind::Name(_) => None,
                };

                let target = descendant.unwrap_or_else(|| self.body());
                supplement.resolve(vt, [target])?
            }
        };

        // Construct the figure's counter.
        let counter = Counter::new(CounterKey::Selector(Selector::Elem(
            Self::elem(),
            Some(dict! {
                "kind" => kind.clone(),
            }),
        )));

        self.push_placement(self.placement(styles));
        self.push_caption_pos(self.caption_pos(styles));
        self.push_caption(self.caption(styles));
        self.push_kind(Smart::Custom(kind));
        self.push_supplement(Smart::Custom(Some(Supplement::Content(supplement))));
        self.push_numbering(numbering);
        self.push_outlined(self.outlined(styles));
        self.push_counter(Some(counter));

        Ok(())
    }
}

impl Show for FigureElem {
    #[tracing::instrument(name = "FigureElem::show", skip_all)]
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();

        // Build the caption, if any.
        if let Some(caption_body) = self.caption(StyleChain::default()) {
            let supplement = self.supplement(StyleChain::default()).unwrap_or_default();
            let loc = self.0.location().unwrap();
            let numbers = if let (Some(counter), Some(numbering)) =
                (self.counter(), self.numbering(StyleChain::default()))
            {
                Some(counter.at(vt, loc)?.display(vt, &numbering)?)
            } else {
                None
            };
            let caption = FigureCaption::new(caption_body, supplement, numbers).pack();
            let v = VElem::weak(self.gap(styles).into()).pack();
            realized = if self.caption_pos(styles) == VAlign::Bottom {
                realized + v + caption
            } else {
                caption + v + realized
            }
        }

        // Wrap the contents in a block.
        realized = BlockElem::new()
            .with_body(Some(realized))
            .pack()
            .aligned(Align::CENTER);

        // Wrap in a float.
        if let Some(align) = self.placement(styles) {
            realized = PlaceElem::new(realized)
                .with_float(true)
                .with_alignment(align.map(|align| HAlign::Center + align))
                .pack();
        }

        Ok(realized)
    }
}

impl Finalize for FigureElem {
    fn finalize(&self, realized: Content, _: StyleChain) -> Content {
        // Allow breakable figures with `show figure: set block(breakable: true)`.
        realized.styled(BlockElem::set_breakable(false))
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
    fn supplement(&self) -> Content {
        // After synthesis, this should always be custom content.
        match self.supplement(StyleChain::default()) {
            Smart::Custom(Some(Supplement::Content(content))) => content,
            _ => Content::empty(),
        }
    }

    fn counter(&self) -> Counter {
        self.counter().unwrap_or_else(|| Counter::of(Self::elem()))
    }

    fn numbering(&self) -> Option<Numbering> {
        self.numbering(StyleChain::default())
    }
}

impl Outlinable for FigureElem {
    fn outline(&self, vt: &mut Vt) -> SourceResult<Option<Content>> {
        if !self.outlined(StyleChain::default()) {
            return Ok(None);
        }

        let Some(mut caption) = self.caption(StyleChain::default()) else {
            return Ok(None);
        };

        if let (
            Smart::Custom(Some(Supplement::Content(mut supplement))),
            Some(counter),
            Some(numbering),
        ) = (
            self.supplement(StyleChain::default()),
            self.counter(),
            self.numbering(StyleChain::default()),
        ) {
            let location = self.0.location().unwrap();
            let numbers = counter.at(vt, location)?.display(vt, &numbering)?;

            if !supplement.is_empty() {
                supplement += TextElem::packed("\u{a0}");
            }

            caption = supplement + numbers + TextElem::packed(": ") + caption;
        }

        Ok(Some(caption))
    }
}

/// The caption of a figure, including the supplement and counter.
///
/// This element is intended to be used in show rules to customize the
/// appearance of the whole caption for all figure kinds.
///
/// ```example
/// #show figure.caption: emph
///
/// #figure(
///   rect(),
///   caption: [A rectangle],
/// )
/// ```
///
/// When using a show rule for figures, you can wrap the full caption in a
/// `{figure.caption}` element if you want it to be styled as other captions.
#[elem(name = "caption", Show)]
pub struct FigureCaption {
    /// The caption's body.
    #[required]
    pub body: Content,

    /// The caption's supplement.
    #[required]
    pub supplement: Option<Supplement>,

    /// The caption's numbers, evaluated from the corresponding figure's
    /// counter.
    #[required]
    pub numbers: Option<Content>,
}

impl Show for FigureCaption {
    #[tracing::instrument(name = "FigureCaption::show", skip(self))]
    fn show(&self, _: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        let mut caption = self.body();

        if let (Some(Supplement::Content(mut supplement)), Some(numbers)) =
            (self.supplement(), self.numbers())
        {
            if !supplement.is_empty() {
                supplement += TextElem::packed("\u{a0}");
            }

            caption = supplement + numbers + TextElem::packed(": ") + caption;
        }

        Ok(caption)
    }
}

/// The `kind` parameter of a [`FigureElem`].
#[derive(Debug, Clone)]
pub enum FigureKind {
    /// The kind is an element function.
    Elem(Element),
    /// The kind is a name.
    Name(EcoString),
}

cast! {
    FigureKind,
    self => match self {
        Self::Elem(v) => v.into_value(),
        Self::Name(v) => v.into_value(),
    },
    v: Element => Self::Elem(v),
    v: EcoString => Self::Name(v),
}

/// An element that can be auto-detected in a figure.
///
/// This trait is used to determine the type of a figure.
pub trait Figurable: LocalName {}
