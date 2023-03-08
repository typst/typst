use crate::layout::{BlockNode, ParNode, Sizing, Spacing};
use crate::prelude::*;
use crate::text::TextNode;

use super::GridLayouter;

/// A bullet list.
///
/// Displays a sequence of items vertically, with each item introduced by a
/// marker.
///
/// ## Example
/// ```example
/// - *Content*
///   - Text
///   - Math
///   - Layout
///   - Visualize
///   - Meta
///   - Symbols
///
/// - *Compute*
///   #list(
///     [Foundations],
///     [Calculate],
///     [Construct],
///     [Data Loading],
///   )
/// ```
///
/// ## Syntax
/// This functions also has dedicated syntax: Start a line with a hyphen,
/// followed by a space to create a list item. A list item can contain multiple
/// paragraphs and other block-level content. All content that is indented
/// more than an item's hyphen becomes part of that item.
///
/// Display: Bullet List
/// Category: layout
#[node(Layout)]
pub struct ListNode {
    /// The bullet list's children.
    ///
    /// When using the list syntax, adjacent items are automatically collected
    /// into lists, even through constructs like for loops.
    ///
    /// ```example
    /// #for letter in "ABC" [
    ///   - Letter #letter
    /// ]
    /// ```
    #[variadic]
    pub items: Vec<ListItem>,

    /// If this is `{false}`, the items are spaced apart with [list
    /// spacing]($func/list.spacing). If it is `{true}`, they use normal
    /// [leading]($func/par.leading) instead. This makes the list more compact,
    /// which can look better if the items are short.
    ///
    /// ```example
    /// - If a list has a lot of text, and
    ///   maybe other inline content, it
    ///   should not be tight anymore.
    ///
    /// - To make a list wide, simply insert
    ///   a blank line between the items.
    /// ```
    #[named]
    #[default(true)]
    pub tight: bool,

    /// The marker which introduces each item.
    ///
    /// Instead of plain content, you can also pass an array with multiple
    /// markers that should be used for nested lists. If the list nesting depth
    /// exceeds the number of markers, the last one is repeated. For total
    /// control, you may pass a function that maps the list's nesting depth
    /// (starting from `{0}`) to a desired marker.
    ///
    /// Default: `•`
    ///
    /// ```example
    /// #set list(marker: [--])
    /// - A more classic list
    /// - With en-dashes
    ///
    /// #set list(marker: ([•], [--]))
    /// - Top-level
    ///   - Nested
    ///   - Items
    /// - Items
    /// ```
    #[settable]
    #[default(ListMarker::Content(vec![]))]
    pub marker: ListMarker,

    /// The indent of each item's marker.
    #[settable]
    #[resolve]
    #[default]
    pub indent: Length,

    /// The spacing between the marker and the body of each item.
    #[settable]
    #[resolve]
    #[default(Em::new(0.5).into())]
    pub body_indent: Length,

    /// The spacing between the items of a wide (non-tight) list.
    ///
    /// If set to `{auto}`, uses the spacing [below blocks]($func/block.below).
    #[settable]
    #[default]
    pub spacing: Smart<Spacing>,

    /// The nesting depth.
    #[settable]
    #[fold]
    #[skip]
    #[default]
    depth: Depth,
}

impl Layout for ListNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let indent = styles.get(Self::INDENT);
        let body_indent = styles.get(Self::BODY_INDENT);
        let gutter = if self.tight() {
            styles.get(ParNode::LEADING).into()
        } else {
            styles
                .get(Self::SPACING)
                .unwrap_or_else(|| styles.get(BlockNode::BELOW).amount())
        };

        let depth = styles.get(Self::DEPTH);
        let marker = styles.get(Self::MARKER).resolve(vt.world(), depth)?;

        let mut cells = vec![];
        for item in self.items() {
            cells.push(Content::empty());
            cells.push(marker.clone());
            cells.push(Content::empty());
            cells.push(item.body().styled(Self::DEPTH, Depth));
        }

        let layouter = GridLayouter::new(
            vt,
            Axes::with_x(&[
                Sizing::Rel(indent.into()),
                Sizing::Auto,
                Sizing::Rel(body_indent.into()),
                Sizing::Auto,
            ]),
            Axes::with_y(&[gutter.into()]),
            &cells,
            regions,
            styles,
        );

        Ok(layouter.layout()?.fragment)
    }
}

/// A bullet list item.
///
/// Display: Bullet List Item
/// Category: layout
#[node]
pub struct ListItem {
    /// The item's body.
    #[positional]
    #[required]
    pub body: Content,
}

cast_from_value! {
    ListItem,
    v: Content => v.to::<Self>().cloned().unwrap_or_else(|| Self::new(v.clone())),
}

/// A list's marker.
#[derive(Debug, Clone, Hash)]
pub enum ListMarker {
    Content(Vec<Content>),
    Func(Func),
}

impl ListMarker {
    /// Resolve the marker for the given depth.
    fn resolve(&self, world: Tracked<dyn World>, depth: usize) -> SourceResult<Content> {
        Ok(match self {
            Self::Content(list) => list
                .get(depth)
                .or(list.last())
                .cloned()
                .unwrap_or_else(|| TextNode::packed('•')),
            Self::Func(func) => {
                let args = Args::new(func.span(), [Value::Int(depth as i64)]);
                func.call_detached(world, args)?.display()
            }
        })
    }
}

cast_from_value! {
    ListMarker,
    v: Content => Self::Content(vec![v]),
    array: Array => {
        if array.len() == 0 {
            Err("must contain at least one marker")?;
        }
        Self::Content(array.into_iter().map(Value::display).collect())
    },
    v: Func => Self::Func(v),
}

cast_to_value! {
    v: ListMarker => match v {
        ListMarker::Content(vec) => vec.into(),
        ListMarker::Func(func) => func.into(),
    }
}

struct Depth;

cast_from_value! {
    Depth,
    _: Value => Self,
}

cast_to_value! {
    _: Depth => Value::None
}

impl Fold for Depth {
    type Output = usize;

    fn fold(self, outer: Self::Output) -> Self::Output {
        outer + 1
    }
}
