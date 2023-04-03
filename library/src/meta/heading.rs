use typst::font::FontWeight;

use super::{Counter, CounterUpdate, LocalName, Numbering, Refable};
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
/// [numbering pattern or function]($func/numbering).
///
/// Independently from the numbering, Typst can also automatically generate an
/// [outline]($func/outline) of all headings for you. To exclude one or more
/// headings from this outline, you can set the `outlined` parameter to
/// `{false}`.
///
/// ## Example
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
/// ## Syntax
/// Headings have dedicated syntax: They can be created by starting a line with
/// one or multiple equals signs, followed by a space. The number of equals
/// signs determines the heading's logical nesting depth.
///
/// Display: Heading
/// Category: meta
#[element(Locatable, Synthesize, Count, Show, Finalize, LocalName, Refable)]
pub struct HeadingElem {
    /// The logical nesting depth of the heading, starting from one.
    #[default(NonZeroUsize::ONE)]
    pub level: NonZeroUsize,

    /// How to number the heading. Accepts a
    /// [numbering pattern or function]($func/numbering).
    ///
    /// ```example
    /// #set heading(numbering: "1.a.")
    ///
    /// = A section
    /// == A subsection
    /// === A sub-subsection
    /// ```
    pub numbering: Option<Numbering>,

    /// Whether the heading should appear in the outline.
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

    /// A supplement for the heading.
    ///
    /// For references to headings, this is added before the
    /// referenced number.
    ///
    /// ```example
    /// #set heading(numbering: "1.", supplement: "Chapter")
    ///
    /// = Introduction <intro>
    /// In @intro, we see how to turn
    /// Sections into Chapters. And
    /// in @intro[Part], it is done
    /// manually.
    /// ```
    pub supplement: Smart<Option<Supplement>>,

    /// The heading's title.
    #[required]
    pub body: Content,
}

impl Synthesize for HeadingElem {
    fn synthesize(&mut self, styles: StyleChain) -> SourceResult<()> {
        self.push_level(self.level(styles));
        self.push_numbering(self.numbering(styles));
        self.push_outlined(self.outlined(styles));

        Ok(())
    }
}

impl Show for HeadingElem {
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();
        if let Some(numbering) = self.numbering(styles) {
            realized = Counter::of(Self::func())
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

cast_from_value! {
    HeadingElem,
    v: Content => v.to::<Self>().ok_or("expected heading")?.clone(),
}

impl Refable for HeadingElem {
    fn reference(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        supplement: Option<Content>,
    ) -> SourceResult<Content> {
        // first we create the supplement of the heading
        let mut supplement = supplement.unwrap_or_else(|| {
            TextElem::packed(self.local_name(TextElem::lang_in(styles)))
        });

        // we append a space if the supplement is not empty
        if !supplement.is_empty() {
            supplement += TextElem::packed('\u{a0}')
        };

        // we check for a numbering
        let Some(numbering) = self.numbering(styles) else {
            bail!(self.span(), "only numbered headings can be referenced");
        };

        // we get the counter and display it
        let numbers = Counter::of(Self::func())
            .at(vt, self.0.location().expect("missing location"))?
            .display(vt, &numbering.trimmed())?;

        Ok(supplement + numbers)
    }

    fn level(&self, styles: StyleChain) -> usize {
        self.level(styles).get()
    }

    fn location(&self) -> Option<Location> {
        self.0.location()
    }

    fn outline(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Option<Content>> {
        // we check if the heading is outlined
        if !self.outlined(styles) {
            return Ok(None);
        }

        // We build the numbering followed by the title
        let mut start = self.body();
        if let Some(numbering) = self.numbering(StyleChain::default()) {
            let numbers = Counter::of(HeadingElem::func())
                .at(vt, self.location().expect("missing location"))?
                .display(vt, &numbering)?;
            start = numbers + SpaceElem::new().pack() + start;
        };

        Ok(Some(start))
    }
}

impl LocalName for HeadingElem {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::CHINESE => "小节",
            Lang::FRENCH => "Chapitre",
            Lang::GERMAN => "Abschnitt",
            Lang::ITALIAN => "Sezione",
            Lang::PORTUGUESE => "Seção",
            Lang::RUSSIAN => "Раздел",
            Lang::ENGLISH | _ => "Section",
        }
    }
}
