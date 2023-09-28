use super::{SmartquoteElem, SpaceElem, TextElem};
use crate::layout::{BlockElem, HElem, PadElem, Spacing};
use crate::meta::CiteElem;
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
    /// #quote(source: "tolkien54")[
    ///   You cannot pass... I am a servant of the Secret Fire, wielder of the
    ///   flame of Anor. You cannot pass. The dark fire will not avail you,
    ///   flame of Udûn. Go back to the Shadow! You cannot pass.
    /// ]
    /// #bibliography("works.bib")
    /// ```
    source: Option<EcoString>,

    /// The author of this quote.
    ///
    /// ```example
    /// #quote(author: [René Descartes])[cogito, ergo sum] is the author. \
    /// #quote[cogito, ergo sum] --- _Unknown_
    /// ```
    author: Option<Content>,

    /// The quote.
    #[required]
    body: Content,
}

impl Show for QuoteElem {
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();
        let author = self.author(styles);
        let block = self.block(styles);

        let (citation, url) = if let Some(source) = self.source(styles) {
            if source.starts_with("http://") || source.starts_with("https://") {
                (None, Some(source))
            } else {
                (Some(source), None)
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

        let author = if let Some(author) = author {
            let mut seq = vec![];

            let space = SpaceElem::new().pack();
            if !block {
                seq.push(space.clone());
            }

            seq.extend([TextElem::packed('—'), space, author]);

            if let Some(source) = citation {
                realized += CiteElem::new(vec![source]).pack();
            }

            Some(Content::sequence(seq))
        } else {
            None
        };

        if block {
            realized = BlockElem::new().with_body(Some(realized)).pack();

            if let Some(author) = author {
                realized += author.aligned(Align::END);
            }

            realized = PadElem::new(realized).pack();
        } else if let Some(author) = author {
            realized += author;
        }

        if let Some(url) = url {
            realized = realized.linked(Destination::Url(url));
        }

        Ok(realized)
    }
}

impl Finalize for QuoteElem {
    fn finalize(&self, realized: Content, _: StyleChain) -> Content {
        let pad: Rel = Em::new(1.0).into();
        realized
            .styled(PadElem::set_left(pad))
            .styled(PadElem::set_right(pad))
    }
}
