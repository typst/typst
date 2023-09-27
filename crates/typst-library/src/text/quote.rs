use super::{Quotes, SmartquoteElem, SpaceElem, TextElem};
use crate::{
    layout::{BlockElem, PadElem},
    prelude::*,
};

#[elem(Show)]
pub struct QuoteElem {
    /// Whether this is a block quote.
    #[default(false)]
    block: bool,

    /// Whether quotes should be added around the quote.
    ///
    /// - `{true}`: Wrap the quote in double quotes.
    /// - `{false}`: Do not wrap the quote in double quotes.
    /// - `{auto}`: Infer whether to wrap the quote in double quotes based on
    ///   the `block` property. If `block` is `{true}` no quotes are used.
    #[resolve]
    quotes: QuotesEnabled,

    /// The source url to this quote.
    source: Option<EcoString>,

    /// The author of this quote.
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
            // TODO: should alternative be respected when we don't even check for smart quotes to be
            // enabled?
            let quotes = SmartquoteElem::quotes_in(styles);
            let alternative = SmartquoteElem::alternative_in(styles);
            let lang = TextElem::lang_in(styles);
            let region = TextElem::region_in(styles);

            let Quotes { double_open, double_close, .. } =
                Quotes::new(&quotes, lang, region, alternative);

            realized = Content::sequence([
                TextElem::packed(double_open),
                realized,
                TextElem::packed(double_close),
            ]);
        }

        let dir = TextElem::dir_in(styles);

        if self.block(styles) {
            realized = BlockElem::new().with_body(Some(realized)).pack();

            if let Some(author) = author {
                let mut new = Content::empty();
                new += TextElem::packed('—');
                new += SpaceElem::new().pack();
                new += author;
                realized += new.aligned(Align::END);
            }

            let pad: Rel = Em::new(1.0).into();
            realized = PadElem::new(realized).with_left(pad).with_right(pad).pack();
        } else if let Some(author) = author {
            let inline = |mut first: Content, second: Content| {
                first += SpaceElem::new().pack();
                first += TextElem::packed('—');
                first += SpaceElem::new().pack();
                first + second
            };

            if dir == Dir::LTR {
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
