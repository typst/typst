use crate::layout::{BlockNode, GridLayouter, HNode, ParNode, Sizing, Spacing};
use crate::prelude::*;
use crate::text::{SpaceNode, TextNode};

/// A list of terms and their descriptions.
///
/// Displays a sequence of terms and their descriptions vertically. When the
/// descriptions span over multiple lines, they use hanging indent to
/// communicate the visual hierarchy.
///
/// ## Syntax
/// This function also has dedicated syntax: Starting a line with a slash,
/// followed by a term, a colon and a description creates a term list item.
///
/// ## Example
/// ```example
/// / Ligature: A merged glyph.
/// / Kerning: A spacing adjustment
///   between two adjacent letters.
/// ```
///
/// Display: Term List
/// Category: layout
#[node(Layout)]
pub struct TermsNode {
    /// The term list's children.
    ///
    /// When using the term list syntax, adjacent items are automatically
    /// collected into term lists, even through constructs like for loops.
    ///
    /// ```example
    /// #for year, product in (
    ///   "1978": "TeX",
    ///   "1984": "LaTeX",
    ///   "2019": "Typst",
    /// ) [/ #product: Born in #year.]
    /// ```
    #[variadic]
    pub items: Vec<TermItem>,

    /// If this is `{false}`, the items are spaced apart with [term list
    /// spacing]($func/terms.spacing). If it is `{true}`, they use normal
    /// [leading]($func/par.leading) instead. This makes the term list more
    /// compact, which can look better if the items are short.
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
    #[named]
    #[default(true)]
    pub tight: bool,

    /// The indentation of each item's term.
    #[settable]
    #[resolve]
    #[default]
    pub indent: Length,

    /// The hanging indent of the description.
    ///
    /// ```example
    /// #set terms(hanging-indent: 0pt)
    /// / Term: This term list does not
    ///   make use of hanging indents.
    /// ```
    #[settable]
    #[resolve]
    #[default(Em::new(1.0).into())]
    pub hanging_indent: Length,

    /// The spacing between the items of a wide (non-tight) term list.
    ///
    /// If set to `{auto}`, uses the spacing [below blocks]($func/block.below).
    #[settable]
    #[default]
    pub spacing: Smart<Spacing>,
}

impl Layout for TermsNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let indent = styles.get(Self::INDENT);
        let body_indent = styles.get(Self::HANGING_INDENT);
        let gutter = if self.tight() {
            styles.get(ParNode::LEADING).into()
        } else {
            styles
                .get(Self::SPACING)
                .unwrap_or_else(|| styles.get(BlockNode::BELOW).amount())
        };

        let mut cells = vec![];
        for item in self.items() {
            let body = Content::sequence(vec![
                HNode::new((-body_indent).into()).pack(),
                (item.term() + TextNode::packed(':')).strong(),
                SpaceNode::new().pack(),
                item.description(),
            ]);

            cells.push(Content::empty());
            cells.push(body);
        }

        let layouter = GridLayouter::new(
            vt,
            Axes::with_x(&[Sizing::Rel((indent + body_indent).into()), Sizing::Auto]),
            Axes::with_y(&[gutter.into()]),
            &cells,
            regions,
            styles,
        );

        Ok(layouter.layout()?.fragment)
    }
}

/// A term list item.
#[node]
pub struct TermItem {
    /// The term described by the list item.
    #[positional]
    #[required]
    pub term: Content,

    /// The description of the term.
    #[positional]
    #[required]
    pub description: Content,
}

cast_from_value! {
    TermItem,
    array: Array => {
        let mut iter = array.into_iter();
        let (term, description) = match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => (a.cast()?, b.cast()?),
            _ => Err("array must contain exactly two entries")?,
        };
        Self::new(term, description)
    },
    v: Content => v.to::<Self>().cloned().ok_or("expected term item or array")?,
}
