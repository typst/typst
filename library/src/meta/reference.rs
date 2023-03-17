use super::{BibliographyNode, CiteNode, Counter, LocalName, Numbering};
use crate::prelude::*;
use crate::text::TextNode;

/// A reference to a label.
///
/// The reference function produces a textual reference to a label. For example,
/// a reference to a heading will yield an appropriate string such as "Section
/// 1" for a reference to the first heading. The references are also links to
/// the respective element.
///
/// # Example
/// ```example
/// #set heading(numbering: "1.")
///
/// = Introduction <intro>
/// Recent developments in typesetting
/// software have rekindled hope in
/// previously frustrated researchers.
/// As shown in @results, we ...
///
/// = Results <results>
/// We evaluate our method in a
/// series of tests. @perf discusses
/// the performance aspects of ...
///
/// == Performance <perf>
/// As described in @intro, we ...
/// ```
///
/// ## Syntax
/// This function also has dedicated syntax: A reference to a label can be
/// created by typing an `@` followed by the name of the label (e.g.
/// `[= Introduction <intro>]` can be referenced by typing `[@intro]`).
///
/// Display: Reference
/// Category: meta
#[node(Locatable, Show)]
pub struct RefNode {
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
    /// Sections into Chapters.
    /// ```
    pub supplement: Smart<Option<Supplement>>,
}

impl Show for RefNode {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        if !vt.introspector.init() {
            return Ok(Content::empty());
        }

        let target = self.target();
        let matches = vt.introspector.query(Selector::Label(self.target()));

        if BibliographyNode::has(vt, &target.0) {
            if !matches.is_empty() {
                bail!(self.span(), "label occurs in the document and its bibliography");
            }

            return self.to_citation(styles).show(vt, styles);
        }

        let [node] = matches.as_slice() else {
            bail!(self.span(), if matches.is_empty() {
                "label does not exist in the document"
            } else {
                "label occurs multiple times in the document"
            });
        };

        if !node.can::<dyn Locatable>() {
            bail!(self.span(), "cannot reference {}", node.id().name);
        }

        let supplement = self.supplement(styles);
        let mut supplement = match supplement {
            Smart::Auto => node
                .with::<dyn LocalName>()
                .map(|node| node.local_name(TextNode::lang_in(styles)))
                .map(TextNode::packed)
                .unwrap_or_default(),
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(Supplement::Content(content))) => content.clone(),
            Smart::Custom(Some(Supplement::Func(func))) => {
                func.call_vt(vt, [node.clone().into()])?.display()
            }
        };

        if !supplement.is_empty() {
            supplement += TextNode::packed('\u{a0}');
        }

        let Some(numbering) = node.cast_field::<Numbering>("numbering") else {
            bail!(self.span(), "only numbered elements can be referenced");
        };

        let numbers =
            Counter::of(node.id()).resolve(vt, node.stable_id(), &numbering.trimmed())?;

        Ok((supplement + numbers).linked(Link::Node(node.stable_id().unwrap())))
    }
}

impl RefNode {
    /// Turn the rference into a citation.
    pub fn to_citation(&self, styles: StyleChain) -> CiteNode {
        let mut node = CiteNode::new(vec![self.target().0]);
        node.push_supplement(match self.supplement(styles) {
            Smart::Custom(Some(Supplement::Content(content))) => Some(content),
            _ => None,
        });
        node.0.set_stable_id(self.0.stable_id().unwrap());
        node
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
