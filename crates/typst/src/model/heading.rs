use std::num::NonZeroUsize;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, NativeElement, Packed, Show, ShowSet, Smart, StyleChain, Styles,
    Synthesize,
};
use crate::introspection::{Count, Counter, CounterUpdate, Locatable};
use crate::layout::{BlockElem, Em, HElem, VElem};
use crate::model::{Numbering, Outlinable, Refable, Supplement};
use crate::text::{FontWeight, Lang, LocalName, Region, SpaceElem, TextElem, TextSize};
use crate::util::{option_eq, NonZeroExt};

/// A section heading.
///
/// With headings, you can structure your document into sections. Each heading
/// has a _level,_ which starts at one and is unbounded upwards. This level
/// indicates the logical role of the following content (section, subsection,
/// etc.)  A top-level heading indicates a top-level section of the document
/// (not the document's title).
///
/// Typst can automatically number your headings for you. To enable numbering,
/// specify how you want your headings to be numbered with a
/// [numbering pattern or function]($numbering).
///
/// Independently from the numbering, Typst can also automatically generate an
/// [outline]($outline) of all headings for you. To exclude one or more headings
/// from this outline, you can set the `outlined` parameter to `{false}`.
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
/// signs determines the heading's logical nesting depth.
#[elem(Locatable, Synthesize, Count, Show, ShowSet, LocalName, Refable, Outlinable)]
pub struct HeadingElem {
    /// The logical nesting depth of the heading, starting from one.
    #[default(NonZeroUsize::ONE)]
    pub level: NonZeroUsize,

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

    /// Whether the heading should appear in the [outline]($outline).
    ///
    /// Note that this property, if set to `{true}`, ensures the heading is also
    /// shown as a bookmark in the exported PDF's outline (when exporting to
    /// PDF). To change that behaviour, use the `bookmarked` property.
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
    /// to `{true}`, that is, if it would also be listed in Typst's
    /// [outline]($outline). Setting this property to either `{true}` (bookmark)
    /// or `{false}` (don't bookmark) bypasses that behaviour.
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

    /// The heading's title.
    #[required]
    pub body: Content,
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
                supplement.resolve(engine, [self.clone().pack()])?
            }
        };

        self.push_supplement(Smart::Custom(Some(Supplement::Content(supplement))));
        Ok(())
    }
}

impl Show for Packed<HeadingElem> {
    #[typst_macros::time(name = "heading", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body().clone();
        if let Some(numbering) = (**self).numbering(styles).as_ref() {
            realized = Counter::of(HeadingElem::elem())
                .at(engine, self.location().unwrap())?
                .display(engine, numbering)?
                .spanned(self.span())
                + HElem::new(Em::new(0.3).into()).with_weak(true).pack()
                + realized;
        }
        Ok(BlockElem::new().with_body(Some(realized)).pack().spanned(self.span()))
    }
}

impl ShowSet for Packed<HeadingElem> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let level = (**self).level(styles).get();
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
        out.set(BlockElem::set_above(VElem::block_around(above.into())));
        out.set(BlockElem::set_below(VElem::block_around(below.into())));
        out.set(BlockElem::set_sticky(true));
        out
    }
}

impl Count for Packed<HeadingElem> {
    fn update(&self) -> Option<CounterUpdate> {
        (**self)
            .numbering(StyleChain::default())
            .is_some()
            .then(|| CounterUpdate::Step((**self).level(StyleChain::default())))
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
    fn outline(&self, engine: &mut Engine) -> SourceResult<Option<Content>> {
        if !self.outlined(StyleChain::default()) {
            return Ok(None);
        }

        let mut content = self.body().clone();
        if let Some(numbering) = (**self).numbering(StyleChain::default()).as_ref() {
            let numbers = Counter::of(HeadingElem::elem())
                .at(engine, self.location().unwrap())?
                .display(engine, numbering)?;
            content = numbers + SpaceElem::new().pack() + content;
        };

        Ok(Some(content))
    }

    fn level(&self) -> NonZeroUsize {
        (**self).level(StyleChain::default())
    }
}

impl LocalName for Packed<HeadingElem> {
    fn local_name(lang: Lang, region: Option<Region>) -> &'static str {
        match lang {
            Lang::ALBANIAN => "Kapitull",
            Lang::ARABIC => "الفصل",
            Lang::BOKMÅL => "Kapittel",
            Lang::CATALAN => "Secció",
            Lang::CHINESE if option_eq(region, "TW") => "小節",
            Lang::CHINESE => "小节",
            Lang::CZECH => "Kapitola",
            Lang::DANISH => "Afsnit",
            Lang::DUTCH => "Hoofdstuk",
            Lang::ESTONIAN => "Peatükk",
            Lang::FILIPINO => "Seksyon",
            Lang::FINNISH => "Osio",
            Lang::FRENCH => "Chapitre",
            Lang::GERMAN => "Abschnitt",
            Lang::GREEK => "Κεφάλαιο",
            Lang::HUNGARIAN => "Fejezet",
            Lang::ITALIAN => "Sezione",
            Lang::NYNORSK => "Kapittel",
            Lang::POLISH => "Sekcja",
            Lang::PORTUGUESE if option_eq(region, "PT") => "Secção",
            Lang::PORTUGUESE => "Seção",
            Lang::ROMANIAN => "Secțiunea",
            Lang::RUSSIAN => "Раздел",
            Lang::SERBIAN => "Поглавље",
            Lang::SLOVENIAN => "Poglavje",
            Lang::SPANISH => "Sección",
            Lang::SWEDISH => "Kapitel",
            Lang::TURKISH => "Bölüm",
            Lang::UKRAINIAN => "Розділ",
            Lang::VIETNAMESE => "Phần", // TODO: This may be wrong.
            Lang::JAPANESE => "節",
            Lang::ENGLISH | _ => "Section",
        }
    }
}
