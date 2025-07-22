use std::num::NonZeroUsize;

use ecow::EcoString;
use typst_utils::NonZeroExt;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    Content, NativeElement, Packed, ShowSet, Smart, StyleChain, Styles, Synthesize, elem,
};
use crate::introspection::{Count, Counter, CounterUpdate, Locatable};
use crate::layout::{BlockElem, Em, Length};
use crate::model::{Numbering, Outlinable, Refable, Supplement};
use crate::text::{FontWeight, LocalName, TextElem, TextSize};

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
#[elem(Locatable, Synthesize, Count, ShowSet, LocalName, Refable, Outlinable)]
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
    pub numbering: Option<Numbering>,

    /// Plain-text displayed numbering.
    ///
    /// This field is only necessary for creating PDF bookmarks.
    /// The problem is that in the export stage, we don't have access to the World/Engine etc.
    /// which is needed to resolve numbers and numbering patterns (or functions) into a concrete string/content.
    /// Therefore, we have to save the result before the export stage.
    #[internal]
    #[synthesized]
    pub numbering_displayed: EcoString,

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

        let numbering_displayed = if let (Some(numbering), Some(location)) =
            (self.numbering.get_ref(styles).as_ref(), self.location())
        {
            Some(
                self.counter()
                    .display_at_loc(engine, location, styles, numbering)?
                    .plain_text(),
            )
        } else {
            None
        };

        let elem = self.as_mut();
        elem.level.set(Smart::Custom(elem.resolve_level(styles)));
        elem.supplement
            .set(Smart::Custom(Some(Supplement::Content(supplement))));
        elem.numbering_displayed = numbering_displayed;
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
