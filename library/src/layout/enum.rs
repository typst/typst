use std::str::FromStr;

use crate::layout::{BlockNode, ParNode, Sizing, Spacing};
use crate::meta::{Numbering, NumberingPattern};
use crate::prelude::*;
use crate::text::TextNode;

use super::GridLayouter;

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
/// You can easily switch all your enumerations to a different numbering style
/// with a set rule.
/// ```example
/// #set enum(numbering: "a)")
///
/// + Starting off ...
/// + Don't forget step two
/// ```
///
/// ## Syntax
/// This functions also has dedicated syntax:
///
/// - Starting a line with a plus sign creates an automatically numbered
///   enumeration item.
/// - Starting a line with a number followed by a dot creates an explicitly
///   numbered enumeration item.
///
/// Enumeration items can contain multiple paragraphs and other block-level
/// content. All content that is indented more than an item's plus sign or dot
/// becomes part of that item.
///
/// ## Parameters
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
/// Display: Numbered List
/// Category: layout
#[node(Construct, Layout)]
pub struct EnumNode {
    /// The numbered list's items.
    ///
    /// When using the enum syntax, adjacent items are automatically collected
    /// into enumerations, even through constructs like for loops.
    ///
    /// ```example
    /// #for phase in (
    ///    "Launch",
    ///    "Orbit",
    ///    "Descent",
    /// ) [+ #phase]
    /// ```
    #[variadic]
    pub children: Vec<EnumItem>,

    /// If this is `{false}`, the items are spaced apart with
    /// [enum spacing]($func/enum.spacing). If it is `{true}`, they use normal
    /// [leading]($func/par.leading) instead. This makes the enumeration more
    /// compact, which can look better if the items are short.
    ///
    /// ```example
    /// + If an enum has a lot of text, and
    ///   maybe other inline content, it
    ///   should not be tight anymore.
    ///
    /// + To make an enum wide, simply
    ///   insert a blank line between the
    ///   items.
    /// ```
    #[named]
    #[default(true)]
    pub tight: bool,

    /// How to number the enumeration. Accepts a
    /// [numbering pattern or function]($func/numbering).
    ///
    /// If the numbering pattern contains multiple counting symbols, they apply
    /// to nested enums. If given a function, the function receives one argument
    /// if `full` is `{false}` and multiple arguments if `full` is `{true}`.
    ///
    /// ```example
    /// #set enum(numbering: "1.a)")
    /// + Different
    /// + Numbering
    ///   + Nested
    ///   + Items
    /// + Style
    ///
    /// #set enum(numbering: n => super[#n])
    /// + Superscript
    /// + Numbering!
    /// ```
    #[settable]
    #[default(Numbering::Pattern(NumberingPattern::from_str("1.").unwrap()))]
    pub numbering: Numbering,

    /// Whether to display the full numbering, including the numbers of
    /// all parent enumerations.
    ///
    /// Defaults to `{false}`.
    ///
    /// ```example
    /// #set enum(numbering: "1.a)", full: true)
    /// + Cook
    ///   + Heat water
    ///   + Add integredients
    /// + Eat
    /// ```
    #[settable]
    #[default(false)]
    pub full: bool,

    /// The indentation of each item's label.
    #[settable]
    #[resolve]
    #[default]
    pub indent: Length,

    /// The space between the numbering and the body of each item.
    #[settable]
    #[resolve]
    #[default(Em::new(0.5).into())]
    pub body_indent: Length,

    /// The spacing between the items of a wide (non-tight) enumeration.
    ///
    /// If set to `{auto}`, uses the spacing [below blocks]($func/block.below).
    #[settable]
    #[default]
    pub spacing: Smart<Spacing>,

    /// The numbers of parent items.
    #[settable]
    #[fold]
    #[skip]
    #[default]
    parents: Parent,
}

impl Construct for EnumNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let mut items = args.all::<EnumItem>()?;
        if let Some(number) = args.named::<NonZeroUsize>("start")? {
            if let Some(first) = items.first_mut() {
                if first.number().is_none() {
                    *first = EnumItem::new(first.body()).with_number(Some(number));
                }
            }
        }

        Ok(Self::new(items)
            .with_tight(args.named("tight")?.unwrap_or(true))
            .pack())
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
        let gutter = if self.tight() {
            styles.get(ParNode::LEADING).into()
        } else {
            styles
                .get(Self::SPACING)
                .unwrap_or_else(|| styles.get(BlockNode::BELOW).amount())
        };

        let mut cells = vec![];
        let mut number = NonZeroUsize::new(1).unwrap();
        let mut parents = styles.get(Self::PARENTS);
        let full = styles.get(Self::FULL);

        for item in self.children() {
            number = item.number().unwrap_or(number);

            let resolved = if full {
                parents.push(number);
                let content = numbering.apply(vt.world(), &parents)?.display();
                parents.pop();
                content
            } else {
                match &numbering {
                    Numbering::Pattern(pattern) => {
                        TextNode::packed(pattern.apply_kth(parents.len(), number))
                    }
                    other => other.apply(vt.world(), &[number])?.display(),
                }
            };

            cells.push(Content::empty());
            cells.push(resolved);
            cells.push(Content::empty());
            cells.push(item.body().styled(Self::PARENTS, Parent(number)));
            number = number.saturating_add(1);
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

/// An enumeration item.
///
/// Display: Numbered List Item
/// Category: layout
#[node]
pub struct EnumItem {
    /// The item's number.
    #[positional]
    #[default]
    pub number: Option<NonZeroUsize>,

    /// The item's body.
    #[positional]
    #[required]
    pub body: Content,
}

cast_from_value! {
    EnumItem,
    array: Array => {
        let mut iter = array.into_iter();
        let (number, body) = match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => (a.cast()?, b.cast()?),
            _ => Err("array must contain exactly two entries")?,
        };
        Self::new(body).with_number(number)
    },
    v: Content => v.to::<Self>().cloned().unwrap_or_else(|| Self::new(v.clone())),
}

struct Parent(NonZeroUsize);

cast_from_value! {
    Parent,
    v: NonZeroUsize => Self(v),
}

cast_to_value! {
    v: Parent => v.0.into()
}

impl Fold for Parent {
    type Output = Vec<NonZeroUsize>;

    fn fold(self, mut outer: Self::Output) -> Self::Output {
        outer.push(self.0);
        outer
    }
}
