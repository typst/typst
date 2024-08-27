use std::str::FromStr;

use comemo::Track;
use smallvec::{smallvec, SmallVec};

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, Array, Content, Context, NativeElement, Packed, Show, ShowSet,
    Smart, StyleChain, Styles,
};
use crate::introspection::{Locatable, Locator, LocatorLink};
use crate::layout::{
    layout_fragment, layout_frame, Abs, Axes, BlockElem, Em, Fragment, HElem, Length,
    Region, Regions, Sides, Size, VElem,
};
use crate::model::{Numbering, NumberingPattern, ParElem};
use crate::text::{isolate, TextElem};

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
#[elem(scope, title = "Numbered List", Locatable, Show, ShowSet)]
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
    #[borrowed]
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

    /// The indentation of the enumeration.
    #[resolve]
    #[default(Em::new(1.75).into())]
    pub indent: Length,

    /// The spacing between the number and the body of each item.
    #[resolve]
    #[default(Em::new(0.5).into())]
    pub number_gap: Length,

    /// The spacing between the items of the enumeration.
    ///
    /// If set to `{auto}`, uses paragraph [`leading`]($par.leading) for tight
    /// enumerations and paragraph [`spacing`]($par.spacing) for wide
    /// (non-tight) enumerations.
    pub spacing: Smart<Length>,

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
    pub children: Vec<Packed<EnumItem>>,

    /// The numbers of parent items.
    #[internal]
    #[fold]
    #[ghost]
    parents: SmallVec<[usize; 4]>,
}

#[scope]
impl EnumElem {
    #[elem]
    type EnumItem;
}

impl Show for Packed<EnumElem> {
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let indent = self.indent(styles);
        let inset = Sides::one(Some(indent.into()), TextElem::dir_in(styles).start());

        let mut realized = BlockElem::multi_layouter(self.clone(), layout_enum)
            .with_inset(inset)
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

impl ShowSet for Packed<EnumElem> {
    fn show_set(&self, _: StyleChain) -> Styles {
        let mut out = Styles::new();
        out.set(ParElem::set_first_line_indent(Length::zero()));
        out
    }
}

/// Layout the enumeration.
#[typst_macros::time(span = elem.span())]
fn layout_enum(
    elem: &Packed<EnumElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let numbering = elem.numbering(styles);
    let full = elem.full(styles);
    let indent = elem.indent(styles);
    let number_gap = elem.number_gap(styles);
    let spacing = elem.spacing(styles).unwrap_or_else(|| {
        if elem.tight(styles) {
            ParElem::leading_in(styles).into()
        } else {
            ParElem::spacing_in(styles).into()
        }
    });

    let spacing_elem = VElem::block_spacing(spacing.into()).pack();
    let number_gap_elem = HElem::new(number_gap.into()).pack();

    let mut number = elem.start(styles);
    let mut parents = EnumElem::parents_in(styles);
    let mut seq = vec![];

    for (i, child) in elem.children.iter().enumerate() {
        if i > 0 {
            seq.push(spacing_elem.clone());
        }

        number = child.number(styles).unwrap_or(number);

        let context = Context::new(None, Some(styles));
        let resolved = if full {
            parents.push(number);
            let content = numbering.apply(engine, context.track(), &parents)?.display();
            parents.pop();
            content
        } else {
            match numbering {
                Numbering::Pattern(pattern) => {
                    TextElem::packed(pattern.apply_kth(parents.len(), number))
                }
                other => other.apply(engine, context.track(), &[number])?.display(),
            }
        };

        let number_width = layout_frame(
            engine,
            &resolved,
            Locator::link(&LocatorLink::measure(elem.location().unwrap())),
            styles,
            Region::new(Size::splat(Abs::inf()), Axes::splat(false)),
        )?
        .width();

        let dedent = -(number_width + number_gap).min(indent);
        let dedent_elem = HElem::new(dedent.into()).pack();

        seq.push(dedent_elem);
        isolate(&mut seq, resolved, styles);
        seq.push(number_gap_elem.clone());
        seq.push(child.body.clone().styled(EnumElem::set_parents(smallvec![number])));

        number = number.saturating_add(1);
    }

    let realized = Content::sequence(seq);
    layout_fragment(engine, &realized, locator, styles, regions)
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

impl Packed<EnumItem> {
    /// Apply styles to this enum item.
    pub fn styled(mut self, styles: Styles) -> Self {
        self.body.style_in_place(styles);
        self
    }
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
    v: Content => v.unpack::<Self>().unwrap_or_else(Self::new),
}
