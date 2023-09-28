use typst::font::FontWeight;
use typst::util::option_eq;

use super::{Counter, CounterUpdate, LocalName, Numbering, Outlinable, Refable};
use crate::layout::{BlockElem, HElem, VElem};
use crate::meta::{Count, Supplement};
use crate::prelude::*;
use crate::text::{SpaceElem, TextElem, TextSize};

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
#[elem(Locatable, Synthesize, Count, Show, Finalize, LocalName, Refable, Outlinable)]
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
    /// to `{true}`, that is, if it would also be listed in Typst's
    /// [outline]($outline). Setting this property to either `{true}` (bookmark)
    /// or `{false}` (don't bookmark) bypasses that behavior.
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

impl Synthesize for HeadingElem {
    fn synthesize(&mut self, vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        // Resolve the supplement.
        let supplement = match self.supplement(styles) {
            Smart::Auto => TextElem::packed(self.local_name_in(styles)),
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(supplement)) => supplement.resolve(vt, [self.clone()])?,
        };

        self.push_level(self.level(styles));
        self.push_numbering(self.numbering(styles));
        self.push_supplement(Smart::Custom(Some(Supplement::Content(supplement))));
        self.push_outlined(self.outlined(styles));
        self.push_bookmarked(self.bookmarked(styles));

        Ok(())
    }
}

impl Show for HeadingElem {
    #[tracing::instrument(name = "HeadingElem::show", skip_all)]
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();
        if let Some(numbering) = self.numbering(styles) {
            realized = Counter::of(Self::elem())
                .display(Some(numbering), false)
                .spanned(self.span())
                + HElem::new(Em::new(0.3).into()).with_weak(true).pack()
                + realized;
        }
        Ok(BlockElem::new().with_body(Some(realized)).pack())
    }
}

impl Finalize for HeadingElem {
    fn finalize(&self, realized: Content, styles: StyleChain) -> Content {
        let level = self.level(styles).get();
        let scale = match level {
            1 => 1.4,
            2 => 1.2,
            _ => 1.0,
        };

        let size = Em::new(scale);
        let above = Em::new(if level == 1 { 1.8 } else { 1.44 }) / scale;
        let below = Em::new(0.75) / scale;

        let mut styles = Styles::new();
        styles.set(TextElem::set_size(TextSize(size.into())));
        styles.set(TextElem::set_weight(FontWeight::BOLD));
        styles.set(BlockElem::set_above(VElem::block_around(above.into())));
        styles.set(BlockElem::set_below(VElem::block_around(below.into())));
        styles.set(BlockElem::set_sticky(true));
        realized.styled_with_map(styles)
    }
}

impl Count for HeadingElem {
    fn update(&self) -> Option<CounterUpdate> {
        self.numbering(StyleChain::default())
            .is_some()
            .then(|| CounterUpdate::Step(self.level(StyleChain::default())))
    }
}

cast! {
    HeadingElem,
    v: Content => v.to::<Self>().ok_or("expected heading")?.clone(),
}

impl Refable for HeadingElem {
    fn supplement(&self) -> Content {
        // After synthesis, this should always be custom content.
        match self.supplement(StyleChain::default()) {
            Smart::Custom(Some(Supplement::Content(content))) => content,
            _ => Content::empty(),
        }
    }

    fn counter(&self) -> Counter {
        Counter::of(Self::elem())
    }

    fn numbering(&self) -> Option<Numbering> {
        self.numbering(StyleChain::default())
    }
}

impl Outlinable for HeadingElem {
    fn outline(&self, vt: &mut Vt) -> SourceResult<Option<Content>> {
        if !self.outlined(StyleChain::default()) {
            return Ok(None);
        }

        let mut content = self.body();
        if let Some(numbering) = self.numbering(StyleChain::default()) {
            let numbers = Counter::of(Self::elem())
                .at(vt, self.0.location().unwrap())?
                .display(vt, &numbering)?;
            content = numbers + SpaceElem::new().pack() + content;
        };

        Ok(Some(content))
    }

    fn level(&self) -> NonZeroUsize {
        self.level(StyleChain::default())
    }
}

impl LocalName for HeadingElem {
    fn local_name(&self, lang: Lang, region: Option<Region>) -> &'static str {
        match lang {
            Lang::ALBANIAN => "Kapitull",
            Lang::ARABIC => "الفصل",
            Lang::BOKMÅL => "Kapittel",
            Lang::CHINESE if option_eq(region, "TW") => "小節",
            Lang::CHINESE => "小节",
            Lang::CZECH => "Kapitola",
            Lang::DANISH => "Afsnit",
            Lang::DUTCH => "Hoofdstuk",
            Lang::FILIPINO => "Seksyon",
            Lang::FINNISH => "Osio",
            Lang::FRENCH => "Chapitre",
            Lang::GERMAN => "Abschnitt",
            Lang::HUNGARIAN => "Fejezet",
            Lang::ITALIAN => "Sezione",
            Lang::NYNORSK => "Kapittel",
            Lang::POLISH => "Sekcja",
            Lang::PORTUGUESE if option_eq(region, "PT") => "Secção",
            Lang::PORTUGUESE => "Seção",
            Lang::ROMANIAN => "Secțiunea",
            Lang::RUSSIAN => "Раздел",
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
