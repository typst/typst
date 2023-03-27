use std::str::FromStr;

use crate::layout::{BlockElem, ParElem, Sizing, Spacing};
use crate::meta::{Numbering, NumberingPattern};
use crate::prelude::*;
use crate::text::TextElem;

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
/// Display: Numbered List
/// Category: layout
#[element(Layout)]
pub struct EnumElem {
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
    #[default(Numbering::Pattern(NumberingPattern::from_str("1.").unwrap()))]
    pub numbering: Numbering,

    /// Which number to start the enumeration with.
    ///
    /// ```example
    /// #enum(
    ///   start: 3,
    ///   [Skipping],
    ///   [Ahead],
    /// )
    /// ```
    #[default(1)]
    pub start: usize,

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
    #[default(false)]
    pub full: bool,

    /// The indentation of each item's label.
    #[resolve]
    pub indent: Length,

    /// The space between the numbering and the body of each item.
    #[resolve]
    #[default(Em::new(0.5).into())]
    pub body_indent: Length,

    /// The spacing between the items of a wide (non-tight) enumeration.
    ///
    /// If set to `{auto}`, uses the spacing [below blocks]($func/block.below).
    pub spacing: Smart<Spacing>,

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

    /// The numbers of parent items.
    #[internal]
    #[fold]
    parents: Parent,
}

impl Layout for EnumElem {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let numbering = self.numbering(styles);
        let indent = self.indent(styles);
        let body_indent = self.body_indent(styles);
        let gutter = if self.tight(styles) {
            ParElem::leading_in(styles).into()
        } else {
            self.spacing(styles)
                .unwrap_or_else(|| BlockElem::below_in(styles).amount())
        };

        let mut cells = vec![];
        let mut number = self.start(styles);
        let mut parents = self.parents(styles);
        let full = self.full(styles);

        for item in self.children() {
            number = item.number(styles).unwrap_or(number);

            let resolved = if full {
                parents.push(number);
                let content = numbering.apply_vt(vt, &parents)?.display();
                parents.pop();
                content
            } else {
                match &numbering {
                    Numbering::Pattern(pattern) => {
                        TextElem::packed(pattern.apply_kth(parents.len(), number))
                    }
                    other => other.apply_vt(vt, &[number])?.display(),
                }
            };

            cells.push(Content::empty());
            cells.push(resolved);
            cells.push(Content::empty());
            cells.push(item.body().styled(Self::set_parents(Parent(number))));
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
#[element]
pub struct EnumItem {
    /// The item's number.
    #[positional]
    pub number: Option<usize>,

    /// The item's body.
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

struct Parent(usize);

cast_from_value! {
    Parent,
    v: usize => Self(v),
}

cast_to_value! {
    v: Parent => v.0.into()
}

impl Fold for Parent {
    type Output = Vec<usize>;

    fn fold(self, mut outer: Self::Output) -> Self::Output {
        outer.push(self.0);
        outer
    }
}
