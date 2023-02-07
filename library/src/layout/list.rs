use crate::layout::{BlockNode, GridNode, ParNode, Spacing, TrackSizing};
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
///   - Basics
///   - Text
///   - Math
///   - Layout
///   - Visualize
///   - Meta
///
/// - *Compute*
///   #list(
///     [Foundations],
///     [Calculate],
///     [Create],
///     [Data Loading],
///     [Utility],
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
    /// The marker which introduces each element.
    ///
    /// ```example
    /// #set list(marker: [--])
    ///
    /// - A more classic list
    /// - With en-dashes
    /// ```
    #[property(referenced)]
    pub const MARKER: Content = TextNode::packed('•');

    /// The indent of each item's marker.
    #[property(resolve)]
    pub const INDENT: Length = Length::zero();

    /// The spacing between the marker and the body of each item.
    #[property(resolve)]
    pub const BODY_INDENT: Length = Em::new(0.5).into();

    /// The spacing between the items of a wide (non-tight) list.
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
        let marker = styles.get(Self::MARKER);
        let indent = styles.get(Self::INDENT);
        let body_indent = styles.get(Self::BODY_INDENT);
        let gutter = if self.tight {
            styles.get(ParNode::LEADING).into()
        } else {
            styles
                .get(Self::SPACING)
                .unwrap_or_else(|| styles.get(BlockNode::BELOW).amount)
        };

        let mut cells = vec![];
        for (item, map) in self.items.iter() {
            cells.push(Content::empty());
            cells.push(marker.clone());
            cells.push(Content::empty());
            cells.push(item.clone().styled_with_map(map.clone()));
        }

        GridNode {
            tracks: Axes::with_x(vec![
                TrackSizing::Relative(indent.into()),
                TrackSizing::Auto,
                TrackSizing::Relative(body_indent.into()),
                TrackSizing::Auto,
            ]),
            gutter: Axes::with_y(vec![gutter.into()]),
            cells,
        }
        .layout(vt, styles, regions)
    }
}
