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
/// #quote(author: [Plato])[
///   ... ἔοικα γοῦν τούτου γε σμικρῷ τινι αὐτῷ τούτῳ σοφώτερος εἶναι, ὅτι
///   ἃ μὴ οἶδα οὐδὲ οἴομαι εἰδέναι.
/// ]
/// #quote(author: [from the Henry Cary literal translation of 1897])[
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
    /// #quote(author: [René Descartes])[cogito, ergo sum]
    ///
    /// #set quote(block: true)
    /// #quote(author: [JFK])[Ich bin ein Berliner.]
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

    /// The source to this quote, can be a URL or bibliography key.
    ///
    /// ```example
    /// #show link: set text(blue)
    /// #quote(source: "https://typst.app/home")[Compose papers faster]
    ///
    /// #quote(source: <tolkien54>)[
    ///   You cannot pass... I am a servant of the Secret Fire, wielder of the
    ///   flame of Anor. You cannot pass. The dark fire will not avail you,
    ///   flame of Udûn. Go back to the Shadow! You cannot pass.
    /// ]
    /// #bibliography("works.bib")
    /// ```
    source: Option<Source>,

    /// The author of this quote. By default only displayed for block quotes.
    ///
    /// ```example
    /// #quote(author: [René Descartes])[cogito, ergo sum]
    ///
    /// #set quote(block: true)
    /// #quote(author: [René Descartes])[cogito, ergo sum]
    /// ```
    author: Option<Content>,

    /// The quote.
    #[required]
    body: Content,
}

#[derive(Debug, Clone)]
pub enum Source {
    Url(EcoString),
    Label(Label),
}

cast! {
    Source,
    self => match self {
        Self::Url(url) => url.into_value(),
        Self::Label(label) => label.into_value(),
    },
    url: EcoString => Self::Url(url),
    label: Label => Self::Label(label),
}

impl Show for QuoteElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();
        let author = self.author(styles);
        let block = self.block(styles);

        let (citation, url) = if let Some(source) = self.source(styles) {
            match source {
                Source::Url(url) => (None, Some(url)),
                Source::Label(label) => (Some(label.0), None),
            }
        } else {
            (None, None)
        };

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

            if let Some(author) = author {
                let mut seq = vec![TextElem::packed('—'), SpaceElem::new().pack()];

                if let Some(source) = citation {
                    vt.delayed(|vt| {
                        let bib =
                            BibliographyElem::find(vt.introspector).at(self.span())?;

                        // TODO: these should use the citation-format attribute once CSL is
                        //       implemented
                        match bib.style(styles) {
                            // author-date and author
                            BibliographyStyle::Apa
                            | BibliographyStyle::Mla
                            | BibliographyStyle::ChicagoAuthorDate => {
                                seq.push(
                                    CiteElem::new(vec![source])
                                        .with_brackets(false)
                                        .pack(),
                                );
                            }
                            // notes
                            BibliographyStyle::ChicagoNotes => {
                                seq.extend([author, CiteElem::new(vec![source]).pack()]);
                            }
                            // label and numeric
                            BibliographyStyle::Ieee => {
                                seq.extend([
                                    author,
                                    TextElem::packed(", "),
                                    CiteElem::new(vec![source]).pack(),
                                ]);
                            }
                        }

                        Ok(())
                    });
                } else {
                    seq.push(author);
                }

                // use v(1em, weak: true) bring the attribution closer to the quote
                let weak_v = VElem::weak(Spacing::Rel(Em::new(0.9).into())).pack();
                realized += weak_v + Content::sequence(seq).aligned(Align::END);
            } else if let Some(source) = citation {
                realized += CiteElem::new(vec![source]).pack();
            }

            realized = PadElem::new(realized).pack();
        }

        if let Some(url) = url {
            realized = realized.linked(Destination::Url(url));
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
