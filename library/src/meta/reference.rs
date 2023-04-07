use super::{BibliographyElem, CiteElem, Counter, Figurable, Numbering};
use crate::prelude::*;
use crate::text::TextElem;

/// A reference to a label or bibliography.
///
/// The reference function produces a textual reference to a label. For example,
/// a reference to a heading will yield an appropriate string such as "Section
/// 1" for a reference to the first heading. The references are also links to
/// the respective element. Reference syntax can also be used to
/// [cite]($func/cite) from a bibliography.
///
/// Referenceable elements include [headings]($func/heading),
/// [figures]($func/figure), and [equations]($func/equation). To create a custom
/// referenceable element like a theorem, you can create a figure of a custom
/// [`kind`]($func/figure.kind) and write a show rule for it. In the future,
/// there might be a more direct way to define a custom referenceable element.
///
/// If you just want to link to a labelled element and not get an automatic
/// textual reference, consider using the [`link`]($func/link) function instead.
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

    /// Content of the referee, it should be referable.
    ///
    /// ```example
    /// #set heading(numbering: (..nums) => {
    ///   nums.pos().map(str).join(".")
    ///   }, supplement: [Chapt])
    ///
    /// #show ref: it => {
    ///   if it.has("referee") and it.referee.func() == heading {
    ///     let referee = it.referee
    ///     "["
    ///     referee.supplement
    ///     "-"
    ///     numbering(referee.numbering, ..counter(heading).at(referee.location()))
    ///     "]"
    ///   } else {
    ///     it
    ///   }
    /// }
    ///
    /// = Introduction <intro>
    /// = Summary <sum>
    /// == Subsection <sub>
    /// @intro
    ///
    /// @sum
    ///
    /// @sub
    /// ```
    #[synthesized]
    pub referee: Option<Content>,
}

impl Synthesize for RefElem {
    fn synthesize(&mut self, vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        let citation = self.to_citation(vt, styles)?;
        self.push_citation(Some(citation));

        if !vt.introspector.init() {
            return Ok(());
        }

        // find the referee element
        let target = self.target();
        let elem = vt.introspector.query_label(&self.target());

        if BibliographyElem::has(vt, &target.0) {
            // only push bib element if it is not in the document
            if elem.is_err() {
                self.push_referee(Some(self.to_citation(vt, styles)?.pack()));
            }
        } else if let Ok(elem) = elem.at(self.span()) {
            if elem.can::<dyn Refable>() {
                self.push_referee(Some(elem));
            }
        }

        Ok(())
    }
}

impl Show for RefElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        if !vt.introspector.init() {
            return Ok(Content::empty());
        }

        let target = self.target();
        let elem = vt.introspector.query_label(&self.target());

        if BibliographyElem::has(vt, &target.0) {
            if elem.is_ok() {
                bail!(self.span(), "label occurs in the document and its bibliography");
            }

            return Ok(self.to_citation(vt, styles)?.pack());
        }

        let elem = elem.at(self.span())?;
        if !elem.can::<dyn Refable>() {
            if elem.can::<dyn Figurable>() {
                bail!(
                    self.span(),
                    "cannot reference {} directly, try putting it into a figure",
                    elem.func().name()
                );
            } else {
                bail!(self.span(), "cannot reference {}", elem.func().name());
            }
        }

        let supplement = match self.supplement(styles) {
            Smart::Auto | Smart::Custom(None) => None,
            Smart::Custom(Some(supplement)) => {
                Some(supplement.resolve(vt, [elem.clone().into()])?)
            }
        };

        let lang = TextElem::lang_in(styles);
        let reference = elem
            .with::<dyn Refable>()
            .expect("element should be refable")
            .reference(vt, supplement, lang)?;

        Ok(reference.linked(Destination::Location(elem.location().unwrap())))
    }
}

impl RefElem {
    /// Turn the reference into a citation.
    pub fn to_citation(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<CiteElem> {
        let mut elem = CiteElem::new(vec![self.target().0]);
        elem.0.set_location(self.0.location().unwrap());
        elem.synthesize(vt, styles)?;
        elem.push_supplement(match self.supplement(styles) {
            Smart::Custom(Some(Supplement::Content(content))) => Some(content),
            _ => None,
        });

        Ok(elem)
    }
}

/// Additional content for a reference.
pub enum Supplement {
    Content(Content),
    Func(Func),
}

impl Supplement {
    /// Tries to resolve the supplement into its content.
    pub fn resolve(
        &self,
        vt: &mut Vt,
        args: impl IntoIterator<Item = Value>,
    ) -> SourceResult<Content> {
        match self {
            Supplement::Content(content) => Ok(content.clone()),
            Supplement::Func(func) => func.call_vt(vt, args).map(|v| v.display()),
        }
    }

    /// Tries to get the content of the supplement.
    /// Returns `None` if the supplement is a function.
    pub fn as_content(self) -> Option<Content> {
        match self {
            Supplement::Content(content) => Some(content),
            _ => None,
        }
    }
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

/// Marks an element as being able to be referenced.
/// This is used to implement the `@ref` macro.
/// It is expected to build the [`Content`] that gets linked
/// by the [`RefElement`].
pub trait Refable {
    /// Tries to build a reference content for this element.
    ///
    /// # Arguments
    /// - `vt` - The virtual typesetter.
    /// - `styles` - The styles of the reference.
    /// - `location` - The location where the reference is being created.
    /// - `supplement` - The supplement of the reference.
    fn reference(
        &self,
        vt: &mut Vt,
        supplement: Option<Content>,
        lang: Lang,
    ) -> SourceResult<Content>;

    /// Tries to build an outline element for this element.
    /// If this returns `None`, the outline will not include this element.
    /// By default this just calls [`Refable::reference`].
    fn outline(&self, vt: &mut Vt, lang: Lang) -> SourceResult<Option<Content>> {
        self.reference(vt, None, lang).map(Some)
    }

    /// Returns the level of this element.
    /// This is used to determine the level of the outline.
    /// By default this returns `0`.
    fn level(&self) -> usize {
        0
    }

    /// Returns the numbering of this element.
    fn numbering(&self) -> Option<Numbering>;

    /// Returns the counter of this element.
    fn counter(&self) -> Counter;
}
