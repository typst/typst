use comemo::Track;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, Array, Content, Context, Depth, Func, NativeElement, Packed, Show,
    Smart, StyleChain, Styles, Value,
};
use crate::layout::{BlockElem, Em, Length, VElem};
use crate::model::ParElem;
use crate::text::TextElem;

/// A bullet list.
///
/// Displays a sequence of items vertically, with each item introduced by a
/// marker.
///
/// # Example
/// ```example
/// Normal list.
/// - Text
/// - Math
/// - Layout
/// - ...
///
/// Multiple lines.
/// - This list item spans multiple
///   lines because it is indented.
///
/// Function call.
/// #list(
///   [Foundations],
///   [Calculate],
///   [Construct],
///   [Data Loading],
/// )
/// ```
///
/// # Syntax
/// This functions also has dedicated syntax: Start a line with a hyphen,
/// followed by a space to create a list item. A list item can contain multiple
/// paragraphs and other block-level content. All content that is indented
/// more than an item's marker becomes part of that item.
#[elem(scope, title = "Bullet List", Show)]
pub struct ListElem {
    /// Defines the default [spacing]($list.spacing) of the list. If it is
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
    /// - If a list has a lot of text, and
    ///   maybe other inline content, it
    ///   should not be tight anymore.
    ///
    /// - To make a list wide, simply insert
    ///   a blank line between the items.
    /// ```
    #[default(true)]
    pub tight: bool,

    /// The marker which introduces each item.
    ///
    /// Instead of plain content, you can also pass an array with multiple
    /// markers that should be used for nested lists. If the list nesting depth
    /// exceeds the number of markers, the markers are cycled. For total
    /// control, you may pass a function that maps the list's nesting depth
    /// (starting from `{0}`) to a desired marker.
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
    #[borrowed]
    #[default(ListMarker::Content(vec![
        // These are all available in the default font, vertically centered, and
        // roughly of the same size (with the last one having slightly lower
        // weight because it is not filled).
        TextElem::packed('\u{2022}'), // Bullet
        TextElem::packed('\u{2023}'), // Triangular Bullet
        TextElem::packed('\u{2013}'), // En-dash
    ]))]
    pub marker: ListMarker,

    /// The indent of each item.
    #[resolve]
    pub indent: Length,

    /// The spacing between the marker and the body of each item.
    #[resolve]
    #[default(Em::new(0.5).into())]
    pub body_indent: Length,

    /// The spacing between the items of the list.
    ///
    /// If set to `{auto}`, uses paragraph [`leading`]($par.leading) for tight
    /// lists and paragraph [`spacing`]($par.spacing) for wide (non-tight)
    /// lists.
    pub spacing: Smart<Length>,

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
    pub children: Vec<Packed<ListItem>>,

    /// The nesting depth.
    #[internal]
    #[fold]
    #[ghost]
    pub depth: Depth,
}

#[scope]
impl ListElem {
    #[elem]
    type ListItem;
}

impl Show for Packed<ListElem> {
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let mut realized =
            BlockElem::multi_layouter(self.clone(), engine.routines.layout_list)
                .pack()
                .spanned(self.span());

        if self.tight(styles) {
            let leading = ParElem::leading_in(styles);
            let spacing =
                VElem::new(leading.into()).with_weak(true).with_attach(true).pack();
            realized = spacing + realized;
        }

        Ok(realized)
    }
}

/// A bullet list item.
#[elem(name = "item", title = "Bullet List Item")]
pub struct ListItem {
    /// The item's body.
    #[required]
    pub body: Content,
}

cast! {
    ListItem,
    v: Content => v.unpack::<Self>().unwrap_or_else(Self::new)
}

/// A list's marker.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ListMarker {
    Content(Vec<Content>),
    Func(Func),
}

impl ListMarker {
    /// Resolve the marker for the given depth.
    pub fn resolve(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        depth: usize,
    ) -> SourceResult<Content> {
        Ok(match self {
            Self::Content(list) => {
                list.get(depth % list.len()).cloned().unwrap_or_default()
            }
            Self::Func(func) => func
                .call(engine, Context::new(None, Some(styles)).track(), [depth])?
                .display(),
        })
    }
}

cast! {
    ListMarker,
    self => match self {
        Self::Content(vec) => if vec.len() == 1 {
            vec.into_iter().next().unwrap().into_value()
        } else {
            vec.into_value()
        },
        Self::Func(func) => func.into_value(),
    },
    v: Content => Self::Content(vec![v]),
    array: Array => {
        if array.is_empty() {
            bail!("array must contain at least one marker");
        }
        Self::Content(array.into_iter().map(Value::display).collect())
    },
    v: Func => Self::Func(v),
}

/// A list, enum, or term list.
pub trait ListLike: NativeElement {
    /// The kind of list item this list is composed of.
    type Item: ListItemLike;

    /// Create this kind of list from its children and tightness.
    fn create(children: Vec<Packed<Self::Item>>, tight: bool) -> Self;
}

/// A list item, enum item, or term list item.
pub trait ListItemLike: NativeElement {
    /// Apply styles to the element's body.
    fn styled(item: Packed<Self>, styles: Styles) -> Packed<Self>;
}

impl ListLike for ListElem {
    type Item = ListItem;

    fn create(children: Vec<Packed<Self::Item>>, tight: bool) -> Self {
        Self::new(children).with_tight(tight)
    }
}

impl ListItemLike for ListItem {
    fn styled(mut item: Packed<Self>, styles: Styles) -> Packed<Self> {
        item.body.style_in_place(styles);
        item
    }
}
