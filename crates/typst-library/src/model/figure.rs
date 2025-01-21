use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::str::FromStr;

use ecow::EcoString;
use typst_utils::NonZeroExt;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, select_where, Content, Element, NativeElement, Packed, Selector,
    Show, ShowSet, Smart, StyleChain, Styles, Synthesize, TargetElem,
};
use crate::html::{tag, HtmlElem};
use crate::introspection::{
    Count, Counter, CounterKey, CounterUpdate, Locatable, Location,
};
use crate::layout::{
    AlignElem, Alignment, BlockBody, BlockElem, Em, HAlignment, Length, OuterVAlignment,
    PlaceElem, PlacementScope, VAlignment, VElem,
};
use crate::model::{Numbering, NumberingPattern, Outlinable, Refable, Supplement};
use crate::text::{Lang, Region, TextElem};
use crate::visualize::ImageElem;

/// A figure with an optional caption.
///
/// Automatically detects its kind to select the correct counting track. For
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
/// You can modify the appearance of the figure's caption with its associated
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
#[elem(scope, Locatable, Synthesize, Count, Show, ShowSet, Refable, Outlinable)]
pub struct FigureElem {
    /// The content of the figure. Often, an [image].
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
    pub placement: Option<Smart<VAlignment>>,

    /// Relative to which containing scope the figure is placed.
    ///
    /// Set this to `{"parent"}` to create a full-width figure in a two-column
    /// document.
    ///
    /// Has no effect if `placement` is `{none}`.
    ///
    /// ```example
    /// #set page(height: 250pt, columns: 2)
    ///
    /// = Introduction
    /// #figure(
    ///   placement: bottom,
    ///   scope: "parent",
    ///   caption: [A glacier],
    ///   image("glacier.jpg", width: 60%),
    /// )
    /// #lorem(60)
    /// ```
    pub scope: PlacementScope,

    /// The figure's caption.
    #[borrowed]
    pub caption: Option<Packed<FigureCaption>>,

    /// The kind of figure this is.
    ///
    /// All figures of the same kind share a common counter.
    ///
    /// If set to `{auto}`, the figure will try to automatically determine its
    /// kind based on the type of its body. Automatically detected kinds are
    /// [tables]($table) and [code]($raw). In other cases, the inferred kind is
    /// that of an [image].
    ///
    /// Setting this to something other than `{auto}` will override the
    /// automatic detection. This can be useful if
    /// - you wish to create a custom figure type that is not an
    ///   [image], a [table] or [code]($raw),
    /// - you want to force the figure to use a specific counter regardless of
    ///   its content.
    ///
    /// You can set the kind to be an element function or a string. If you set
    /// it to an element function other than [`{table}`]($table), [`{raw}`](raw)
    /// or [`{image}`](image), you will need to manually specify the figure's
    /// supplement.
    ///
    /// ```example
    /// #figure(
    ///   circle(radius: 10pt),
    ///   caption: [A curious atom.],
    ///   kind: "atom",
    ///   supplement: [Atom],
    /// )
    /// ```
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
    #[borrowed]
    pub supplement: Smart<Option<Supplement>>,

    /// How to number the figure. Accepts a
    /// [numbering pattern or function]($numbering).
    #[default(Some(NumberingPattern::from_str("1").unwrap().into()))]
    #[borrowed]
    pub numbering: Option<Numbering>,

    /// The vertical gap between the body and caption.
    #[default(Em::new(0.65).into())]
    pub gap: Length,

    /// Whether the figure should appear in an [`outline`] of figures.
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

impl Synthesize for Packed<FigureElem> {
    fn synthesize(
        &mut self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let span = self.span();
        let location = self.location();
        let elem = self.as_mut();
        let numbering = elem.numbering(styles);

        // Determine the figure's kind.
        let kind = elem.kind(styles).unwrap_or_else(|| {
            elem.body
                .query_first(&Selector::can::<dyn Figurable>())
                .map(|elem| FigureKind::Elem(elem.func()))
                .unwrap_or_else(|| FigureKind::Elem(ImageElem::elem()))
        });

        // Resolve the supplement.
        let supplement = match elem.supplement(styles).as_ref() {
            Smart::Auto => {
                // Default to the local name for the kind, if available.
                let name = match &kind {
                    FigureKind::Elem(func) => func
                        .local_name(
                            TextElem::lang_in(styles),
                            TextElem::region_in(styles),
                        )
                        .map(TextElem::packed),
                    FigureKind::Name(_) => None,
                };

                if numbering.is_some() && name.is_none() {
                    bail!(span, "please specify the figure's supplement")
                }

                Some(name.unwrap_or_default())
            }
            Smart::Custom(None) => None,
            Smart::Custom(Some(supplement)) => {
                // Resolve the supplement with the first descendant of the kind or
                // just the body, if none was found.
                let descendant = match kind {
                    FigureKind::Elem(func) => {
                        elem.body.query_first(&Selector::Elem(func, None)).map(Cow::Owned)
                    }
                    FigureKind::Name(_) => None,
                };

                let target = descendant.unwrap_or_else(|| Cow::Borrowed(&elem.body));
                Some(supplement.resolve(engine, styles, [target])?)
            }
        };

        // Construct the figure's counter.
        let counter = Counter::new(CounterKey::Selector(
            select_where!(FigureElem, Kind => kind.clone()),
        ));

        // Fill the figure's caption.
        let mut caption = elem.caption(styles).clone();
        if let Some(caption) = &mut caption {
            caption.synthesize(engine, styles)?;
            caption.push_kind(kind.clone());
            caption.push_supplement(supplement.clone());
            caption.push_numbering(numbering.clone());
            caption.push_counter(Some(counter.clone()));
            caption.push_figure_location(location);
        }

        elem.push_kind(Smart::Custom(kind));
        elem.push_supplement(Smart::Custom(supplement.map(Supplement::Content)));
        elem.push_counter(Some(counter));
        elem.push_caption(caption);

        Ok(())
    }
}

impl Show for Packed<FigureElem> {
    #[typst_macros::time(name = "figure", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let target = TargetElem::target_in(styles);
        let mut realized = self.body.clone();

        // Build the caption, if any.
        if let Some(caption) = self.caption(styles).clone() {
            let (first, second) = match caption.position(styles) {
                OuterVAlignment::Top => (caption.pack(), realized),
                OuterVAlignment::Bottom => (realized, caption.pack()),
            };
            let mut seq = Vec::with_capacity(3);
            seq.push(first);
            if !target.is_html() {
                let v = VElem::new(self.gap(styles).into()).with_weak(true);
                seq.push(v.pack().spanned(self.span()))
            }
            seq.push(second);
            realized = Content::sequence(seq)
        }

        if target.is_html() {
            return Ok(HtmlElem::new(tag::figure)
                .with_body(Some(realized))
                .pack()
                .spanned(self.span()));
        }

        // Wrap the contents in a block.
        realized = BlockElem::new()
            .with_body(Some(BlockBody::Content(realized)))
            .pack()
            .spanned(self.span());

        // Wrap in a float.
        if let Some(align) = self.placement(styles) {
            realized = PlaceElem::new(realized)
                .with_alignment(align.map(|align| HAlignment::Center + align))
                .with_scope(self.scope(styles))
                .with_float(true)
                .pack()
                .spanned(self.span());
        } else if self.scope(styles) == PlacementScope::Parent {
            bail!(
                self.span(),
                "parent-scoped placement is only available for floating figures";
                hint: "you can enable floating placement with `figure(placement: auto, ..)`"
            );
        }

        Ok(realized)
    }
}

impl ShowSet for Packed<FigureElem> {
    fn show_set(&self, _: StyleChain) -> Styles {
        // Still allows breakable figures with
        // `show figure: set block(breakable: true)`.
        let mut map = Styles::new();
        map.set(BlockElem::set_breakable(false));
        map.set(AlignElem::set_alignment(Alignment::CENTER));
        map
    }
}

impl Count for Packed<FigureElem> {
    fn update(&self) -> Option<CounterUpdate> {
        // If the figure is numbered, step the counter by one.
        // This steps the `counter(figure)` which is global to all numbered figures.
        self.numbering()
            .is_some()
            .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl Refable for Packed<FigureElem> {
    fn supplement(&self) -> Content {
        // After synthesis, this should always be custom content.
        match (**self).supplement(StyleChain::default()).as_ref() {
            Smart::Custom(Some(Supplement::Content(content))) => content.clone(),
            _ => Content::empty(),
        }
    }

    fn counter(&self) -> Counter {
        (**self)
            .counter()
            .cloned()
            .flatten()
            .unwrap_or_else(|| Counter::of(FigureElem::elem()))
    }

    fn numbering(&self) -> Option<&Numbering> {
        (**self).numbering(StyleChain::default()).as_ref()
    }
}

impl Outlinable for Packed<FigureElem> {
    fn outlined(&self) -> bool {
        (**self).outlined(StyleChain::default())
            && (self.caption(StyleChain::default()).is_some()
                || self.numbering().is_some())
    }

    fn prefix(&self, numbers: Content) -> Content {
        let supplement = self.supplement();
        if !supplement.is_empty() {
            supplement + TextElem::packed('\u{a0}') + numbers
        } else {
            numbers
        }
    }

    fn body(&self) -> Content {
        self.caption(StyleChain::default())
            .as_ref()
            .map(|caption| caption.body.clone())
            .unwrap_or_default()
    }
}

/// The caption of a figure. This element can be used in set and show rules to
/// customize the appearance of captions for all figures or figures of a
/// specific kind.
///
/// In addition to its `pos` and `body`, the `caption` also provides the
/// figure's `kind`, `supplement`, `counter`, and `numbering` as fields. These
/// parts can be used in [`where`]($function.where) selectors and show rules to
/// build a completely custom caption.
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
    #[default(OuterVAlignment::Bottom)]
    pub position: OuterVAlignment,

    /// The separator which will appear between the number and body.
    ///
    /// If set to `{auto}`, the separator will be adapted to the current
    /// [language]($text.lang) and [region]($text.region).
    ///
    /// ```example
    /// #set figure.caption(separator: [ --- ])
    ///
    /// #figure(
    ///   rect[Hello],
    ///   caption: [A rectangle],
    /// )
    /// ```
    pub separator: Smart<Content>,

    /// The caption's body.
    ///
    /// Can be used alongside `kind`, `supplement`, `counter`, `numbering`, and
    /// `location` to completely customize the caption.
    ///
    /// ```example
    /// #show figure.caption: it => [
    ///   #underline(it.body) |
    ///   #it.supplement
    ///   #context it.counter.display(it.numbering)
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
    #[internal]
    #[synthesized]
    pub figure_location: Option<Location>,
}

impl FigureCaption {
    /// Gets the default separator in the given language and (optionally)
    /// region.
    fn local_separator(lang: Lang, _: Option<Region>) -> &'static str {
        match lang {
            Lang::CHINESE => "\u{2003}",
            Lang::FRENCH => ".\u{a0}– ",
            Lang::RUSSIAN => ". ",
            Lang::ENGLISH | _ => ": ",
        }
    }

    fn get_separator(&self, styles: StyleChain) -> Content {
        self.separator(styles).unwrap_or_else(|| {
            TextElem::packed(Self::local_separator(
                TextElem::lang_in(styles),
                TextElem::region_in(styles),
            ))
        })
    }
}

impl Synthesize for Packed<FigureCaption> {
    fn synthesize(&mut self, _: &mut Engine, styles: StyleChain) -> SourceResult<()> {
        let elem = self.as_mut();
        elem.push_separator(Smart::Custom(elem.get_separator(styles)));
        Ok(())
    }
}

impl Show for Packed<FigureCaption> {
    #[typst_macros::time(name = "figure.caption", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body.clone();

        if let (
            Some(Some(mut supplement)),
            Some(Some(numbering)),
            Some(Some(counter)),
            Some(Some(location)),
        ) = (
            self.supplement().cloned(),
            self.numbering(),
            self.counter(),
            self.figure_location(),
        ) {
            let numbers = counter.display_at_loc(engine, *location, styles, numbering)?;
            if !supplement.is_empty() {
                supplement += TextElem::packed('\u{a0}');
            }
            realized = supplement + numbers + self.get_separator(styles) + realized;
        }

        if TargetElem::target_in(styles).is_html() {
            return Ok(HtmlElem::new(tag::figcaption)
                .with_body(Some(realized))
                .pack()
                .spanned(self.span()));
        }

        Ok(realized)
    }
}

cast! {
    FigureCaption,
    v: Content => v.unpack::<Self>().unwrap_or_else(Self::new),
}

/// The `kind` parameter of a [`FigureElem`].
#[derive(Debug, Clone, PartialEq, Hash)]
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
pub trait Figurable {}
