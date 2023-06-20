use std::str::FromStr;

use super::{Counter, Numbering, NumberingPattern};
use crate::layout::{HElem, ParElem};
use crate::prelude::*;
use crate::text::{SuperElem, TextElem, TextSize};
use crate::visualize::LineElem;

/// A footnote.
///
/// Include additional remarks and references on the same page with footnotes. A
/// footnote will insert a superscript number that links to the note at the
/// bottom of the page. Notes are numbered sequentially throughout your document
/// and can break across multiple pages.
///
/// To customize the appearance of the entry in the footnote listing, see
/// [`footnote.entry`]($func/footnote.entry). The footnote itself is realized as
/// a normal superscript, so you can use a set rule on the
/// [`super`]($func/super) function to customize it.
///
/// ## Example { #example }
/// ```example
/// Check the docs for more details.
/// #footnote[https://typst.app/docs]
/// ```
///
/// The footnote automatically attaches itself to the preceding word, even if
/// there is a space before it in the markup. To force space, you can use the
/// string `[#" "]` or explicit [horizontal spacing]($func/h).
///
/// _Note:_ Set and show rules in the scope where `footnote` is called may not
/// apply to the footnote's content. See [here][issue] more information.
///
/// [issue]: https://github.com/typst/typst/issues/1467#issuecomment-1588799440
///
/// Display: Footnote
/// Category: meta
#[element(Locatable, Synthesize, Show)]
#[scope(
    scope.define("entry", FootnoteEntry::func());
    scope
)]
pub struct FootnoteElem {
    /// How to number footnotes.
    ///
    /// By default, the footnote numbering continues throughout your document.
    /// If you prefer per-page footnote numbering, you can reset the footnote
    /// [counter]($func/counter) in the page [header]($func/page.header). In the
    /// future, there might be a simpler way to achieve this.
    ///
    /// ```example
    /// #set footnote(numbering: "*")
    ///
    /// Footnotes:
    /// #footnote[Star],
    /// #footnote[Dagger]
    /// ```
    #[default(Numbering::Pattern(NumberingPattern::from_str("1").unwrap()))]
    pub numbering: Numbering,

    /// The content to put into the footnote.
    #[required]
    pub body: Content,
}

impl Synthesize for FootnoteElem {
    fn synthesize(&mut self, _vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        self.push_numbering(self.numbering(styles));
        Ok(())
    }
}

impl Show for FootnoteElem {
    #[tracing::instrument(name = "FootnoteElem::show", skip_all)]
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let loc = self.0.location().unwrap();
        let numbering = self.numbering(styles);
        let counter = Counter::of(Self::func());
        let num = counter.at(vt, loc)?.display(vt, &numbering)?;
        let sup = SuperElem::new(num).pack();
        let hole = HElem::new(Abs::zero().into()).with_weak(true).pack();
        let loc = self.0.location().unwrap().variant(1);
        Ok(hole + sup.linked(Destination::Location(loc)))
    }
}

/// An entry in a footnote list.
///
/// This function is not intended to be called directly. Instead, it is used
/// in set and show rules to customize footnote listings.
///
/// ## Example { #example }
/// ```example
/// #show footnote.entry: set text(red)
///
/// My footnote listing
/// #footnote[It's down here]
/// has red text!
/// ```
///
/// Display: Footnote Entry
/// Category: meta
#[element(Show, Finalize)]
pub struct FootnoteEntry {
    /// The footnote for this entry. It's location can be used to determine
    /// the footnote counter state.
    ///
    /// ```example
    /// #show footnote.entry: it => {
    ///   let loc = it.note.location()
    ///   numbering(
    ///     "1: ",
    ///     ..counter(footnote).at(loc),
    ///   )
    ///   it.note.body
    /// }
    ///
    /// Customized #footnote[Hello]
    /// listing #footnote[World! 🌏]
    /// ```
    #[required]
    pub note: FootnoteElem,

    /// The separator between the document body and the footnote listing.
    ///
    /// ```example
    /// #set footnote.entry(
    ///   separator: repeat[.]
    /// )
    ///
    /// Testing a different separator.
    /// #footnote[
    ///   Unconventional, but maybe
    ///   not that bad?
    /// ]
    /// ```
    #[default(
        LineElem::new()
            .with_length(Ratio::new(0.3).into())
            .with_stroke(PartialStroke {
                thickness: Smart::Custom(Abs::pt(0.5).into()),
                ..Default::default()
            })
            .pack()
    )]
    pub separator: Content,

    /// The amount of clearance between the document body and the separator.
    ///
    /// ```example
    /// #set footnote.entry(clearance: 3em)
    ///
    /// Footnotes also need ...
    /// #footnote[
    ///   ... some space to breathe.
    /// ]
    /// ```
    #[default(Em::new(1.0).into())]
    #[resolve]
    pub clearance: Length,

    /// The gap between footnote entries.
    ///
    /// ```example
    /// #set footnote.entry(gap: 0.8em)
    ///
    /// Footnotes:
    /// #footnote[Spaced],
    /// #footnote[Apart]
    /// ```
    #[default(Em::new(0.5).into())]
    #[resolve]
    pub gap: Length,

    /// The indent of each footnote entry.
    ///
    /// ```example
    /// #set footnote.entry(indent: 0em)
    ///
    /// Footnotes:
    /// #footnote[No],
    /// #footnote[Indent]
    /// ```
    #[default(Em::new(1.0).into())]
    pub indent: Length,
}

impl Show for FootnoteEntry {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let note = self.note();
        let number_gap = Em::new(0.05);
        let numbering = note.numbering(StyleChain::default());
        let counter = Counter::of(FootnoteElem::func());
        let loc = note.0.location().unwrap();
        let num = counter.at(vt, loc)?.display(vt, &numbering)?;
        let sup = SuperElem::new(num)
            .pack()
            .linked(Destination::Location(loc))
            .backlinked(loc.variant(1));
        Ok(Content::sequence([
            HElem::new(self.indent(styles).into()).pack(),
            sup,
            HElem::new(number_gap.into()).with_weak(true).pack(),
            note.body(),
        ]))
    }
}

impl Finalize for FootnoteEntry {
    fn finalize(&self, realized: Content, _: StyleChain) -> Content {
        let text_size = Em::new(0.85);
        let leading = Em::new(0.5);
        realized
            .styled(ParElem::set_leading(leading.into()))
            .styled(TextElem::set_size(TextSize(text_size.into())))
    }
}

cast! {
    FootnoteElem,
    v: Content => v.to::<Self>().cloned().unwrap_or_else(|| Self::new(v.clone())),
}
