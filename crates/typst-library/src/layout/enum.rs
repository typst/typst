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
/// # Example
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
/// Multiple lines:
/// + This enum item has multiple
///   lines because the next line
///   is indented.
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
/// You can also use [`enum.item`]($enum.item) to programmatically customize the
/// number of each item in the enumeration:
///
/// ```example
/// #enum(
///   enum.item(1)[First step],
///   enum.item(5)[Fifth step],
///   enum.item(10)[Tenth step]
/// )
/// ```
///
/// # Syntax
/// This functions also has dedicated syntax:
///
/// - Starting a line with a plus sign creates an automatically numbered
///   enumeration item.
/// - Starting a line with a number followed by a dot creates an explicitly
///   numbered enumeration item.
///
/// Enumeration items can contain multiple paragraphs and other block-level
/// content. All content that is indented more than an item's marker becomes
/// part of that item.
#[elem(scope, title = "Numbered List", Layout)]
pub struct EnumElem {
    /// If this is `{false}`, the items are spaced apart with
    /// [enum spacing]($enum.spacing). If it is `{true}`, they use normal
    /// [leading]($par.leading) instead. This makes the enumeration more
    /// compact, which can look better if the items are short.
    ///
    /// In markup mode, the value of this parameter is determined based on
    /// whether items are separated with a blank line. If items directly follow
    /// each other, this is set to `{true}`; if items are separated by a blank
    /// line, this is set to `{false}`.
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
    /// [numbering pattern or function]($numbering).
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

    /// The indentation of each item.
    #[resolve]
    pub indent: Length,

    /// The space between the numbering and the body of each item.
    #[resolve]
    #[default(Em::new(0.5).into())]
    pub body_indent: Length,

    /// The spacing between the items of a wide (non-tight) enumeration.
    ///
    /// If set to `{auto}`, uses the spacing [below blocks]($block.below).
    pub spacing: Smart<Spacing>,

    /// The horizontal alignment that enum numbers should have.
    ///
    /// By default, this is set to `{end}`, which aligns enum numbers
    /// towards end of the current text direction (in left-to-right script,
    /// for example, this is the same as `{right}`). The choice of `{end}`
    /// for horizontal alignment of enum numbers is usually preferred over
    /// `{start}`, as numbers then grow away from the text instead of towards
    /// it, avoiding certain visual issues. This option lets you override this
    /// behavior, however.
    ///
    /// ````example
    /// #set enum(number-align: start)
    ///
    /// Here are some powers of two:
    /// 1. One
    /// 2. Two
    /// 4. Four
    /// 8. Eight
    /// 16. Sixteen
    /// 32. Thirty two
    /// ````
    #[default(HAlign::End)]
    pub number_align: HAlign,

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

#[scope]
impl EnumElem {
    #[elem]
    type EnumItem;
}

impl Layout for EnumElem {
    #[tracing::instrument(name = "EnumElem::layout", skip_all)]
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

        // Horizontally align based on the given respective parameter.
        // Vertically align to the top to avoid inheriting `horizon` or `bottom`
        // alignment from the context and having the number be displaced in
        // relation to the item it refers to.
        let number_align = self.number_align(styles) + VAlign::Top;

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

            // Disable overhang as a workaround to end-aligned dots glitching
            // and decreasing spacing between numbers and items.
            let resolved =
                resolved.aligned(number_align).styled(TextElem::set_overhang(false));

            cells.push(Content::empty());
            cells.push(resolved);
            cells.push(Content::empty());
            cells.push(item.body().styled(Self::set_parents(Parent(number))));
            number = number.saturating_add(1);
        }

        let layouter = GridLayouter::new(
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
            self.span(),
        );

        Ok(layouter.layout(vt)?.fragment)
    }
}

/// An enumeration item.
#[elem(name = "item", title = "Numbered List Item")]
pub struct EnumItem {
    /// The item's number.
    #[positional]
    pub number: Option<usize>,

    /// The item's body.
    #[required]
    pub body: Content,
}

cast! {
    EnumItem,
    array: Array => {
        let mut iter = array.into_iter();
        let (number, body) = match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => (a.cast()?, b.cast()?),
            _ => bail!("array must contain exactly two entries"),
        };
        Self::new(body).with_number(number)
    },
    v: Content => v.to::<Self>().cloned().unwrap_or_else(|| Self::new(v.clone())),
}

struct Parent(usize);

cast! {
    Parent,
    self => self.0.into_value(),
    v: usize => Self(v),
}

impl Fold for Parent {
    type Output = Vec<usize>;

    fn fold(self, mut outer: Self::Output) -> Self::Output {
        outer.push(self.0);
        outer
    }
}
