use std::str::FromStr;

use crate::compute::{Numbering, NumberingPattern};
use crate::layout::{BlockNode, GridNode, ParNode, Spacing, TrackSizing};
use crate::prelude::*;

/// # Numbered List
/// A numbered list.
///
/// Displays a sequence of items vertically and numbers them consecutively.
///
/// ## Example
/// ```example
/// Automatically numbered:
/// + Preparations
/// + Analysis
/// + Conclusions
///
/// Manually numbered:
/// 2. What is the first step?
/// 5. I am confused.
/// +  Moving on ...
///
/// Function call.
/// #enum[First][Second]
/// ```
///
/// ## Syntax
/// This functions also has dedicated syntax:
///
/// - Starting a line with a plus sign creates an automatically numbered
///   enumeration item.
/// - Start a line with a number followed by a dot creates an explicitly
///   numbered enumeration item.
///
/// Enumeration items can contain multiple paragraphs and other block-level
/// content. All content that is indented more than an item's plus sign or dot
/// becomes part of that item.
///
/// ## Parameters
/// - items: `Content` (positional, variadic)
///   The enumeration's children.
///
///   When using the enum syntax, adjacent items are automatically collected
///   into enumerations, even through constructs like for loops.
///
///   ```example
///   #for phase in (
///      "Launch",
///      "Orbit",
///      "Descent",
///   ) [+ #phase]
///   ```
///
/// - start: `NonZeroUsize` (named)
///   Which number to start the enumeration with.
///
///   ```example
///   #enum(
///     start: 3,
///     [Skipping],
///     [Ahead],
///   )
///   ```
///
/// - tight: `bool` (named)
///   If this is `{false}`, the items are spaced apart with
///   [enum spacing]($func/enum.spacing). If it is `{true}`, they use normal
///   [leading]($func/par.leading) instead. This makes the enumeration more
///   compact, which can look better if the items are short.
///
///   ```example
///   + If an enum has a lot of text, and
///     maybe other inline content, it
///     should not be tight anymore.
///
///   + To make an enum wide, simply
///     insert a blank line between the
///     items.
///   ```
///
/// ## Category
/// basics
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct EnumNode {
    /// If true, the items are separated by leading instead of list spacing.
    pub tight: bool,
    /// The individual numbered items.
    pub items: StyleVec<(Option<NonZeroUsize>, Content)>,
}

#[node]
impl EnumNode {
    /// How to number the enumeration. Accepts a
    /// [numbering pattern or function]($func/numbering).
    ///
    /// ```example
    /// #set enum(numbering: "(a)")
    ///
    /// + Different
    /// + Numbering
    /// + Style
    /// ```
    #[property(referenced)]
    pub const NUMBERING: Numbering =
        Numbering::Pattern(NumberingPattern::from_str("1.").unwrap());

    /// The indentation of each item's label.
    #[property(resolve)]
    pub const INDENT: Length = Length::zero();

    /// The space between the numbering and the body of each item.
    #[property(resolve)]
    pub const BODY_INDENT: Length = Em::new(0.5).into();

    /// The spacing between the items of a wide (non-tight) enumeration.
    ///
    /// If set to `{auto}` uses the spacing [below blocks]($func/block.below).
    pub const SPACING: Smart<Spacing> = Smart::Auto;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let mut number: NonZeroUsize =
            args.named("start")?.unwrap_or(NonZeroUsize::new(1).unwrap());

        Ok(Self {
            tight: args.named("tight")?.unwrap_or(true),
            items: args
                .all()?
                .into_iter()
                .map(|body| {
                    let item = (Some(number), body);
                    number = number.saturating_add(1);
                    item
                })
                .collect(),
        }
        .pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "tight" => Some(Value::Bool(self.tight)),
            "items" => Some(Value::Array(
                self.items
                    .items()
                    .map(|(number, body)| {
                        Value::Dict(dict! {
                            "number" => match *number {
                                Some(n) => Value::Int(n.get() as i64),
                                None => Value::None,
                            },
                            "body" => Value::Content(body.clone()),
                        })
                    })
                    .collect(),
            )),
            _ => None,
        }
    }
}

impl Layout for EnumNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let numbering = styles.get(Self::NUMBERING);
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
        let mut number = NonZeroUsize::new(1).unwrap();
        for ((n, item), map) in self.items.iter() {
            number = n.unwrap_or(number);
            let resolved = numbering.apply(vt.world(), &[number])?.display();
            cells.push(Content::empty());
            cells.push(resolved.styled_with_map(map.clone()));
            cells.push(Content::empty());
            cells.push(item.clone().styled_with_map(map.clone()));
            number = number.saturating_add(1);
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
