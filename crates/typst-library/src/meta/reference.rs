use super::{BibliographyElem, CiteElem, Counter, Figurable, Numbering};
use crate::math::EquationElem;
use crate::meta::FootnoteElem;
use crate::prelude::*;
use crate::text::TextElem;

/// A reference to a label or bibliography.
///
/// Produces a textual reference to a label. For example, a reference to a
/// heading will yield an appropriate string such as "Section 1" for a reference
/// to the first heading. The references are also links to the respective
/// element. Reference syntax can also be used to [cite]($cite) from a
/// bibliography.
///
/// Referenceable elements include [headings]($heading), [figures]($figure),
/// [equations]($math.equation), and [footnotes]($footnote). To create a custom
/// referenceable element like a theorem, you can create a figure of a custom
/// [`kind`]($figure.kind) and write a show rule for it. In the future, there
/// might be a more direct way to define a custom referenceable element.
///
/// If you just want to link to a labelled element and not get an automatic
/// textual reference, consider using the [`link`]($link) function instead.
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
/// # Syntax
/// This function also has dedicated syntax: A reference to a label can be
/// created by typing an `@` followed by the name of the label (e.g.
/// `[= Introduction <intro>]` can be referenced by typing `[@intro]`).
///
/// To customize the supplement, add content in square brackets after the
/// reference: `[@intro[Chapter]]`.
///
/// # Customization
/// If you write a show rule for references, you can access the referenced
/// element through the `element` field of the reference. The `element` may
/// be `{none}` even if it exists if Typst hasn't discovered it yet, so you
/// always need to handle that case in your code.
///
/// ```example
/// #set heading(numbering: "1.")
/// #set math.equation(numbering: "(1)")
///
/// #show ref: it => {
///   let eq = math.equation
///   let el = it.element
///   if el != none and el.func() == eq {
///     // Override equation references.
///     numbering(
///       el.numbering,
///       ..counter(eq).at(el.location())
///     )
///   } else {
///     // Other references as usual.
///     it
///   }
/// }
///
/// = Beginnings <beginning>
/// In @beginning we prove @pythagoras.
/// $ a^2 + b^2 = c^2 $ <pythagoras>
/// ```
#[elem(title = "Reference", Synthesize, Locatable, Show)]
pub struct RefElem {
    /// The target label that should be referenced.
    #[required]
    pub target: Label,

    /// A supplement for the reference.
    ///
    /// For references to headings or figures, this is added before the
    /// referenced number. For citations, this can be used to add a page number.
    ///
    /// If a function is specified, it is passed the referenced element and
    /// should return content.
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

    /// The referenced element.
    #[synthesized]
    pub element: Option<Content>,
}

impl Synthesize for RefElem {
    fn synthesize(&mut self, vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        let citation = self.to_citation(vt, styles)?;
        self.push_citation(Some(citation));
        self.push_element(None);

        let target = self.target();
        if !BibliographyElem::has(vt, &target.0) {
            if let Ok(elem) = vt.introspector.query_label(&target) {
                self.push_element(Some(elem.into_inner()));
                return Ok(());
            }
        }

        Ok(())
    }
}

impl Show for RefElem {
    #[tracing::instrument(name = "RefElem::show", skip_all)]
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        Ok(vt.delayed(|vt| {
            let target = self.target();
            let elem = vt.introspector.query_label(&self.target());
            let span = self.span();

            if BibliographyElem::has(vt, &target.0) {
                if elem.is_ok() {
                    bail!(span, "label occurs in the document and its bibliography");
                }

                return Ok(self.to_citation(vt, styles)?.pack().spanned(span));
            }

            let elem = elem.at(span)?;

            if elem.func() == FootnoteElem::elem() {
                return Ok(FootnoteElem::with_label(target).pack().spanned(span));
            }

            let refable = elem
                .with::<dyn Refable>()
                .ok_or_else(|| {
                    if elem.can::<dyn Figurable>() {
                        eco_format!(
                            "cannot reference {} directly, try putting it into a figure",
                            elem.func().name()
                        )
                    } else {
                        eco_format!("cannot reference {}", elem.func().name())
                    }
                })
                .at(span)?;

            let numbering = refable
                .numbering()
                .ok_or_else(|| {
                    eco_format!(
                        "cannot reference {} without numbering",
                        elem.func().name()
                    )
                })
                .hint(eco_format!(
                    "you can enable {} numbering with `#set {}(numbering: \"1.\")`",
                    elem.func().name(),
                    if elem.func() == EquationElem::elem() {
                        "math.equation"
                    } else {
                        elem.func().name()
                    }
                ))
                .at(span)?;

            let numbers = refable
                .counter()
                .at(vt, elem.location().unwrap())?
                .display(vt, &numbering.trimmed())?;

            let supplement = match self.supplement(styles) {
                Smart::Auto => refable.supplement(),
                Smart::Custom(None) => Content::empty(),
                Smart::Custom(Some(supplement)) => {
                    supplement.resolve(vt, [(*elem).clone()])?
                }
            };

            let mut content = numbers;
            if !supplement.is_empty() {
                content = supplement + TextElem::packed("\u{a0}") + content;
            }

            Ok(content.linked(Destination::Location(elem.location().unwrap())))
        }))
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
    pub fn resolve<T: IntoValue>(
        &self,
        vt: &mut Vt,
        args: impl IntoIterator<Item = T>,
    ) -> SourceResult<Content> {
        Ok(match self {
            Supplement::Content(content) => content.clone(),
            Supplement::Func(func) => func.call_vt(vt, args)?.display(),
        })
    }
}

cast! {
    Supplement,
    self => match self {
        Self::Content(v) => v.into_value(),
        Self::Func(v) => v.into_value(),
    },
    v: Content => Self::Content(v),
    v: Func => Self::Func(v),
}

/// Marks an element as being able to be referenced. This is used to implement
/// the `@ref` element.
pub trait Refable {
    /// The supplement, if not overridden by the reference.
    fn supplement(&self) -> Content;

    /// Returns the counter of this element.
    fn counter(&self) -> Counter;

    /// Returns the numbering of this element.
    fn numbering(&self) -> Option<Numbering>;
}
