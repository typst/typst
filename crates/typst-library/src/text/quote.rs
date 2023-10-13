use super::{SmartquoteElem, SpaceElem, TextElem};
use crate::layout::{BlockElem, HElem, PadElem, Spacing, VElem};
use crate::meta::{BibliographyElem, BibliographyStyle, CiteElem};
use crate::prelude::*;

/// Displays a quote alongside it's author.
///
/// # Example
/// ```example
/// Plato is often misquoted as the author of #quote[I know that I know
/// nothing], however, this is a derivation form his orginal quote:
/// #set quote(block: true)
/// #quote(attribution: [Plato])[
///   ... ἔοικα γοῦν τούτου γε σμικρῷ τινι αὐτῷ τούτῳ σοφώτερος εἶναι, ὅτι
///   ἃ μὴ οἶδα οὐδὲ οἴομαι εἰδέναι.
/// ]
/// #quote(attribution: [from the Henry Cary literal translation of 1897])[
///   ... I seem, then, in just this little thing to be wiser than this man at
///   any rate, that what I do not know I do not think I know either.
/// ]
/// ```
///
/// By default block quotes are padded left and right by `{1em}`, alignment and
/// padding can be controlled with show rules:
/// ```example
/// #set quote(block: true)
/// #show quote: set align(center)
/// #show quote: set pad(x: 5em)
///
/// #quote[
///   You cannot pass... I am a servant of the Secret Fire, wielder of the
///   flame of Anor. You cannot pass. The dark fire will not avail you,
///   flame of Udûn. Go back to the Shadow! You cannot pass.
/// ]
/// ```
#[elem(Finalize, Show)]
pub struct QuoteElem {
    /// Whether this is a block quote.
    ///
    /// ```example
    /// #quote(attribution: [René Descartes])[cogito, ergo sum]
    ///
    /// #set quote(block: true)
    /// #quote(attribution: [JFK])[Ich bin ein Berliner.]
    /// ```
    block: bool,

    /// Whether double quotes should be added around this quote.
    ///
    /// The double quotes used are inferred from the `quotes` property on
    /// [smartquote]($smartquote), which is affected by the `lang` property on
    /// [text]($text).
    ///
    /// - `{true}`: Wrap this quote in double quotes.
    /// - `{false}`: Do not wrap this quote in double quotes.
    /// - `{auto}`: Infer whether to wrap this quote in double quotes based on
    ///   the `block` property. If `block` is `{false}`, double quotes are
    ///   auomatically added.
    ///
    /// ```example
    /// #set text(lang: "de")
    /// #quote[Ich bin ein Berliner.]
    ///
    /// #set text(lang: "en")
    /// #set quote(quotes: true)
    /// #quote(block: true)[I am a Berliner.]
    /// ```
    quotes: Smart<bool>,

    /// The attribution of this quote, usually the author or source. Can be a
    /// label pointing to a bibliography entry or any content. By default only
    /// displayed for block quotes, but can be changed using a `{show}` rule.
    ///
    /// ```example
    /// #quote(attribution: [René Descartes])[cogito, ergo sum] \
    ///
    /// #show quote.where(block: false): it => [
    ///   "#it.body"
    ///   #if it.attribution != none [(#it.attribution)]
    /// ]
    /// #quote(attribution: link("https://typst.app/home")[typst.com])[
    ///   Compose papers faster
    /// ]
    ///
    /// #set quote(block: true)
    /// #quote(attribution: <tolkien54>)[
    ///   You cannot pass... I am a servant of the Secret Fire, wielder of the
    ///   flame of Anor. You cannot pass. The dark fire will not avail you,
    ///   flame of Udûn. Go back to the Shadow! You cannot pass.
    /// ]
    /// #bibliography("works.bib", style: "apa")
    /// ```
    ///
    /// Note that bilbiography styles which do not include the author in the
    /// citation (label, numberic and notes) currently produce attributions such
    /// as `[---#super[1]]` or `[--- [1]]`, this will be fixed soon with CSL
    /// support. In the mean time you can simply cite yourself:
    /// ```example
    /// #set quote(block: true)
    /// #quote(attribution: [J. R. R. Tolkien, @tolkien54])[In a hole there lived a hobbit.]
    ///
    /// #bibliography("works.bib")
    /// ```
    attribution: Option<Attribution>,

    /// The quote.
    #[required]
    body: Content,
}

#[derive(Debug, Clone)]
pub enum Attribution {
    Content(Content),
    Label(Label),
}

cast! {
    Attribution,
    self => match self {
        Self::Content(content) => content.into_value(),
        Self::Label(label) => label.into_value(),
    },
    content: Content => Self::Content(content),
    label: Label => Self::Label(label),
}

impl Show for QuoteElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();
        let block = self.block(styles);

        if self.quotes(styles) == Smart::Custom(true) || !block {
            // use h(0pt, weak: true) to make the quotes "sticky"
            let quote = SmartquoteElem::new().with_double(true).pack();
            let weak_h = HElem::new(Spacing::Rel(Rel::zero())).with_weak(true).pack();

            realized = Content::sequence([
                quote.clone(),
                weak_h.clone(),
                realized,
                weak_h,
                quote,
            ]);
        }

        if block {
            realized = BlockElem::new().with_body(Some(realized)).pack();

            if let Some(attribution) = self.attribution(styles) {
                let mut seq = vec![TextElem::packed('—'), SpaceElem::new().pack()];

                match attribution {
                    Attribution::Content(content) => {
                        seq.push(content);
                    }
                    Attribution::Label(label) => {
                        let citation = vt.delayed(|vt| {
                            let citation = CiteElem::new(vec![label.0]);
                            let bib =
                                BibliographyElem::find(vt.introspector).at(self.span())?;

                            // TODO: these should use the citation-format attribute, once CSL
                            // is implemented and retrieve the authors for non-author formats
                            // themeselves, see:
                            // - https://github.com/typst/typst/pull/2252#issuecomment-1741146989
                            // - https://github.com/typst/typst/pull/2252#issuecomment-1744634132
                            Ok(match bib.style(styles) {
                                // author-date and author
                                BibliographyStyle::Apa
                                | BibliographyStyle::Mla
                                | BibliographyStyle::ChicagoAuthorDate => {
                                    citation.with_brackets(false).pack()
                                }
                                // notes, label and numeric
                                BibliographyStyle::ChicagoNotes
                                | BibliographyStyle::Ieee => citation.pack(),
                            })
                        });

                        seq.push(citation);
                    }
                }

                // use v(0.9em, weak: true) bring the attribution closer to the quote
                let weak_v = VElem::weak(Spacing::Rel(Em::new(0.9).into())).pack();
                realized += weak_v + Content::sequence(seq).aligned(Align::END);
            }

            realized = PadElem::new(realized).pack();
        } else if let Some(Attribution::Label(label)) = self.attribution(styles) {
            realized += SpaceElem::new().pack() + CiteElem::new(vec![label.0]).pack();
        }

        Ok(realized)
    }
}

impl Finalize for QuoteElem {
    fn finalize(&self, realized: Content, _: StyleChain) -> Content {
        let x = Em::new(1.0).into();
        let above = Em::new(2.4).into();
        let below = Em::new(1.8).into();
        realized
            .styled(PadElem::set_left(x))
            .styled(PadElem::set_right(x))
            .styled(BlockElem::set_above(VElem::block_around(above)))
            .styled(BlockElem::set_below(VElem::block_around(below)))
    }
}
