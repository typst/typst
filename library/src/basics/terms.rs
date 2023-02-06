use crate::layout::{BlockNode, GridNode, HNode, ParNode, Spacing, TrackSizing};
use crate::prelude::*;
use crate::text::{SpaceNode, TextNode};

/// # Term List
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
/// ## Parameters
/// - items: `Content` (positional, variadic)
///   The term list's children.
///
///   When using the term list syntax, adjacent items are automatically
///   collected into term lists, even through constructs like for loops.
///
///   ```example
///   #for year, product in (
///     "1978": "TeX",
///     "1984": "LaTeX",
///     "2019": "Typst",
///   ) [/ #product: Born in #year.]
///   ```
///
/// - tight: `bool` (named)
///   If this is `{false}`, the items are spaced apart with [term list
///   spacing]($func/terms.spacing). If it is `{true}`, they use normal
///   [leading]($func/par.leading) instead. This makes the term list more
///   compact, which can look better if the items are short.
///
///   ```example
///   / Fact: If a term list has a lot
///     of text, and maybe other inline
///     content, it should not be tight
///     anymore.
///
///   / Tip: To make it wide, simply
///     insert a blank line between the
///     items.
///   ```
///
/// ## Category
/// basics
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct TermsNode {
    /// If true, the items are separated by leading instead of list spacing.
    pub tight: bool,
    /// The individual bulleted or numbered items.
    pub items: StyleVec<TermItem>,
}

#[node]
impl TermsNode {
    /// The indentation of each item's term.
    #[property(resolve)]
    pub const INDENT: Length = Length::zero();

    /// The hanging indent of the description.
    ///
    /// ```example
    /// #set terms(hanging-indent: 0pt)
    /// / Term: This term list does not
    ///   make use of hanging indents.
    /// ```
    #[property(resolve)]
    pub const HANGING_INDENT: Length = Em::new(1.0).into();

    /// The spacing between the items of a wide (non-tight) term list.
    ///
    /// If set to `{auto}` uses the spacing [below blocks]($func/block.below).
    pub const SPACING: Smart<Spacing> = Smart::Auto;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self {
            tight: args.named("tight")?.unwrap_or(true),
            items: args.all()?.into_iter().collect(),
        }
        .pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "tight" => Some(Value::Bool(self.tight)),
            "items" => {
                Some(Value::Array(self.items.items().map(|item| item.encode()).collect()))
            }
            _ => None,
        }
    }
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
        let gutter = if self.tight {
            styles.get(ParNode::LEADING).into()
        } else {
            styles
                .get(Self::SPACING)
                .unwrap_or_else(|| styles.get(BlockNode::BELOW).amount)
        };

        let mut cells = vec![];
        for (item, map) in self.items.iter() {
            let body = Content::sequence(vec![
                HNode { amount: (-body_indent).into(), weak: false }.pack(),
                (item.term.clone() + TextNode::packed(':')).strong(),
                SpaceNode.pack(),
                item.description.clone(),
            ]);

            cells.push(Content::empty());
            cells.push(body.styled_with_map(map.clone()));
        }

        GridNode {
            tracks: Axes::with_x(vec![
                TrackSizing::Relative((indent + body_indent).into()),
                TrackSizing::Auto,
            ]),
            gutter: Axes::with_y(vec![gutter.into()]),
            cells,
        }
        .layout(vt, styles, regions)
    }
}

/// A term list item.
#[derive(Debug, Clone, Hash)]
pub struct TermItem {
    /// The term described by the list item.
    pub term: Content,
    /// The description of the term.
    pub description: Content,
}

impl TermItem {
    /// Encode the item into a value.
    fn encode(&self) -> Value {
        Value::Array(array![
            Value::Content(self.term.clone()),
            Value::Content(self.description.clone()),
        ])
    }
}

castable! {
    TermItem,
    array: Array => {
        let mut iter = array.into_iter();
        let (term, description) = match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => (a.cast()?, b.cast()?),
            _ => Err("array must contain exactly two entries")?,
        };
        Self { term, description }
    },
}
