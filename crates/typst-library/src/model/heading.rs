use std::num::NonZeroUsize;

use ecow::eco_format;
use typst_utils::{Get, NonZeroExt};

use crate::diag::{warning, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, NativeElement, Packed, Resolve, Show, ShowSet, Smart, StyleChain,
    Styles, Synthesize, TargetElem,
};
use crate::html::{attr, tag, HtmlElem};
use crate::introspection::{
    Count, Counter, CounterUpdate, Locatable, Locator, LocatorLink,
};
use crate::layout::{Abs, Axes, BlockBody, BlockElem, Em, HElem, Length, Region, Sides};
use crate::model::{Numbering, Outlinable, Refable, Supplement};
use crate::text::{FontWeight, LocalName, SpaceElem, TextElem, TextSize};

/// A section heading.
///
/// With headings, you can structure your document into sections. Each heading
/// has a _level,_ which starts at one and is unbounded upwards. This level
/// indicates the logical role of the following content (section, subsection,
/// etc.) A top-level heading indicates a top-level section of the document
/// (not the document's title).
///
/// Typst can automatically number your headings for you. To enable numbering,
/// specify how you want your headings to be numbered with a
/// [numbering pattern or function]($numbering).
///
/// Independently of the numbering, Typst can also automatically generate an
/// [outline] of all headings for you. To exclude one or more headings from this
/// outline, you can set the `outlined` parameter to `{false}`.
///
/// # Example
/// ```example
/// #set heading(numbering: "1.a)")
///
/// = Introduction
/// In recent years, ...
///
/// == Preliminaries
/// To start, ...
/// ```
///
/// # Syntax
/// Headings have dedicated syntax: They can be created by starting a line with
/// one or multiple equals signs, followed by a space. The number of equals
/// signs determines the heading's logical nesting depth. The `{offset}` field
/// can be set to configure the starting depth.
#[elem(Locatable, Synthesize, Count, Show, ShowSet, LocalName, Refable, Outlinable)]
pub struct HeadingElem {
    /// The absolute nesting depth of the heading, starting from one. If set
    /// to `{auto}`, it is computed from `{offset + depth}`.
    ///
    /// This is primarily useful for usage in [show rules]($styling/#show-rules)
    /// (either with [`where`]($function.where) selectors or by accessing the
    /// level directly on a shown heading).
    ///
    /// ```example
    /// #show heading.where(level: 2): set text(red)
    ///
    /// = Level 1
    /// == Level 2
    ///
    /// #set heading(offset: 1)
    /// = Also level 2
    /// == Level 3
    /// ```
    pub level: Smart<NonZeroUsize>,

    /// The relative nesting depth of the heading, starting from one. This is
    /// combined with `{offset}` to compute the actual `{level}`.
    ///
    /// This is set by the heading syntax, such that `[== Heading]` creates a
    /// heading with logical depth of 2, but actual level `{offset + 2}`. If you
    /// construct a heading manually, you should typically prefer this over
    /// setting the absolute level.
    #[default(NonZeroUsize::ONE)]
    pub depth: NonZeroUsize,

    /// The starting offset of each heading's `{level}`, used to turn its
    /// relative `{depth}` into its absolute `{level}`.
    ///
    /// ```example
    /// = Level 1
    ///
    /// #set heading(offset: 1, numbering: "1.1")
    /// = Level 2
    ///
    /// #heading(offset: 2, depth: 2)[
    ///   I'm level 4
    /// ]
    /// ```
    #[default(0)]
    pub offset: usize,

    /// How to number the heading. Accepts a
    /// [numbering pattern or function]($numbering).
    ///
    /// ```example
    /// #set heading(numbering: "1.a.")
    ///
    /// = A section
    /// == A subsection
    /// === A sub-subsection
    /// ```
    #[borrowed]
    pub numbering: Option<Numbering>,

    /// A supplement for the heading.
    ///
    /// For references to headings, this is added before the referenced number.
    ///
    /// If a function is specified, it is passed the referenced heading and
    /// should return content.
    ///
    /// ```example
    /// #set heading(numbering: "1.", supplement: [Chapter])
    ///
    /// = Introduction <intro>
    /// In @intro, we see how to turn
    /// Sections into Chapters. And
    /// in @intro[Part], it is done
    /// manually.
    /// ```
    pub supplement: Smart<Option<Supplement>>,

    /// Whether the heading should appear in the [outline].
    ///
    /// Note that this property, if set to `{true}`, ensures the heading is also
    /// shown as a bookmark in the exported PDF's outline (when exporting to
    /// PDF). To change that behavior, use the `bookmarked` property.
    ///
    /// ```example
    /// #outline()
    ///
    /// #heading[Normal]
    /// This is a normal heading.
    ///
    /// #heading(outlined: false)[Hidden]
    /// This heading does not appear
    /// in the outline.
    /// ```
    #[default(true)]
    pub outlined: bool,

    /// Whether the heading should appear as a bookmark in the exported PDF's
    /// outline. Doesn't affect other export formats, such as PNG.
    ///
    /// The default value of `{auto}` indicates that the heading will only
    /// appear in the exported PDF's outline if its `outlined` property is set
    /// to `{true}`, that is, if it would also be listed in Typst's [outline].
    /// Setting this property to either `{true}` (bookmark) or `{false}` (don't
    /// bookmark) bypasses that behavior.
    ///
    /// ```example
    /// #heading[Normal heading]
    /// This heading will be shown in
    /// the PDF's bookmark outline.
    ///
    /// #heading(bookmarked: false)[Not bookmarked]
    /// This heading won't be
    /// bookmarked in the resulting
    /// PDF.
    /// ```
    #[default(Smart::Auto)]
    pub bookmarked: Smart<bool>,

    /// The indent all but the first line of a heading should have.
    ///
    /// The default value of `{auto}` indicates that the subsequent heading
    /// lines will be indented based on the width of the numbering.
    ///
    /// ```example
    /// #set heading(numbering: "1.")
    /// #heading[A very, very, very, very, very, very long heading]
    /// ```
    #[default(Smart::Auto)]
    pub hanging_indent: Smart<Length>,

    /// The spacing between the numbering and the heading's title.
    /// 
    /// The default value is `{0.3em}`.
    /// 
    /// ```example
    /// #set heading(numbering: "1.")
    /// #heading[This is a heading with 0.3em spacing]
    /// #heading(spacing-to-numbering: 0em)[This is a heading with 0em spacing]
    /// ```
    #[default(Em::new(0.3).into())]
    pub spacing_to_numbering: Length,

    /// The heading's title.
    #[required]
    pub body: Content,
}

impl HeadingElem {
    pub fn resolve_level(&self, styles: StyleChain) -> NonZeroUsize {
        self.level(styles).unwrap_or_else(|| {
            NonZeroUsize::new(self.offset(styles) + self.depth(styles).get())
                .expect("overflow to 0 on NoneZeroUsize + usize")
        })
    }
}

impl Synthesize for Packed<HeadingElem> {
    fn synthesize(
        &mut self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let supplement = match (**self).supplement(styles) {
            Smart::Auto => TextElem::packed(Self::local_name_in(styles)),
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(supplement)) => {
                supplement.resolve(engine, styles, [self.clone().pack()])?
            }
        };

        let elem = self.as_mut();
        elem.push_level(Smart::Custom(elem.resolve_level(styles)));
        elem.push_supplement(Smart::Custom(Some(Supplement::Content(supplement))));
        Ok(())
    }
}

impl Show for Packed<HeadingElem> {
    #[typst_macros::time(name = "heading", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let html = TargetElem::target_in(styles).is_html();

        let span = self.span();
        let mut realized = self.body.clone();

        let hanging_indent = self.hanging_indent(styles);
        let mut indent = match hanging_indent {
            Smart::Custom(length) => length.resolve(styles),
            Smart::Auto => Abs::zero(),
        };

        let spacing_to_numbering: Length = self.spacing_to_numbering.unwrap();

        if let Some(numbering) = (**self).numbering(styles).as_ref() {
            let location = self.location().unwrap();
            let numbering = Counter::of(HeadingElem::elem())
                .display_at_loc(engine, location, styles, numbering)?
                .spanned(span);

            if hanging_indent.is_auto() && !html {
                let pod = Region::new(Axes::splat(Abs::inf()), Axes::splat(false));

                // We don't have a locator for the numbering here, so we just
                // use the measurement infrastructure for now.
                let link = LocatorLink::measure(location);
                let size = (engine.routines.layout_frame)(
                    engine,
                    &numbering,
                    Locator::link(&link),
                    styles,
                    pod,
                )?
                .size();

                indent = size.x + spacing_to_numbering.resolve(styles);
            }

            let spacing = if html {
                SpaceElem::shared().clone()
            } else {
                HElem::new(spacing_to_numbering.resolve(styles).into()).with_weak(true).pack()
            };

            realized = numbering + spacing + realized;
        }

        Ok(if html {
            // HTML's h1 is closer to a title element. There should only be one.
            // Meanwhile, a level 1 Typst heading is a section heading. For this
            // reason, levels are offset by one: A Typst level 1 heading becomes
            // a `<h2>`.
            let level = self.resolve_level(styles).get();
            if level >= 6 {
                engine.sink.warn(warning!(span,
                    "heading of level {} was transformed to \
                    <div role=\"heading\" aria-level=\"{}\">, which is not \
                    supported by all assistive technology",
                    level, level + 1;
                    hint: "HTML only supports <h1> to <h6>, not <h{}>", level + 1;
                    hint: "you may want to restructure your document so that \
                          it doesn't contain deep headings"));
                HtmlElem::new(tag::div)
                    .with_body(Some(realized))
                    .with_attr(attr::role, "heading")
                    .with_attr(attr::aria_level, eco_format!("{}", level + 1))
                    .pack()
                    .spanned(span)
            } else {
                let t = [tag::h2, tag::h3, tag::h4, tag::h5, tag::h6][level - 1];
                HtmlElem::new(t).with_body(Some(realized)).pack().spanned(span)
            }
        } else {
            let block = if indent != Abs::zero() {
                let body = HElem::new((-indent).into()).pack() + realized;
                let inset = Sides::default()
                    .with(TextElem::dir_in(styles).start(), Some(indent.into()));
                BlockElem::new()
                    .with_body(Some(BlockBody::Content(body)))
                    .with_inset(inset)
            } else {
                BlockElem::new().with_body(Some(BlockBody::Content(realized)))
            };
            block.pack().spanned(span)
        })
    }
}

impl ShowSet for Packed<HeadingElem> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let level = (**self).resolve_level(styles).get();
        let scale = match level {
            1 => 1.4,
            2 => 1.2,
            _ => 1.0,
        };

        let size = Em::new(scale);
        let above = Em::new(if level == 1 { 1.8 } else { 1.44 }) / scale;
        let below = Em::new(0.75) / scale;

        let mut out = Styles::new();
        out.set(TextElem::set_size(TextSize(size.into())));
        out.set(TextElem::set_weight(FontWeight::BOLD));
        out.set(BlockElem::set_above(Smart::Custom(above.into())));
        out.set(BlockElem::set_below(Smart::Custom(below.into())));
        out.set(BlockElem::set_sticky(true));
        out
    }
}

impl Count for Packed<HeadingElem> {
    fn update(&self) -> Option<CounterUpdate> {
        (**self)
            .numbering(StyleChain::default())
            .is_some()
            .then(|| CounterUpdate::Step((**self).resolve_level(StyleChain::default())))
    }
}

impl Refable for Packed<HeadingElem> {
    fn supplement(&self) -> Content {
        // After synthesis, this should always be custom content.
        match (**self).supplement(StyleChain::default()) {
            Smart::Custom(Some(Supplement::Content(content))) => content,
            _ => Content::empty(),
        }
    }

    fn counter(&self) -> Counter {
        Counter::of(HeadingElem::elem())
    }

    fn numbering(&self) -> Option<&Numbering> {
        (**self).numbering(StyleChain::default()).as_ref()
    }
}

impl Outlinable for Packed<HeadingElem> {
    fn outlined(&self) -> bool {
        (**self).outlined(StyleChain::default())
    }

    fn level(&self) -> NonZeroUsize {
        (**self).resolve_level(StyleChain::default())
    }

    fn prefix(&self, numbers: Content) -> Content {
        numbers
    }

    fn body(&self) -> Content {
        self.body.clone()
    }
}

impl LocalName for Packed<HeadingElem> {
    const KEY: &'static str = "heading";
}
