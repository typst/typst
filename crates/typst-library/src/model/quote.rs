use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Content, Depth, Label, NativeElement, Packed, Show, ShowSet, Smart,
    StyleChain, Styles, TargetElem,
};
use crate::html::{tag, HtmlElem};
use crate::introspection::Locatable;
use crate::layout::{
    Alignment, BlockBody, BlockElem, Em, HElem, PadElem, Spacing, VElem,
};
use crate::model::{CitationForm, CiteElem};
use crate::text::{SmartQuoteElem, SmartQuotes, SpaceElem, TextElem};

/// Displays a quote alongside an optional attribution.
///
/// # Example
/// ```example
/// Plato is often misquoted as the author of #quote[I know that I know
/// nothing], however, this is a derivation form his original quote:
///
/// #set quote(block: true)
///
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
#[elem(Locatable, ShowSet, Show)]
pub struct QuoteElem {
    /// Whether this is a block quote.
    ///
    /// ```example
    /// An inline citation would look like
    /// this: #quote(
    ///   attribution: [René Descartes]
    /// )[
    ///   cogito, ergo sum
    /// ], and a block equation like this:
    /// #quote(
    ///   block: true,
    ///   attribution: [JFK]
    /// )[
    ///   Ich bin ein Berliner.
    /// ]
    /// ```
    block: bool,

    /// Whether double quotes should be added around this quote.
    ///
    /// The double quotes used are inferred from the `quotes` property on
    /// [smartquote], which is affected by the `lang` property on [text].
    ///
    /// - `{true}`: Wrap this quote in double quotes.
    /// - `{false}`: Do not wrap this quote in double quotes.
    /// - `{auto}`: Infer whether to wrap this quote in double quotes based on
    ///   the `block` property. If `block` is `{false}`, double quotes are
    ///   automatically added.
    ///
    /// ```example
    /// #set text(lang: "de")
    ///
    /// Ein deutsch-sprechender Author
    /// zitiert unter umständen JFK:
    /// #quote[Ich bin ein Berliner.]
    ///
    /// #set text(lang: "en")
    ///
    /// And an english speaking one may
    /// translate the quote:
    /// #quote[I am a Berliner.]
    /// ```
    quotes: Smart<bool>,

    /// The attribution of this quote, usually the author or source. Can be a
    /// label pointing to a bibliography entry or any content. By default only
    /// displayed for block quotes, but can be changed using a `{show}` rule.
    ///
    /// ```example
    /// #quote(attribution: [René Descartes])[
    ///   cogito, ergo sum
    /// ]
    ///
    /// #show quote.where(block: false): it => {
    ///   ["] + h(0pt, weak: true) + it.body + h(0pt, weak: true) + ["]
    ///   if it.attribution != none [ (#it.attribution)]
    /// }
    ///
    /// #quote(
    ///   attribution: link("https://typst.app/home")[typst.com]
    /// )[
    ///   Compose papers faster
    /// ]
    ///
    /// #set quote(block: true)
    ///
    /// #quote(attribution: <tolkien54>)[
    ///   You cannot pass... I am a servant
    ///   of the Secret Fire, wielder of the
    ///   flame of Anor. You cannot pass. The
    ///   dark fire will not avail you, flame
    ///   of Udûn. Go back to the Shadow! You
    ///   cannot pass.
    /// ]
    ///
    /// #bibliography("works.bib", style: "apa")
    /// ```
    #[borrowed]
    attribution: Option<Attribution>,

    /// The quote.
    #[required]
    body: Content,

    /// The nesting depth.
    #[internal]
    #[fold]
    #[ghost]
    depth: Depth,
}

/// Attribution for a [quote](QuoteElem).
#[derive(Debug, Clone, PartialEq, Hash)]
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

impl Show for Packed<QuoteElem> {
    #[typst_macros::time(name = "quote", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body.clone();
        let block = self.block(styles);
        let html = TargetElem::target_in(styles).is_html();

        if self.quotes(styles) == Smart::Custom(true) || !block {
            let quotes = SmartQuotes::get(
                SmartQuoteElem::quotes_in(styles),
                TextElem::lang_in(styles),
                TextElem::region_in(styles),
                SmartQuoteElem::alternative_in(styles),
            );

            // Alternate between single and double quotes.
            let Depth(depth) = QuoteElem::depth_in(styles);
            let double = depth % 2 == 0;

            if !html {
                // Add zero-width weak spacing to make the quotes "sticky".
                let hole = HElem::hole().pack();
                realized = Content::sequence([hole.clone(), realized, hole]);
            }
            realized = Content::sequence([
                TextElem::packed(quotes.open(double)),
                realized,
                TextElem::packed(quotes.close(double)),
            ])
            .styled(QuoteElem::set_depth(Depth(1)));
        }

        if block {
            realized = if html {
                HtmlElem::new(tag::blockquote).with_body(Some(realized)).pack()
            } else {
                BlockElem::new().with_body(Some(BlockBody::Content(realized))).pack()
            }
            .spanned(self.span());

            if let Some(attribution) = self.attribution(styles).as_ref() {
                let attribution = match attribution {
                    Attribution::Content(content) => content.clone(),
                    Attribution::Label(label) => CiteElem::new(*label)
                        .with_form(Some(CitationForm::Prose))
                        .pack()
                        .spanned(self.span()),
                };
                let attribution =
                    [TextElem::packed('—'), SpaceElem::shared().clone(), attribution];

                if !html {
                    // Use v(0.9em, weak: true) to bring the attribution closer
                    // to the quote.
                    let gap = Spacing::Rel(Em::new(0.9).into());
                    let v = VElem::new(gap).with_weak(true).pack();
                    realized += v;
                }
                realized += Content::sequence(attribution).aligned(Alignment::END);
            }

            if !html {
                realized = PadElem::new(realized).pack();
            }
        } else if let Some(Attribution::Label(label)) = self.attribution(styles) {
            realized += SpaceElem::shared().clone()
                + CiteElem::new(*label).pack().spanned(self.span());
        }

        Ok(realized)
    }
}

impl ShowSet for Packed<QuoteElem> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        if self.block(styles) {
            out.set(PadElem::set_left(Em::new(1.0).into()));
            out.set(PadElem::set_right(Em::new(1.0).into()));
            out.set(BlockElem::set_above(Smart::Custom(Em::new(2.4).into())));
            out.set(BlockElem::set_below(Smart::Custom(Em::new(1.8).into())));
        }
        out
    }
}
