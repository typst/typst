use typst_syntax::Spanned;

use crate::diag::{error, At, HintedString, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Cast, Content, Derived, Label, Packed, Show, Smart, StyleChain,
    Synthesize,
};
use crate::introspection::Locatable;
use crate::model::bibliography::Works;
use crate::model::{CslSource, CslStyle};
use crate::text::{Lang, Region, TextElem};

/// Cite a work from the bibliography.
///
/// Before you starting citing, you need to add a [bibliography] somewhere in
/// your document. Note that you can cite using BibLaTeX or Hayagriva
/// bibliography file. For more information see [bibliography.summary].
///
/// # Example
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
/// # Syntax
/// This function indirectly has dedicated syntax. [References]($ref) can be
/// used to cite works from the bibliography. The label then corresponds to the
/// citation key.
#[elem(Synthesize)]
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
    ///   [bibliography's style]($bibliography.style) for citations.
    /// - A string with the name of one of the built-in styles (see below). Some
    ///   of the styles listed below appear twice, once with their full name and
    ///   once with a short alias.
    /// - A path string to a [CSL file](https://citationstyles.org/). For more
    ///   details about paths, see the [Paths section]($syntax/#paths).
    /// - Raw bytes from which a CSL style should be decoded.
    #[parse(match args.named::<Spanned<Smart<CslSource>>>("style")? {
        Some(Spanned { v: Smart::Custom(source), span }) => Some(Smart::Custom(
            CslStyle::load(engine.world, Spanned::new(source, span))?
        )),
        Some(Spanned { v: Smart::Auto, .. }) => Some(Smart::Auto),
        None => None,
    })]
    #[borrowed]
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
        elem.push_lang(TextElem::lang_in(styles));
        elem.push_region(TextElem::region_in(styles));
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

/// A group of citations.
///
/// This is automatically created from adjacent citations during show rule
/// application.
#[elem(Locatable, Show)]
pub struct CiteGroup {
    /// The citations.
    #[required]
    pub children: Vec<Packed<CiteElem>>,
}

impl Show for Packed<CiteGroup> {
    #[typst_macros::time(name = "cite", span = self.span())]
    fn show(&self, engine: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        let location = self.location().unwrap();
        let span = self.span();
        Works::generate(engine)
            .at(span)?
            .citations
            .get(&location)
            .cloned()
            .ok_or_else(failed_to_format_citation)
            .at(span)?
    }
}

/// The error message when a citation wasn't found in the pre-formatted list.
#[cold]
fn failed_to_format_citation() -> HintedString {
    error!(
        "cannot format citation in isolation";
        hint: "check whether this citation is measured \
               without being inserted into the document"
    )
}
