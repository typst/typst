use std::num::NonZeroUsize;

use ecow::EcoString;
use typst_utils::NonZeroExt;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    Content, NativeElement, Packed, ShowSet, Smart, StyleChain, Styles, Synthesize, elem,
};
use crate::introspection::{Count, Counter, CounterUpdate, Locatable, Tagged};
use crate::layout::{BlockElem, Em, Length};
use crate::model::{Numbering, Outlinable, Refable, Supplement};
use crate::text::{FontWeight, LocalName, TextElem, TextSize};

/// A section heading.
///
/// With headings, you can structure your document into sections. Each heading
/// has a _level,_ which starts at one and is unbounded upwards. This level
/// indicates the logical role of the following content (section, subsection,
/// etc.) A top-level heading indicates a top-level section of the document (not
/// the document's title). To insert a title, use the [`title`]($title) element
/// instead.
///
/// Typst can automatically number your headings for you. To enable numbering,
/// specify how you want your headings to be numbered with a
/// [numbering pattern or function]($numbering).
///
/// Independently of the numbering, Typst can also automatically generate an
/// [outline] of all headings for you. To exclude one or more headings from this
/// outline, you can set the `outlined` parameter to `{false}`.
///
/// When writing a [show rule]($styling/#show-rules) that accesses the
/// [`body` field]($heading.body) to create a completely custom look for
/// headings, make sure to wrap the content in a [`block`]($block) (which is
/// implicitly [sticky]($block.sticky) for headings through a built-in show-set
/// rule). This prevents headings from becoming "orphans", i.e. remaining
/// at the end of the page with the following content being on the next page.
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
///
/// # Accessibility
/// Headings are important for accessibility, as they help users of Assistive
/// Technologies (AT) like screen readers to navigate within your document.
/// Screen reader users will be able to skip from heading to heading, or get an
/// overview of all headings in the document.
///
/// To make your headings accessible, you should not skip heading levels. This
/// means that you should start with a first-level heading. Also, when the
/// previous heading was of level 3, the next heading should be of level 3
/// (staying at the same depth), level 4 (going exactly one level deeper), or
/// level 1 or 2 (new hierarchically higher headings).
///
/// # HTML export
/// As mentioned above, a top-level heading indicates a top-level section of
/// the document rather than its title. This is in contrast to the HTML `<h1>`
/// element of which there should be only one per document.
///
/// For this reason, in HTML export, a [`title`] element will turn into an
/// `<h1>` and headings turn into `<h2>` and lower (a level 1 heading thus turns
/// into `<h2>`, a level 2 heading into `<h3>`, etc).
#[elem(Locatable, Tagged, Synthesize, Count, ShowSet, LocalName, Refable, Outlinable)]
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
    /// [numbering pattern or function]($numbering) taking multiple numbers.
    ///
    /// ```example
    /// #set heading(numbering: "1.a.")
    ///
    /// = A section
    /// == A subsection
    /// === A sub-subsection
    /// ```
    pub numbering: Option<Numbering>,

    /// The resolved plain-text numbers.
    ///
    /// This field is internal and only used for creating PDF bookmarks. We
    /// don't currently have access to `World`, `Engine`, or `styles` in export,
    /// which is needed to resolve the counter and numbering pattern into a
    /// concrete string.
    ///
    /// This remains unset if `numbering` is `None`.
    #[internal]
    #[synthesized]
    pub numbers: EcoString,

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
    /// The default value of `{auto}` uses the width of the numbering as indent
    /// if the heading is aligned at the [start]($direction.start) of the [text
    /// direction]($text.dir), and no indent for center and other alignments.
    ///
    /// ```example
    /// #set heading(numbering: "1.")
    /// = A very, very, very, very, very, very long heading
    ///
    /// #show heading: set align(center)
    /// == A very long heading\ with center alignment
    /// ```
    #[default(Smart::Auto)]
    pub hanging_indent: Smart<Length>,

    /// The heading's title.
    #[required]
    pub body: Content,
}

impl HeadingElem {
    pub fn resolve_level(&self, styles: StyleChain) -> NonZeroUsize {
        self.level.get(styles).unwrap_or_else(|| {
            NonZeroUsize::new(self.offset.get(styles) + self.depth.get(styles).get())
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
        let supplement = match self.supplement.get_ref(styles) {
            Smart::Auto => TextElem::packed(Self::local_name_in(styles)),
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(supplement)) => {
                supplement.resolve(engine, styles, [self.clone().pack()])?
            }
        };

        if let Some((numbering, location)) =
            self.numbering.get_ref(styles).as_ref().zip(self.location())
            // We are not early returning on error here because of
            // https://github.com/typst/typst/issues/7428
            //
            // A more comprehensive fix might introduce the error catching logic
            // of show rules for synthesis, too.
            && let Ok(numbers) = self.counter().display_at(
                engine,
                location,
                styles,
                numbering,
                self.span(),
            )
        {
            self.numbers = Some(numbers.plain_text());
        }

        let elem = self.as_mut();
        elem.level.set(Smart::Custom(elem.resolve_level(styles)));
        elem.supplement
            .set(Smart::Custom(Some(Supplement::Content(supplement))));
        Ok(())
    }
}

impl ShowSet for Packed<HeadingElem> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let level = self.resolve_level(styles).get();
        let scale = match level {
            1 => 1.4,
            2 => 1.2,
            _ => 1.0,
        };

        let size = Em::new(scale);
        let above = Em::new(if level == 1 { 1.8 } else { 1.44 }) / scale;
        let below = Em::new(0.75) / scale;

        let mut out = Styles::new();
        out.set(TextElem::size, TextSize(size.into()));
        out.set(TextElem::weight, FontWeight::BOLD);
        out.set(BlockElem::above, Smart::Custom(above.into()));
        out.set(BlockElem::below, Smart::Custom(below.into()));
        out.set(BlockElem::sticky, true);
        out
    }
}

impl Count for Packed<HeadingElem> {
    fn update(&self) -> Option<CounterUpdate> {
        self.numbering
            .get_ref(StyleChain::default())
            .is_some()
            .then(|| CounterUpdate::Step(self.resolve_level(StyleChain::default())))
    }
}

impl Refable for Packed<HeadingElem> {
    fn supplement(&self) -> Content {
        // After synthesis, this should always be custom content.
        match self.supplement.get_cloned(StyleChain::default()) {
            Smart::Custom(Some(Supplement::Content(content))) => content,
            _ => Content::empty(),
        }
    }

    fn counter(&self) -> Counter {
        Counter::of(HeadingElem::ELEM)
    }

    fn numbering(&self) -> Option<&Numbering> {
        self.numbering.get_ref(StyleChain::default()).as_ref()
    }
}

impl Outlinable for Packed<HeadingElem> {
    fn outlined(&self) -> bool {
        self.outlined.get(StyleChain::default())
    }

    fn level(&self) -> NonZeroUsize {
        self.resolve_level(StyleChain::default())
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
