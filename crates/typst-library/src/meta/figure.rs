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
/// Automatically detects its contents to select the correct counting track. For
/// example, figures containing images will be numbered separately from figures
/// containing tables.
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
/// # Figure behaviour
/// By default, figures are placed within the flow of content. To make them
/// float to the top or bottom of the page, you can use the
/// [`placement`]($figure.placement) argument.
///
/// If your figure is too large and its contents are breakable across pages
/// (e.g. if it contains a large table), then you can make the figure itself
/// breakable across pages as well with this show rule:
/// ```typ
/// #show figure: set block(breakable: true)
/// ```
///
/// See the [block]($block.breakable) documentation for more information about
/// breakable and non-breakable blocks.
///
/// # Caption customization
/// You can modify the apperance of the figure's caption with its associated
/// [`caption`]($figure.caption) function. In the example below, we emphasize
/// all captions:
///
/// ```example
/// #show figure.caption: emph
///
/// #figure(
///   rect[Hello],
///   caption: [I am emphasized!],
/// )
/// ```
///
/// By using a [`where`]($function.where) selector, we can scope such rules to
/// specific kinds of figures. For example, to position the caption above
/// tables, but keep it below for all other kinds of figures, we could write the
/// following show-set rule:
///
/// ```example
/// #show figure.where(
///   kind: table
/// ): set figure.caption(position: top)
///
/// #figure(
///   table(columns: 2)[A][B][C][D],
///   caption: [I'm up here],
/// )
/// ```
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
    /// The gap between the main flow content and the floating figure is
    /// controlled by the [`clearance`]($place.clearance) argument on the
    /// `place` function.
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
    pub caption: Option<FigureCaption>,

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

                Some(name.unwrap_or_default())
            }
            Smart::Custom(None) => None,
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
                Some(supplement.resolve(vt, [target])?)
            }
        };

        // Construct the figure's counter.
        let counter = Counter::new(CounterKey::Selector(Selector::Elem(
            Self::elem(),
            Some(dict! {
                "kind" => kind.clone(),
            }),
        )));

        // Fill the figure's caption.
        let mut caption = self.caption(styles);
        if let Some(caption) = &mut caption {
            caption.push_kind(kind.clone());
            caption.push_supplement(supplement.clone());
            caption.push_numbering(numbering.clone());
            caption.push_counter(Some(counter.clone()));
            caption.push_location(self.0.location());
        }

        self.push_placement(self.placement(styles));
        self.push_caption(caption);
        self.push_kind(Smart::Custom(kind));
        self.push_supplement(Smart::Custom(supplement.map(Supplement::Content)));
        self.push_numbering(numbering);
        self.push_outlined(self.outlined(styles));
        self.push_counter(Some(counter));

        Ok(())
    }
}

impl Show for FigureElem {
    #[tracing::instrument(name = "FigureElem::show", skip_all)]
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();

        // Build the caption, if any.
        if let Some(caption) = self.caption(styles) {
            let v = VElem::weak(self.gap(styles).into()).pack();
            realized = if caption.position(styles) == VAlign::Bottom {
                realized + v + caption.pack()
            } else {
                caption.pack() + v + realized
            };
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

        let Some(caption) = self.caption(StyleChain::default()) else {
            return Ok(None);
        };

        let mut realized = caption.body();
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
                supplement += TextElem::packed('\u{a0}');
            }

            let separator = caption.separator(StyleChain::default());

            realized = supplement + numbers + separator + caption.body();
        }

        Ok(Some(realized))
    }
}

/// The caption of a figure. This element can be used in set and show rules to
/// customize the appearance of captions for all figures or figures of a
/// specific kind.
///
/// In addition to its `pos` and `body`, the `caption` also provides the
/// figure's `kind`, `supplement`, `counter`, `numbering`, and `location` as
/// fields. These parts can be used in [`where`]($function.where) selectors and
/// show rules to build a completely custom caption.
///
/// ```example
/// #show figure.caption: emph
///
/// #figure(
///   rect[Hello],
///   caption: [A rectangle],
/// )
/// ```
#[elem(name = "caption", Synthesize, Show)]
pub struct FigureCaption {
    /// The caption's position in the figure. Either `{top}` or `{bottom}`.
    ///
    /// ```example
    /// #show figure.where(
    ///   kind: table
    /// ): set figure.caption(position: top)
    ///
    /// #figure(
    ///   table(columns: 2)[A][B],
    ///   caption: [I'm up here],
    /// )
    ///
    /// #figure(
    ///   rect[Hi],
    ///   caption: [I'm down here],
    /// )
    ///
    /// #figure(
    ///   table(columns: 2)[A][B],
    ///   caption: figure.caption(
    ///     position: bottom,
    ///     [I'm down here too!]
    ///   )
    /// )
    /// ```
    #[default(VAlign::Bottom)]
    #[parse({
        let option: Option<Spanned<VAlign>> = args.named("position")?;
        if let Some(Spanned { v: align, span }) = option {
            if align == VAlign::Horizon {
                bail!(span, "expected `top` or `bottom`");
            }
        }
        option.map(|spanned| spanned.v)
    })]
    pub position: VAlign,

    /// The separator which will appear between the number and body.
    ///
    /// ```example
    /// #set figure.caption(separator: [ --- ])
    ///
    /// #figure(
    ///   rect[Hello],
    ///   caption: [A rectangle],
    /// )
    /// ```
    #[default(TextElem::packed(": "))]
    pub separator: Content,

    /// The caption's body.
    ///
    /// Can be used alongside `kind`, `supplement`, `counter`, `numbering`, and
    /// `location` to completely customize the caption.
    ///
    /// ```example
    /// #show figure.caption: it => [
    ///   #underline(it.body) |
    ///   #it.supplement #it.counter.display(it.numbering)
    /// ]
    ///
    /// #figure(
    ///   rect[Hello],
    ///   caption: [A rectangle],
    /// )
    /// ```
    #[required]
    pub body: Content,

    /// The figure's supplement.
    #[synthesized]
    pub kind: FigureKind,

    /// The figure's supplement.
    #[synthesized]
    pub supplement: Option<Content>,

    /// How to number the figure.
    #[synthesized]
    pub numbering: Option<Numbering>,

    /// The counter for the figure.
    #[synthesized]
    pub counter: Option<Counter>,

    /// The figure's location.
    #[synthesized]
    pub location: Option<Location>,
}

impl Synthesize for FigureCaption {
    fn synthesize(&mut self, _: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        self.push_position(self.position(styles));
        self.push_separator(self.separator(styles));
        Ok(())
    }
}

impl Show for FigureCaption {
    #[tracing::instrument(name = "FigureCaption::show", skip_all)]
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();

        if let (Some(mut supplement), Some(numbering), Some(counter), Some(location)) =
            (self.supplement(), self.numbering(), self.counter(), self.location())
        {
            let numbers = counter.at(vt, location)?.display(vt, &numbering)?;
            if !supplement.is_empty() {
                supplement += TextElem::packed('\u{a0}');
            }
            realized = supplement + numbers + self.separator(styles) + realized;
        }

        Ok(realized)
    }
}

cast! {
    FigureCaption,
    v: Content => v.to::<Self>().cloned().unwrap_or_else(|| Self::new(v.clone())),
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
