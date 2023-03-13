use super::{FigureNode, HeadingNode, Numbering};
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
#[node(Synthesize, Show)]
pub struct RefNode {
    /// The target label that should be referenced.
    #[required]
    pub target: Label,

    /// The prefix before the referenced number.
    ///
    /// ```example
    /// #set heading(numbering: "1.")
    /// #set ref(prefix: it => {
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
    pub prefix: Smart<Option<Func>>,

    /// All elements with the target label in the document.
    #[synthesized]
    pub matches: Vec<Content>,
}

impl Synthesize for RefNode {
    fn synthesize(&mut self, vt: &Vt, _: StyleChain) {
        let matches = vt
            .locate(Selector::Label(self.target()))
            .map(|(_, node)| node.clone())
            .collect();

        self.push_matches(matches);
    }
}

impl Show for RefNode {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let matches = self.matches();
        let [target] = matches.as_slice() else {
            if vt.locatable() {
                bail!(self.span(), if matches.is_empty() {
                    "label does not exist in the document"
                } else {
                    "label occurs multiple times in the document"
                });
            } else {
                return Ok(Content::empty());
            }
        };

        let mut prefix = match self.prefix(styles) {
            Smart::Auto => target
                .with::<dyn LocalName>()
                .map(|node| node.local_name(TextNode::lang_in(styles)))
                .map(TextNode::packed)
                .unwrap_or_default(),
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(func)) => {
                let args = Args::new(func.span(), [target.clone().into()]);
                func.call_detached(vt.world(), args)?.display()
            }
        };

        if !prefix.is_empty() {
            prefix += TextNode::packed('\u{a0}');
        }

        let formatted = if let Some(heading) = target.to::<HeadingNode>() {
            if let Some(numbering) = heading.numbering(StyleChain::default()) {
                let numbers = heading.numbers().unwrap();
                numbered(vt, prefix, &numbering, &numbers)?
            } else {
                bail!(self.span(), "cannot reference unnumbered heading");
            }
        } else if let Some(figure) = target.to::<FigureNode>() {
            if let Some(numbering) = figure.numbering(StyleChain::default()) {
                let number = figure.number().unwrap();
                numbered(vt, prefix, &numbering, &[number])?
            } else {
                bail!(self.span(), "cannot reference unnumbered figure");
            }
        } else {
            bail!(self.span(), "cannot reference {}", target.id().name);
        };

        let loc = target.expect_field::<Location>("location");
        Ok(formatted.linked(Destination::Internal(loc)))
    }
}

/// Generate a numbered reference like "Section 1.1".
fn numbered(
    vt: &Vt,
    prefix: Content,
    numbering: &Numbering,
    numbers: &[NonZeroUsize],
) -> SourceResult<Content> {
    Ok(prefix
        + match numbering {
            Numbering::Pattern(pattern) => {
                TextNode::packed(pattern.apply(&numbers, true))
            }
            Numbering::Func(_) => numbering.apply(vt.world(), &numbers)?.display(),
        })
}

/// The named with which an element is referenced.
pub trait LocalName {
    /// Get the name in the given language.
    fn local_name(&self, lang: Lang) -> &'static str;
}
