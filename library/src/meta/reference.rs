use super::{BibliographyElem, CiteElem, Counter, LocalName, Numbering};
use crate::prelude::*;
use crate::text::TextElem;

/// A reference to a label or bibliography.
///
/// The reference function produces a textual reference to a label. For example,
/// a reference to a heading will yield an appropriate string such as "Section
/// 1" for a reference to the first heading. The references are also links to
/// the respective element.
///
/// Reference syntax can also be used to [cite]($func/cite) from a bibliography.
///
/// # Example
/// ```example
/// #set heading(numbering: "1.")
/// #set math.equation(numbering: "(1)")
///
/// = Introduction <intro>
/// Recent developments in
/// typesetting software have
/// rekindled hope in previously
/// frustrated researchers. @distress
/// As shown in @results, we ...
///
/// = Results <results>
/// We discuss our approach in
/// comparison with others.
///
/// == Performance <perf>
/// @slow demonstrates what slow
/// software looks like.
/// $ O(n) = 2^n $ <slow>
///
/// #bibliography("works.bib")
/// ```
///
/// ## Syntax
/// This function also has dedicated syntax: A reference to a label can be
/// created by typing an `@` followed by the name of the label (e.g.
/// `[= Introduction <intro>]` can be referenced by typing `[@intro]`).
///
/// To customize the supplement, add content in square brackets after the
/// reference: `[@intro[Chapter]]`.
///
/// Display: Reference
/// Category: meta
#[element(Synthesize, Locatable, Show)]
pub struct RefElem {
    /// The target label that should be referenced.
    #[required]
    pub target: Label,

    /// A supplement for the reference.
    ///
    /// For references to headings or figures, this is added before the
    /// referenced number. For citations, this can be used to add a page number.
    ///
    /// ```example
    /// #set heading(numbering: "1.")
    /// #set ref(supplement: it => {
    ///   if it.func() == heading {
    ///     "Chapter"
    ///   } else {
    ///     "Thing"
    ///   }
    /// })
    ///
    /// = Introduction <intro>
    /// In @intro, we see how to turn
    /// Sections into Chapters. And
    /// in @intro[Part], it is done
    /// manually.
    /// ```
    pub supplement: Smart<Option<Supplement>>,

    /// A synthesized citation.
    #[synthesized]
    pub citation: Option<CiteElem>,
}

/// A citable element can impl this trait to set the supplement content
/// when it be referenced.
pub trait RefSupplement {
    fn ref_supplement(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content>;
}

impl Synthesize for RefElem {
    fn synthesize(&mut self, styles: StyleChain) {
        let citation = self.to_citation(styles);
        self.push_citation(Some(citation));
    }
}

impl Show for RefElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        if !vt.introspector.init() {
            return Ok(Content::empty());
        }

        let target = self.target();
        let matches = vt.introspector.query(Selector::Label(self.target()));

        if BibliographyElem::has(vt, &target.0) {
            if !matches.is_empty() {
                bail!(self.span(), "label occurs in the document and its bibliography");
            }

            return Ok(self.to_citation(styles).pack());
        }

        let [elem] = matches.as_slice() else {
            bail!(self.span(), if matches.is_empty() {
                "label does not exist in the document"
            } else {
                "label occurs multiple times in the document"
            });
        };

        if !elem.can::<dyn Locatable>() {
            bail!(self.span(), "cannot reference {}", elem.func().name());
        }

        let supplement = self.supplement(styles);
        let mut supplement = match supplement {
            Smart::Auto => {
                if let Some(elem) = elem.with::<dyn RefSupplement>() {
                    elem.ref_supplement(vt, styles)?
                } else {
                    elem.with::<dyn LocalName>()
                        .map(|elem| elem.local_name(TextElem::lang_in(styles)))
                        .map(TextElem::packed)
                        .unwrap_or_default()
                }
            }
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(Supplement::Content(content))) => content.clone(),
            Smart::Custom(Some(Supplement::Func(func))) => {
                func.call_vt(vt, [elem.clone().into()])?.display()
            }
        };

        if !supplement.is_empty() {
            supplement += TextElem::packed('\u{a0}');
        }

        let Some(numbering) = elem.cast_field::<Numbering>("numbering") else {
            bail!(self.span(), "only numbered elements can be referenced");
        };

        let numbers = Counter::of(elem.func())
            .at(vt, elem.location().unwrap())?
            .display(vt, &numbering.trimmed())?;

        Ok((supplement + numbers).linked(Destination::Location(elem.location().unwrap())))
    }
}

impl RefElem {
    /// Turn the reference into a citation.
    pub fn to_citation(&self, styles: StyleChain) -> CiteElem {
        let mut elem = CiteElem::new(vec![self.target().0]);
        elem.0.set_location(self.0.location().unwrap());
        elem.synthesize(styles);
        elem.push_supplement(match self.supplement(styles) {
            Smart::Custom(Some(Supplement::Content(content))) => Some(content),
            _ => None,
        });
        elem
    }
}

/// Additional content for a reference.
pub enum Supplement {
    Content(Content),
    Func(Func),
}

cast_from_value! {
    Supplement,
    v: Content => Self::Content(v),
    v: Func => Self::Func(v),
}

cast_to_value! {
    v: Supplement => match v {
        Supplement::Content(v) => v.into(),
        Supplement::Func(v) => v.into(),
    }
}
