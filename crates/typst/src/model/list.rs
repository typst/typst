use comemo::Track;
use typst_utils::Numeric;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, Array, Content, Context, Depth, Func, NativeElement, Packed, Show,
    Smart, StyleChain, Styles, Value,
};
use crate::introspection::Locator;
use crate::layout::{
    BlockElem, Dir, Em, Fragment, HElem, Length, Regions, Sides, StackChild, StackElem,
    VElem,
};
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
    /// If this is `{false}`, the items are spaced apart with
    /// [list spacing]($list.spacing). If it is `{true}`, they use normal
    /// [leading]($par.leading) instead. This makes the list more compact, which
    /// can look better if the items are short.
    ///
    /// In markup mode, the value of this parameter is determined based on
    /// whether items are separated with a blank line. If items directly follow
    /// each other, this is set to `{true}`; if items are separated by a blank
    /// line, this is set to `{false}`.
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
    depth: Depth,
}

#[scope]
impl ListElem {
    #[elem]
    type ListItem;
}

impl Show for Packed<ListElem> {
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = BlockElem::multi_layouter(self.clone(), layout_list)
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

/// Layout the list.
#[typst_macros::time(span = elem.span())]
fn layout_list(
    elem: &Packed<ListElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let indent = elem.indent(styles);
    let body_indent = elem.body_indent(styles);
    let gutter = elem.spacing(styles).unwrap_or_else(|| {
        if elem.tight(styles) {
            ParElem::leading_in(styles).into()
        } else {
            ParElem::spacing_in(styles).into()
        }
    });

    let Depth(depth) = ListElem::depth_in(styles);
    let marker: Content = elem.marker(styles).resolve(engine, styles, depth)?;

    let pad = body_indent + indent; // TODO: plus marker width
    let unpad =
        (!body_indent.is_zero()).then(|| HElem::new((-body_indent).into()).pack());
    let mut children = vec![];
    for child in elem.children().iter() {
        let mut seq = vec![];
        seq.extend(unpad.clone());
        seq.push(marker.clone());
        seq.push(HElem::new(elem.body_indent(styles).into()).pack());
        seq.push(child.body.clone());
        children.push(StackChild::Block(Content::sequence(seq)));
    }

    let mut padding = Sides::default();
    if TextElem::dir_in(styles) == Dir::LTR {
        padding.left = pad.into();
    } else {
        padding.right = pad.into();
    }

    let realized = StackElem::new(children)
        .with_spacing(Some(gutter.into()))
        .pack()
        .padded(padding);
    realized.layout(engine, locator, styles, regions)
}

/// A bullet list item.
#[elem(name = "item", title = "Bullet List Item")]
pub struct ListItem {
    /// The item's body.
    #[required]
    pub body: Content,
}

impl Packed<ListItem> {
    /// Apply styles to this list item.
    pub fn styled(mut self, styles: Styles) -> Self {
        self.body.style_in_place(styles);
        self
    }
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
    fn resolve(
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
