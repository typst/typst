use typst_syntax::Spanned;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    Cast, Content, Derived, Label, Packed, Smart, StyleChain, Synthesize, cast, elem,
};
use crate::model::bibliography::Works;
use crate::model::{CslSource, CslStyle};
use crate::text::{Lang, Region, TextElem};

/// Cite a work from the bibliography.
///
/// Before you starting citing, you need to add a @bibliography[bibliography]
/// somewhere in your document.
///
/// = Example <example>
/// ```example
/// This was already noted by
/// pirates long ago. @arrgh
///
/// Multiple sources say ...
/// @arrgh @netwok.
///
/// You can also call `cite`
/// explicitly. #cite(<arrgh>)
///
/// #bibliography("works.bib")
/// ```
///
/// If your source name contains certain characters such as slashes, which are
/// not recognized by the `<>` syntax, you can explicitly call `label` instead.
///
/// ```typ
/// Computer Modern is an example of a modernist serif typeface.
/// #cite(label("DBLP:books/lib/Knuth86a")).
/// >>> #bibliography("works.bib")
/// ```
///
/// = Syntax <syntax>
/// This function indirectly has dedicated syntax. @ref[References] can be used
/// to cite works from the bibliography. The label then corresponds to the
/// citation key.
#[elem(Locatable, Synthesize)]
pub struct CiteElem {
    /// The citation key that identifies the entry in the bibliography that
    /// shall be cited, as a label.
    ///
    /// ```example
    /// // All the same
    /// @netwok \
    /// #cite(<netwok>) \
    /// #cite(label("netwok"))
    /// >>> #set text(0pt)
    /// >>> #bibliography("works.bib", style: "apa")
    /// ```
    #[required]
    pub key: Label,

    /// A supplement for the citation such as page or chapter number.
    ///
    /// In reference syntax, the supplement can be added in square brackets:
    ///
    /// ```example
    /// This has been proven. @distress[p.~7]
    ///
    /// #bibliography("works.bib")
    /// ```
    pub supplement: Option<Content>,

    /// The kind of citation to produce. Different forms are useful in different
    /// scenarios: A normal citation is useful as a source at the end of a
    /// sentence, while a "prose" citation is more suitable for inclusion in the
    /// flow of text.
    ///
    /// If set to `{none}`, the cited work is included in the bibliography, but
    /// nothing will be displayed.
    ///
    /// ```example
    /// #cite(<netwok>, form: "prose")
    /// show the outsized effects of
    /// pirate life on the human psyche.
    /// >>> #set text(0pt)
    /// >>> #bibliography("works.bib", style: "apa")
    /// ```
    #[default(Some(CitationForm::Normal))]
    pub form: Option<CitationForm>,

    /// The citation style.
    ///
    /// This can be:
    /// - `{auto}` to automatically use the
    ///   @bibliography.style[bibliography's style] for citations.
    /// - A string with the name of one of the built-in styles (see below). Some
    ///   of the styles listed below appear twice, once with their full name and
    ///   once with a short alias.
    /// - A path string or @path to a
    ///   #link("https://citationstyles.org/")[CSL file].
    /// - Raw bytes from which a CSL style should be decoded.
    #[parse(match args.named::<Spanned<Smart<CslSource>>>("style")? {
        Some(Spanned { v: Smart::Custom(source), span }) => Some(Smart::Custom(
            CslStyle::load(engine, Spanned::new(source, span))?
        )),
        Some(Spanned { v: Smart::Auto, .. }) => Some(Smart::Auto),
        None => None,
    })]
    pub style: Smart<Derived<CslSource, CslStyle>>,

    /// The text language setting where the citation is.
    #[internal]
    #[synthesized]
    pub lang: Lang,

    /// The text region setting where the citation is.
    #[internal]
    #[synthesized]
    pub region: Option<Region>,
}

impl Synthesize for Packed<CiteElem> {
    fn synthesize(&mut self, _: &mut Engine, styles: StyleChain) -> SourceResult<()> {
        let elem = self.as_mut();
        elem.lang = Some(styles.get(TextElem::lang));
        elem.region = Some(styles.get(TextElem::region));
        Ok(())
    }
}

cast! {
    CiteElem,
    v: Content => v.unpack::<Self>().map_err(|_| "expected citation")?,
}

/// The form of the citation.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum CitationForm {
    /// Display in the standard way for the active style.
    #[default]
    Normal,
    /// Produces a citation that is suitable for inclusion in a sentence.
    Prose,
    /// Mimics a bibliography entry, with full information about the cited work.
    Full,
    /// Shows only the cited work's author(s).
    Author,
    /// Shows only the cited work's year.
    Year,
}

/// A group of consecutive citations.
///
/// This element is automatically created from adjacent citations during
/// realization. Citations are grouped without regard to which bibliography they
/// end up being assigned to. They are only split into subgroups during
/// bibliography assignment within the call tree of [`Works::generate`].
///
/// Each subgroup created there may consist of one or multiple citations that
/// are processed as a union by hayagriva. If hayagriva were to support a single
/// citation group for multiple bibliographies, the subgroup concept could be
/// removed, but it's unclear whether that can be reasonably supported.
///
/// Another alternative would have been to already segment by assigned
/// bibliography when grouping consecutive citations into [`CiteGroup`]s during
/// realization. This would be quite clean, but unfortunately it would incur one
/// additional document iteration, which is too high a price to pay for the
/// conceptual cleanliness.
///
/// The citation group element is purposefully kept internal to retain
/// flexibility in how it is collected.
#[elem(Locatable)]
pub struct CiteGroup {
    /// Holds citations and potentially spaces in between them. The spaces are
    /// retained so that they can be correctly rendered between subgroups
    /// assigned to different bibliographies.
    #[required]
    pub children: Vec<Content>,
}

impl Packed<CiteGroup> {
    pub fn realize(&self, engine: &mut Engine) -> SourceResult<Content> {
        let loc = self.location().unwrap();
        let span = self.span();
        Works::generate(engine, span)?.citation(loc, span)
    }
}
