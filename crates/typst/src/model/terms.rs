use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, Array, Content, NativeElement, Packed, Show, ShowSet, Smart,
    StyleChain, Styles,
};
use crate::introspection::{Locatable, Locator};
use crate::layout::{
    layout_fragment, BlockElem, Em, Fragment, HElem, Length, Regions, Sides, VElem,
};
use crate::model::ParElem;
use crate::text::{isolate, TextElem};

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
#[elem(scope, title = "Term List", Locatable, Show, ShowSet)]
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
    #[borrowed]
    pub separator: Content,

    /// The indentation of the term list.
    ///
    /// ```example
    /// #set terms(indent: 0pt)
    /// / Term: This term list does not
    ///   make use of indents.
    /// ```
    #[default(Em::new(1.75).into())]
    pub indent: Length,

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
        let indent = self.indent(styles);
        let inset = Sides::one(Some(indent.into()), TextElem::dir_in(styles).start());

        let mut realized = BlockElem::multi_layouter(self.clone(), layout_terms)
            .with_inset(inset)
            .pack()
            .spanned(self.span());

        if self.tight(styles) {
            let leading = ParElem::leading_in(styles);
            let spacing = VElem::list_attach(leading.into()).pack();
            realized = spacing + realized;
        }

        Ok(realized)
    }
}

impl ShowSet for Packed<TermsElem> {
    fn show_set(&self, _: StyleChain) -> Styles {
        let mut out = Styles::new();
        out.set(ParElem::set_first_line_indent(Length::zero()));
        out
    }
}

/// Layout the term list.
#[typst_macros::time(span = elem.span())]
fn layout_terms(
    elem: &Packed<TermsElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let separator = elem.separator(styles);
    let indent = elem.indent(styles);
    let spacing = elem.spacing(styles).unwrap_or_else(|| {
        if elem.tight(styles) {
            ParElem::leading_in(styles).into()
        } else {
            ParElem::spacing_in(styles).into()
        }
    });

    let spacing_elem = VElem::block_spacing(spacing.into()).pack();
    let dedent_elem = HElem::new((-indent).into()).pack();

    let mut seq = vec![];
    for (i, child) in elem.children.iter().enumerate() {
        if i > 0 {
            seq.push(spacing_elem.clone());
        }

        seq.push(dedent_elem.clone());
        isolate(&mut seq, child.term().clone().strong(), styles);
        seq.push(separator.clone());
        seq.push(child.description().clone());
    }

    let realized = Content::sequence(seq);
    layout_fragment(engine, &realized, locator, styles, regions)
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

impl Packed<TermItem> {
    /// Apply styles to this term item.
    pub fn styled(mut self, styles: Styles) -> Self {
        self.term.style_in_place(styles.clone());
        self.description.style_in_place(styles);
        self
    }
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
