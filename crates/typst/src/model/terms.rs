use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, Array, Content, NativeElement, Packed, Show, Smart, StyleChain,
    Styles,
};
use crate::layout::{Dir, Em, HElem, Length, Sides, StackChild, StackElem, VElem};
use crate::model::{ListItemLike, ListLike, ParElem};
use crate::text::TextElem;
use crate::utils::Numeric;

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
#[elem(scope, title = "Term List", Show)]
pub struct TermsElem {
    /// Defines the default [spacing]($terms.spacing) of the term list. If it is
    /// `{false}`, the items are spaced apart with
    /// [paragraph spacing]($par.spacing). If it is `{true}`, they use
    /// [paragraph leading]($par.leading) instead. This makes the list more
    /// compact, which can look better if the items are short.
    ///
    /// In markup mode, the value of this parameter is determined based on
    /// whether items are separated with a blank line. If items directly follow
    /// each other, this is set to `{true}`; if items are separated by a blank
    /// line, this is set to `{false}`. The markup-defined tightness cannot be
    /// overridden with set rules.
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
    #[borrowed]
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

    /// The spacing between the items of the term list.
    ///
    /// If set to `{auto}`, uses paragraph [`leading`]($par.leading) for tight
    /// term lists and paragraph [`spacing`]($par.spacing) for wide
    /// (non-tight) term lists.
    pub spacing: Smart<Length>,

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
    pub children: Vec<Packed<TermItem>>,
}

#[scope]
impl TermsElem {
    #[elem]
    type TermItem;
}

impl Show for Packed<TermsElem> {
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let separator = self.separator(styles);
        let indent = self.indent(styles);
        let hanging_indent = self.hanging_indent(styles);
        let gutter = self.spacing(styles).unwrap_or_else(|| {
            if self.tight(styles) {
                ParElem::leading_in(styles).into()
            } else {
                ParElem::spacing_in(styles).into()
            }
        });

        let pad = hanging_indent + indent;
        let unpad = (!hanging_indent.is_zero())
            .then(|| HElem::new((-hanging_indent).into()).pack());

        let mut children = vec![];
        for child in self.children().iter() {
            let mut seq = vec![];
            seq.extend(unpad.clone());
            seq.push(child.term().clone().strong());
            seq.push((*separator).clone());
            seq.push(child.description().clone());
            children.push(StackChild::Block(Content::sequence(seq)));
        }

        let mut padding = Sides::default();
        if TextElem::dir_in(styles) == Dir::LTR {
            padding.left = pad.into();
        } else {
            padding.right = pad.into();
        }

        let mut realized = StackElem::new(children)
            .with_spacing(Some(gutter.into()))
            .pack()
            .padded(padding);

        if self.tight(styles) {
            let leading = ParElem::leading_in(styles);
            let spacing =
                VElem::new(leading.into()).with_weak(true).with_attach(true).pack();
            realized = spacing + realized;
        }

        Ok(realized)
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
    v: Content => v.unpack::<Self>().map_err(|_| "expected term item or array")?,
}

impl ListLike for TermsElem {
    type Item = TermItem;

    fn create(children: Vec<Packed<Self::Item>>, tight: bool) -> Self {
        Self::new(children).with_tight(tight)
    }
}

impl ListItemLike for TermItem {
    fn styled(mut item: Packed<Self>, styles: Styles) -> Packed<Self> {
        item.term.style_in_place(styles.clone());
        item.description.style_in_place(styles);
        item
    }
}
