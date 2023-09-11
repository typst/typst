use comemo::Prehashed;
use std::str::FromStr;

use super::{Counter, Numbering, NumberingPattern};
use crate::layout::{HElem, ParElem};
use crate::meta::{Count, CounterUpdate};
use crate::prelude::*;
use crate::text::{SuperElem, TextElem, TextSize};
use crate::visualize::LineElem;

/// The body of a footnote can be either some content or a label referencing
/// another footnote.
#[derive(Debug)]
pub enum FootnoteBody {
    Content(Content),
    Reference(Label),
}

cast! {
    FootnoteBody,
    self => match self {
        Self::Content(v) => v.into_value(),
        Self::Reference(v) => v.into_value(),
    },
    v: Content => Self::Content(v),
    v: Label => Self::Reference(v),
}

/// A footnote.
///
/// Includes additional remarks and references on the same page with footnotes.
/// A footnote will insert a superscript number that links to the note at the
/// bottom of the page. Notes are numbered sequentially throughout your document
/// and can break across multiple pages.
///
/// To customize the appearance of the entry in the footnote listing, see
/// [`footnote.entry`]($footnote.entry). The footnote itself is realized as a
/// normal superscript, so you can use a set rule on the [`super`]($super)
/// function to customize it.
///
/// # Example
/// ```example
/// Check the docs for more details.
/// #footnote[https://typst.app/docs]
/// ```
///
/// The footnote automatically attaches itself to the preceding word, even if
/// there is a space before it in the markup. To force space, you can use the
/// string `[#" "]` or explicit [horizontal spacing]($h).
///
/// By giving a label to a footnote, you can have multiple references to it.
///
/// ```example
/// You can edit Typst documents online.
/// #footnote[https://typst.app/app] <fn>
/// Checkout Typst's website. @fn
/// And the online app. #footnote(<fn>)
/// ```
///
/// _Note:_ Set and show rules in the scope where `footnote` is called may not
/// apply to the footnote's content. See [here][issue] for more information.
///
/// [issue]: https://github.com/typst/typst/issues/1467#issuecomment-1588799440
#[elem(scope, Locatable, Synthesize, Show, Count)]
pub struct FootnoteElem {
    /// How to number footnotes.
    ///
    /// By default, the footnote numbering continues throughout your document.
    /// If you prefer per-page footnote numbering, you can reset the footnote
    /// [counter]($counter) in the page [header]($page.header). In the future,
    /// there might be a simpler way to achieve this.
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

    /// The content to put into the footnote. Can also be the label of another
    /// footnote this one should point to.
    #[required]
    pub body: FootnoteBody,
}

#[scope]
impl FootnoteElem {
    #[elem]
    type FootnoteEntry;
}

impl FootnoteElem {
    /// Creates a new footnote that the passed content as its body.
    pub fn with_content(content: Content) -> Self {
        Self::new(FootnoteBody::Content(content))
    }

    /// Creates a new footnote referencing the footnote with the specified label.
    pub fn with_label(label: Label) -> Self {
        Self::new(FootnoteBody::Reference(label))
    }

    /// Tests if this footnote is a reference to another footnote.
    pub fn is_ref(&self) -> bool {
        matches!(self.body(), FootnoteBody::Reference(_))
    }

    /// Returns the content of the body of this footnote if it is not a ref.
    pub fn body_content(&self) -> Option<Content> {
        match self.body() {
            FootnoteBody::Content(content) => Some(content),
            _ => None,
        }
    }

    /// Returns the location of the definition of this footnote.
    pub fn declaration_location(&self, vt: &Vt) -> StrResult<Location> {
        match self.body() {
            FootnoteBody::Reference(label) => {
                let element: Prehashed<Content> = vt.introspector.query_label(&label)?;
                let footnote = element
                    .to::<FootnoteElem>()
                    .ok_or("referenced element should be a footnote")?;
                footnote.declaration_location(vt)
            }
            _ => Ok(self.0.location().unwrap()),
        }
    }
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
        Ok(vt.delayed(|vt| {
            let loc = self.declaration_location(vt).at(self.span())?;
            let numbering = self.numbering(styles);
            let counter = Counter::of(Self::elem());
            let num = counter.at(vt, loc)?.display(vt, &numbering)?;
            let sup = SuperElem::new(num).pack();
            let hole = HElem::new(Abs::zero().into()).with_weak(true).pack();
            let loc = loc.variant(1);
            Ok(hole + sup.linked(Destination::Location(loc)))
        }))
    }
}

impl Count for FootnoteElem {
    fn update(&self) -> Option<CounterUpdate> {
        (!self.is_ref()).then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

/// An entry in a footnote list.
///
/// This function is not intended to be called directly. Instead, it is used
/// in set and show rules to customize footnote listings.
///
/// _Note:_ Set and show rules for `footnote.entry` must be defined at the
/// beginning of the document in order to work correctly.
/// See [here](https://github.com/typst/typst/issues/1348#issuecomment-1566316463)
/// for more information.
///
/// ```example
/// #show footnote.entry: set text(red)
///
/// My footnote listing
/// #footnote[It's down here]
/// has red text!
/// ```
#[elem(name = "entry", title = "Footnote Entry", Show, Finalize)]
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
            .with_stroke(Stroke {
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
        let counter = Counter::of(FootnoteElem::elem());
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
            note.body_content().unwrap(),
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
    v: Content => v.to::<Self>().cloned().unwrap_or_else(|| Self::with_content(v.clone())),
}
