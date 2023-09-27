use super::{SmartquoteElem, SpaceElem, TextElem};
use crate::{
    layout::{BlockElem, PadElem},
    prelude::*,
};

/// Displays a quote alongside it's author.
///
/// # Example
/// ```example
/// Plato is often misquoted as the author of #quote[I know that I know
/// nothing], however, this is a derivation form his orginal quote:
/// #quote(block: true, author: [Plato])[
///   ... ἔοικα γοῦν τούτου γε σμικρῷ τινι αὐτῷ τούτῳ σοφώτερος εἶναι, ὅτι
///   ἃ μὴ οἶδα οὐδὲ οἴομαι εἰδέναι.
/// ]
/// #quote(
///  block: true,
///  author: [from the Henry Cary literal translation of 1897]
/// )[
///   ... I seem, then, in just this little thing to be wiser than this man at
///   any rate, that what I do not know I do not think I know either.
/// ]
/// ```
#[elem(Show)]
pub struct QuoteElem {
    /// Whether this is a block quote.
    ///
    /// ```example
    /// #quote(author: [René Descartes])[cogito, ergo sum]
    ///
    /// #set quote(block: true)
    /// #quote(author: [JFK])[Ich bin ein Berliner.]
    /// ```
    #[default(false)]
    block: bool,

    /// Whether quotes should be added around the quote.
    ///
    /// - `{true}`: Wrap the quote in double quotes.
    /// - `{false}`: Do not wrap the quote in double quotes.
    /// - `{auto}`: Infer whether to wrap the quote in double quotes based on
    ///   the `block` property. If `block` is `{true}` no quotes are used.
    ///
    /// ```example
    /// #set text(lang: "de")
    /// #quote[Ich bin ein Berliner.]
    ///
    /// #set smartquote(quotes: "«»")
    /// #set quote(quotes: true)
    /// #quote(block: true)[
    ///   ... ἔοικα γοῦν τούτου γε σμικρῷ τινι αὐτῷ τούτῳ σοφώτερος εἶναι, ὅτι
    ///   ἃ μὴ οἶδα οὐδὲ οἴομαι εἰδέναι.
    /// ]
    /// ```
    #[resolve]
    quotes: QuotesEnabled,

    /// The source url to this quote.
    ///
    /// ```example
    /// #show link: set text(blue)
    /// #quote(source: "https://google.com")[cogito, ergo sum]
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

#[derive(Debug, Default)]
pub struct QuotesEnabled(pub Smart<bool>);

cast! {
    QuotesEnabled,
    self => self.0.into_value(),
    value: Smart<bool> => Self(value),
}

impl Resolve for QuotesEnabled {
    type Output = bool;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self.0 {
            Smart::Auto => !QuoteElem::block_in(styles),
            Smart::Custom(quotes) => quotes,
        }
    }
}

impl Show for QuoteElem {
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();
        let author = self.author(styles);

        if self.quotes(styles) {
            realized = Content::sequence([
                SmartquoteElem::new().with_double(true).pack(),
                realized,
                SmartquoteElem::new().with_double(true).pack(),
            ]);
        }

        if self.block(styles) {
            realized = BlockElem::new().with_body(Some(realized)).pack();

            if let Some(author) = author {
                realized += Content::sequence([
                    TextElem::packed('—'),
                    SpaceElem::new().pack(),
                    author,
                ])
                .aligned(Align::END);
            }

            let pad: Rel = Em::new(1.0).into();
            realized = PadElem::new(realized).with_left(pad).with_right(pad).pack();
        } else if let Some(author) = author {
            let inline = |first: Content, second: Content| {
                Content::sequence([
                    first,
                    SpaceElem::new().pack(),
                    TextElem::packed('—'),
                    SpaceElem::new().pack(),
                    second,
                ])
            };

            if TextElem::dir_in(styles) == Dir::LTR {
                realized = inline(realized, author);
            } else {
                realized = inline(author, realized);
            }
        }

        if let Some(source) = self.source(styles) {
            realized = realized.linked(Destination::Url(source));
        }

        Ok(realized)
    }
}
