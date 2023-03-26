use super::{BibliographyElem, CiteElem};
use crate::meta::AnchorElem;
use crate::prelude::*;

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
#[element(Locatable, Synthesize, Show)]
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

impl Synthesize for RefElem {
    fn synthesize(&mut self, styles: StyleChain) {
        let citation = self.to_citation(styles);
        self.push_citation(Some(citation));
    }
}

impl Show for RefElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        // Don't run on the first introspector loop, since we don't have any meta elements yet.
        if !vt.introspector.init() {
            return Ok(Content::empty());
        }

        // Find the anchor the reference will link to.
        let target_anchor = self.find_target_anchor(vt);

        // If the target is a bibliography link, it requires special handling.
        if BibliographyElem::has(vt, &self.target().0) {
            if let Ok(None) = target_anchor {
                return Ok(self.to_citation(styles).pack());
            }

            bail!(self.span(), "label occurs in the document and its bibliography");
        }

        // At this point we have zero or one anchors. Ensure we have one.
        let Some((anchor, ref_name)) = target_anchor? else {
            bail!(self.span(), "label does not exist in the document");
        };

        // Finally, build the supplement from the anchor.
        let supplement = match self.supplement(styles) {
            Smart::Auto => ref_name,
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(Supplement::Content(content))) => content.clone(),
            Smart::Custom(Some(Supplement::Func(func))) => {
                func.call_vt(vt, [anchor.clone().pack().into()])?.display()
            }
        };

        Ok(supplement.linked(Destination::Location(anchor.0.location().unwrap())))
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

    /// Find the only valid anchor matching the target label.
    fn find_target_anchor(
        &self,
        vt: &mut Vt,
    ) -> SourceResult<Option<(AnchorElem, Content)>> {
        // Find all the anchor elements matching this label
        let target = self.target();
        let matches = vt.introspector.query(Selector::Elem(
            AnchorElem::func(),
            Some(dict! ("matched-label" => target.clone())),
        ));
        let matches = matches.iter().filter_map(AnchorElem::unpack);

        // Filter the matches to only include the valid anchors.
        let mut valid_anchors = matches
            .clone()
            .filter_map(|anchor| anchor.ref_name().map(|name| (anchor, name)));

        // Check the first valid anchor and:
        let (first_anchor, name) = match valid_anchors.next() {
            // if one exists, store it.
            Some((anchor, name)) => (anchor, name),
            // if no valid anchors exist, generate the errors for the invalid anchors (if any).
            None => {
                let invalid_anchors = matches
                    .filter_map(|anchor| match anchor.ref_name() {
                        Some(_) => None,
                        None => Some(anchor),
                    })
                    .collect::<Vec<_>>();

                match &*invalid_anchors {
                    [] => return Ok(None),
                    [anchor] => bail!(anchor.span(), "cannot reference this element"),
                    _ => {
                        bail!(self.span(), "label occurs multiple times in the document")
                    }
                }
            }
        };

        // At this point, at least one valid anchor exists. If there are more valid anchors
        // remaining in the iterator, it means we have an ambuguous reference.
        if let Some(_) = valid_anchors.next() {
            bail!(self.span(), "there are multiple anchors matching this label");
        }

        Ok(Some((first_anchor.clone(), name)))
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
