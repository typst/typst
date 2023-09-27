use super::{Quotes, SmartquoteElem, SpaceElem, TextElem};
use crate::{
    layout::{BlockElem, PadElem},
    prelude::*,
};

#[elem(Finalize, Show)]
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
            Smart::Custom(dir) => dir,
        }
    }
}

// TODO: should alternative be respected when we don't even check for smart quotes to be enabled?
fn quote_body_explicit(body: Content, styles: StyleChain) -> Content {
    let quotes = SmartquoteElem::quotes_in(styles);
    let alternative = SmartquoteElem::alternative_in(styles);
    let lang = TextElem::lang_in(styles);
    let region = TextElem::region_in(styles);

    let Quotes { double_open, double_close, .. } =
        Quotes::new(&quotes, lang, region, alternative);

    Content::sequence([
        TextElem::packed(double_open),
        body,
        TextElem::packed(double_close),
    ])
}

fn pack_author(author: Content, dir: Dir, block: bool) -> Content {
    let mut seq = vec![author, SpaceElem::new().pack(), TextElem::packed('—')];

    if block {
        seq.push(SpaceElem::new().pack());
    }

    if dir == Dir::LTR {
        Content::sequence(seq.into_iter().rev())
    } else {
        Content::sequence(seq)
    }
}

impl Show for QuoteElem {
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();
        let author = self.author(styles);

        if self.quotes(styles) {
            realized = quote_body_explicit(realized, styles);
        }

        let dir = TextElem::dir_in(styles);

        if self.block(styles) {
            realized = BlockElem::new()
                .with_body(Some(realized))
                .pack()
                .aligned(Align::START);

            realized = PadElem::new(realized).pack();

            if let Some(author) = author {
                // the leading em dash is handled like punctuation when at the start or end of a
                // sequence and the sequence is reversed for us, so we don't reverse it ourselves
                realized += pack_author(author, Dir::LTR, true);
            }
        } else {
            if dir == Dir::LTR {
                if let Some(author) = author {
                    realized += SpaceElem::new().pack() + pack_author(author, dir, false);
                }
            } else {
                if let Some(author) = author {
                    realized = pack_author(author, dir, false)
                        + SpaceElem::new().pack()
                        + realized;
                }
            };
        };

        if let Some(source) = self.source(styles) {
            realized = realized.linked(Destination::Url(source));
        }

        Ok(realized)
    }
}

impl Finalize for QuoteElem {
    fn finalize(&self, mut realized: Content, styles: StyleChain) -> Content {
        let (start, end) = if TextElem::dir_in(styles) == Dir::LTR {
            (Side::Left, Side::Right)
        } else {
            (Side::Right, Side::Left)
        };

        let inset: Rel = Em::new(0.5).into();
        let width: Length = Em::new(0.25).into();

        let mut inset = Sides::splat(Some(inset));
        let mut stroke = Sides::splat(None);

        *inset.get_mut(end) = None;
        *stroke.get_mut(start) = Some(Some(Stroke {
            paint: Smart::Custom(Color::GRAY.into()),
            thickness: Smart::Custom(width),
            ..Stroke::default()
        }));

        realized = realized
            .styled(BlockElem::set_stroke(stroke))
            .styled(BlockElem::set_inset(inset));

        let pad = width * 0.5;
        let pad: Rel = pad.into();
        if start == Side::Left {
            realized.styled(PadElem::set_left(pad))
        } else {
            realized.styled(PadElem::set_right(pad))
        }
    }
}
