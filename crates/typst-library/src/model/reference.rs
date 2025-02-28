use comemo::Track;
use ecow::eco_format;

use crate::diag::{bail, At, Hint, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Cast, Content, Context, Func, IntoValue, Label, NativeElement, Packed,
    Show, Smart, StyleChain, Synthesize,
};
use crate::introspection::{Counter, CounterKey, Locatable};
use crate::math::EquationElem;
use crate::model::{
    BibliographyElem, CiteElem, Destination, Figurable, FootnoteElem, Numbering,
};
use crate::text::TextElem;

/// A reference to a label or bibliography.
///
/// Takes a label and cross-references it. There are two kind of references,
/// determined by its [`form`]($ref.form): `{"normal"}` and `{"page"}`.
///
/// The default, a `{"normal"}` reference, produces a textual reference to a
/// label. For example, a reference to a heading will yield an appropriate
/// string such as "Section 1" for a reference to the first heading. The
/// references are also links to the respective element. Reference syntax can
/// also be used to [cite] from a bibliography.
///
/// As the default form requires a supplement and numbering, the label must be
/// attached to a _referenceable element_. Referenceable elements include
/// [headings]($heading), [figures]($figure), [equations]($math.equation), and
/// [footnotes]($footnote). To create a custom referenceable element like a
/// theorem, you can create a figure of a custom [`kind`]($figure.kind) and
/// write a show rule for it. In the future, there might be a more direct way
/// to define a custom referenceable element.
///
/// If you just want to link to a labelled element and not get an automatic
/// textual reference, consider using the [`link`] function instead.
///
/// A `{"page"}` reference produces a page reference to a label, displaying the
/// page number at its location. You can use the
/// [page's supplement]($page.supplement) to modify the text before the page
/// number. Unlike a `{"normal"}` reference, the label can be attached to any
/// element.
///
/// # Example
/// ```example
/// #set page(numbering: "1")
/// #set heading(numbering: "1.")
/// #set math.equation(numbering: "(1)")
///
/// = Introduction <intro>
/// Recent developments in
/// typesetting software have
/// rekindled hope in previously
/// frustrated researchers. @distress
/// As shown in @results (see
/// #ref(<results>, form: "page")),
/// we ...
///
/// = Results <results>
/// We discuss our approach in
/// comparison with others.
///
/// == Performance <perf>
/// @slow demonstrates what slow
/// software looks like.
/// $ T(n) = O(2^n) $ <slow>
///
/// #bibliography("works.bib")
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: A `{"normal"}` reference to a
/// label can be created by typing an `@` followed by the name of the label
/// (e.g. `[= Introduction <intro>]` can be referenced by typing `[@intro]`).
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
///     link(el.location(),numbering(
///       el.numbering,
///       ..counter(eq).at(el.location())
///     ))
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
    ///
    /// Can be a label that is defined in the document or, if the
    /// [`form`]($ref.form) is set to `["normal"]`, an entry from the
    /// [`bibliography`].
    #[required]
    pub target: Label,

    /// A supplement for the reference.
    ///
    /// If the [`form`]($ref.form) is set to `{"normal"}`:
    /// - For references to headings or figures, this is added before the
    ///   referenced number.
    /// - For citations, this can be used to add a page number.
    ///
    /// If the [`form`]($ref.form) is set to `{"page"}`, then this is added
    /// before the page number of the label referenced.
    ///
    /// If a function is specified, it is passed the referenced element and
    /// should return content.
    ///
    /// ```example
    /// #set heading(numbering: "1.")
    /// #show ref.where(
    ///   form: "normal"
    /// ): set ref(supplement: it => {
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
    #[borrowed]
    pub supplement: Smart<Option<Supplement>>,

    /// The kind of reference to produce.
    ///
    /// ```example
    /// #set page(numbering: "1")
    ///
    /// Here <here> we are on
    /// #ref(<here>, form: "page").
    /// ```
    #[default(RefForm::Normal)]
    pub form: RefForm,

    /// A synthesized citation.
    #[synthesized]
    pub citation: Option<Packed<CiteElem>>,

    /// The referenced element.
    #[synthesized]
    pub element: Option<Content>,
}

impl Synthesize for Packed<RefElem> {
    fn synthesize(
        &mut self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let citation = to_citation(self, engine, styles)?;

        let elem = self.as_mut();
        elem.push_citation(Some(citation));
        elem.push_element(None);

        if !BibliographyElem::has(engine, elem.target) {
            if let Ok(found) = engine.introspector.query_label(elem.target).cloned() {
                elem.push_element(Some(found));
                return Ok(());
            }
        }

        Ok(())
    }
}

impl Show for Packed<RefElem> {
    #[typst_macros::time(name = "ref", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let elem = engine.introspector.query_label(self.target);
        let span = self.span();

        let form = self.form(styles);
        if form == RefForm::Page {
            let elem = elem.at(span)?;
            let elem = elem.clone();

            let loc = elem.location().unwrap();
            let numbering = engine
                .introspector
                .page_numbering(loc)
                .ok_or_else(|| eco_format!("cannot reference without page numbering"))
                .hint(eco_format!(
                    "you can enable page numbering with `#set page(numbering: \"1\")`"
                ))
                .at(span)?;
            let supplement = engine.introspector.page_supplement(loc);

            return show_reference(
                self,
                engine,
                styles,
                Counter::new(CounterKey::Page),
                numbering.clone(),
                supplement,
                elem,
            );
        }
        // RefForm::Normal

        if BibliographyElem::has(engine, self.target) {
            if elem.is_ok() {
                bail!(span, "label occurs in the document and its bibliography");
            }

            return Ok(to_citation(self, engine, styles)?.pack().spanned(span));
        }

        let elem = elem.at(span)?;

        if let Some(footnote) = elem.to_packed::<FootnoteElem>() {
            return Ok(footnote.into_ref(self.target).pack().spanned(span));
        }

        let elem = elem.clone();
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
                eco_format!("cannot reference {} without numbering", elem.func().name())
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

        show_reference(
            self,
            engine,
            styles,
            refable.counter(),
            numbering.clone(),
            refable.supplement(),
            elem,
        )
    }
}

/// Show a reference.
fn show_reference(
    reference: &Packed<RefElem>,
    engine: &mut Engine,
    styles: StyleChain,
    counter: Counter,
    numbering: Numbering,
    supplement: Content,
    elem: Content,
) -> SourceResult<Content> {
    let loc = elem.location().unwrap();
    let numbers = counter.display_at_loc(engine, loc, styles, &numbering.trimmed())?;

    let supplement = match reference.supplement(styles).as_ref() {
        Smart::Auto => supplement,
        Smart::Custom(None) => Content::empty(),
        Smart::Custom(Some(supplement)) => supplement.resolve(engine, styles, [elem])?,
    };

    let mut content = numbers;
    if !supplement.is_empty() {
        content = supplement + TextElem::packed("\u{a0}") + content;
    }

    Ok(content.linked(Destination::Location(loc)))
}

/// Turn a reference into a citation.
fn to_citation(
    reference: &Packed<RefElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Packed<CiteElem>> {
    let mut elem = Packed::new(CiteElem::new(reference.target).with_supplement(
        match reference.supplement(styles).clone() {
            Smart::Custom(Some(Supplement::Content(content))) => Some(content),
            _ => None,
        },
    ));

    if let Some(loc) = reference.location() {
        elem.set_location(loc);
    }

    elem.synthesize(engine, styles)?;

    Ok(elem)
}

/// Additional content for a reference.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Supplement {
    Content(Content),
    Func(Func),
}

impl Supplement {
    /// Tries to resolve the supplement into its content.
    pub fn resolve<T: IntoValue>(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        args: impl IntoIterator<Item = T>,
    ) -> SourceResult<Content> {
        Ok(match self {
            Supplement::Content(content) => content.clone(),
            Supplement::Func(func) => func
                .call(engine, Context::new(None, Some(styles)).track(), args)?
                .display(),
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

/// The form of the reference.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum RefForm {
    /// Produces a textual reference to a label.
    #[default]
    Normal,
    /// Produces a page reference to a label.
    Page,
}

/// Marks an element as being able to be referenced. This is used to implement
/// the `@ref` element.
pub trait Refable {
    /// The supplement, if not overridden by the reference.
    fn supplement(&self) -> Content;

    /// Returns the counter of this element.
    fn counter(&self) -> Counter;

    /// Returns the numbering of this element.
    fn numbering(&self) -> Option<&Numbering>;
}
