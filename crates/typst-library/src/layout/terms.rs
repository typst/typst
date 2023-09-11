use super::{HElem, VElem};
use crate::layout::{BlockElem, ParElem, Spacing};
use crate::prelude::*;

/// A list of terms and their descriptions.
///
/// Displays a sequence of terms and their descriptions vertically. When the
/// descriptions span over multiple lines, they use hanging indent to
/// communicate the visual hierarchy.
///
/// # Example
/// ```example
/// / Ligature: A merged glyph.
/// / Kerning: A spacing adjustment
///   between two adjacent letters.
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: Starting a line with a slash,
/// followed by a term, a colon and a description creates a term list item.
#[elem(scope, title = "Term List", Layout)]
pub struct TermsElem {
    /// If this is `{false}`, the items are spaced apart with
    /// [term list spacing]($terms.spacing). If it is `{true}`, they use normal
    /// [leading]($par.leading) instead. This makes the term list more compact,
    /// which can look better if the items are short.
    ///
    /// In markup mode, the value of this parameter is determined based on
    /// whether items are separated with a blank line. If items directly follow
    /// each other, this is set to `{true}`; if items are separated by a blank
    /// line, this is set to `{false}`.
    ///
    /// ```example
    /// / Fact: If a term list has a lot
    ///   of text, and maybe other inline
    ///   content, it should not be tight
    ///   anymore.
    ///
    /// / Tip: To make it wide, simply
    ///   insert a blank line between the
    ///   items.
    /// ```
    #[default(true)]
    pub tight: bool,

    /// The separator between the item and the description.
    ///
    /// If you want to just separate them with a certain amount of space, use
    /// `{h(2cm, weak: true)}` as the separator and replace `{2cm}` with your
    /// desired amount of space.
    ///
    /// ```example
    /// #set terms(separator: [: ])
    ///
    /// / Colon: A nice separator symbol.
    /// ```
    #[default(HElem::new(Em::new(0.6).into()).with_weak(true).pack())]
    pub separator: Content,

    /// The indentation of each item.
    pub indent: Length,

    /// The hanging indent of the description.
    ///
    /// This is in addition to the whole item's `indent`.
    ///
    /// ```example
    /// #set terms(hanging-indent: 0pt)
    /// / Term: This term list does not
    ///   make use of hanging indents.
    /// ```
    #[default(Em::new(2.0).into())]
    pub hanging_indent: Length,

    /// The spacing between the items of a wide (non-tight) term list.
    ///
    /// If set to `{auto}`, uses the spacing [below blocks]($block.below).
    pub spacing: Smart<Spacing>,

    /// The term list's children.
    ///
    /// When using the term list syntax, adjacent items are automatically
    /// collected into term lists, even through constructs like for loops.
    ///
    /// ```example
    /// #for (year, product) in (
    ///   "1978": "TeX",
    ///   "1984": "LaTeX",
    ///   "2019": "Typst",
    /// ) [/ #product: Born in #year.]
    /// ```
    #[variadic]
    pub children: Vec<TermItem>,
}

#[scope]
impl TermsElem {
    #[elem]
    type TermItem;
}

impl Layout for TermsElem {
    #[tracing::instrument(name = "TermsElem::layout", skip_all)]
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let separator = self.separator(styles);
        let indent = self.indent(styles);
        let hanging_indent = self.hanging_indent(styles);
        let gutter = if self.tight(styles) {
            ParElem::leading_in(styles).into()
        } else {
            self.spacing(styles)
                .unwrap_or_else(|| BlockElem::below_in(styles).amount())
        };

        let mut seq = vec![];
        for (i, child) in self.children().into_iter().enumerate() {
            if i > 0 {
                seq.push(VElem::new(gutter).with_weakness(1).pack());
            }
            if !indent.is_zero() {
                seq.push(HElem::new(indent.into()).pack());
            }
            seq.push(child.term().strong());
            seq.push(separator.clone());
            seq.push(child.description());
        }

        Content::sequence(seq)
            .styled(ParElem::set_hanging_indent(hanging_indent + indent))
            .layout(vt, styles, regions)
    }
}

/// A term list item.
#[elem(name = "item", title = "Term List Item")]
pub struct TermItem {
    /// The term described by the list item.
    #[required]
    pub term: Content,

    /// The description of the term.
    #[required]
    pub description: Content,
}

cast! {
    TermItem,
    array: Array => {
        let mut iter = array.into_iter();
        let (term, description) = match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => (a.cast()?, b.cast()?),
            _ => bail!("array must contain exactly two entries"),
        };
        Self::new(term, description)
    },
    v: Content => v.to::<Self>().cloned().ok_or("expected term item or array")?,
}
