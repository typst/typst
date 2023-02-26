use crate::layout::{BlockNode, GridNode, ParNode, Sizing, Spacing};
use crate::prelude::*;
use crate::text::TextNode;

/// # Bullet List
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
/// ## Parameters
/// - items: `Content` (positional, variadic)
///   The list's children.
///
///   When using the list syntax, adjacent items are automatically collected
///   into lists, even through constructs like for loops.
///
///   ```example
///   #for letter in "ABC" [
///     - Letter #letter
///   ]
///   ```
///
/// - tight: `bool` (named)
///   If this is `{false}`, the items are spaced apart with [list
///   spacing]($func/list.spacing). If it is `{true}`, they use normal
///   [leading]($func/par.leading) instead. This makes the list more compact,
///   which can look better if the items are short.
///
///   ```example
///   - If a list has a lot of text, and
///     maybe other inline content, it
///     should not be tight anymore.
///
///   - To make a list wide, simply insert
///     a blank line between the items.
///   ```
///
/// ## Category
/// layout
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct ListNode {
    /// If true, the items are separated by leading instead of list spacing.
    pub tight: bool,
    /// The individual bulleted or numbered items.
    pub items: StyleVec<Content>,
}

#[node]
impl ListNode {
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
    #[property(referenced)]
    pub const MARKER: Marker = Marker::Content(vec![]);

    /// The indent of each item's marker.
    #[property(resolve)]
    pub const INDENT: Length = Length::zero();

    /// The spacing between the marker and the body of each item.
    #[property(resolve)]
    pub const BODY_INDENT: Length = Em::new(0.5).into();

    /// The spacing between the items of a wide (non-tight) list.
    ///
    /// If set to `{auto}`, uses the spacing [below blocks]($func/block.below).
    pub const SPACING: Smart<Spacing> = Smart::Auto;

    /// The nesting depth.
    #[property(skip, fold)]
    const DEPTH: Depth = 0;

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
            "items" => Some(Value::Array(
                self.items.items().cloned().map(Value::Content).collect(),
            )),
            _ => None,
        }
    }
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
        let gutter = if self.tight {
            styles.get(ParNode::LEADING).into()
        } else {
            styles
                .get(Self::SPACING)
                .unwrap_or_else(|| styles.get(BlockNode::BELOW).amount)
        };

        let depth = styles.get(Self::DEPTH);
        let marker = styles.get(Self::MARKER).resolve(vt.world(), depth)?;

        let mut cells = vec![];
        for (item, map) in self.items.iter() {
            cells.push(Content::empty());
            cells.push(marker.clone());
            cells.push(Content::empty());
            cells.push(
                item.clone().styled_with_map(map.clone()).styled(Self::DEPTH, Depth),
            );
        }

        GridNode {
            tracks: Axes::with_x(vec![
                Sizing::Rel(indent.into()),
                Sizing::Auto,
                Sizing::Rel(body_indent.into()),
                Sizing::Auto,
            ]),
            gutter: Axes::with_y(vec![gutter.into()]),
            cells,
        }
        .layout(vt, styles, regions)
    }
}

/// A list's marker.
#[derive(Debug, Clone, Hash)]
pub enum Marker {
    Content(Vec<Content>),
    Func(Func),
}

impl Marker {
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

castable! {
    Marker,
    v: Content => Self::Content(vec![v]),
    array: Array => {
        if array.len() == 0 {
            Err("must contain at least one marker")?;
        }
        Self::Content(array.into_iter().map(Value::display).collect())
    },
    v: Func => Self::Func(v),
}

#[derive(Debug, Clone, Hash)]
struct Depth;

impl Fold for Depth {
    type Output = usize;

    fn fold(self, mut outer: Self::Output) -> Self::Output {
        outer += 1;
        outer
    }
}
