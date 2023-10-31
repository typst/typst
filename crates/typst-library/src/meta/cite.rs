use super::bibliography::Works;
use super::CslStyle;
use crate::prelude::*;
use crate::text::TextElem;

/// Cite a work from the bibliography.
///
/// Before you starting citing, you need to add a [bibliography]($bibliography)
/// somewhere in your document.
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
    /// Should be either `{auto}`, one of the built-in styles (see below) or a
    /// path to a [CSL file](https://citationstyles.org/). Some of the styles
    /// listed below appear twice, once with their full name and once with a
    /// short alias.
    ///
    /// When set to `{auto}`, automatically use the
    /// [bibliography's style]($bibliography.style) for the citations.
    #[parse(CslStyle::parse_smart(vm, args)?)]
    pub style: Smart<CslStyle>,

    /// The text language setting where the citation is.
    #[internal]
    #[synthesized]
    pub lang: Lang,

    /// The text region setting where the citation is.
    #[internal]
    #[synthesized]
    pub region: Option<Region>,
}

impl Synthesize for CiteElem {
    fn synthesize(&mut self, _vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        self.push_supplement(self.supplement(styles));
        self.push_form(self.form(styles));
        self.push_style(self.style(styles));
        self.push_lang(TextElem::lang_in(styles));
        self.push_region(TextElem::region_in(styles));
        Ok(())
    }
}

cast! {
    CiteElem,
    v: Content => v.to::<Self>().cloned().ok_or("expected citation")?,
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
    pub children: Vec<CiteElem>,
}

impl Show for CiteGroup {
    #[tracing::instrument(name = "CiteGroup::show", skip(self, vt))]
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(vt.delayed(|vt| {
            let location = self.0.location().unwrap();
            let span = self.span();
            Works::generate(vt.world, vt.introspector)
                .at(span)?
                .citations
                .get(&location)
                .cloned()
                .unwrap_or_else(|| {
                    bail!(span, "failed to format citation (this is a bug)")
                })
        }))
    }
}
