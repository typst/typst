use std::num::NonZeroUsize;
use std::ops::RangeInclusive;
use std::str::FromStr;

use typst_utils::{NonZeroExt, Scalar, singleton};

use crate::diag::{SourceResult, bail};
use crate::engine::Engine;
use crate::foundations::{
    Args, AutoValue, Cast, Construct, Content, Dict, Fold, NativeElement, Set, Smart,
    Value, cast, elem,
};
use crate::introspection::Introspector;
use crate::layout::{
    Abs, Alignment, FlushElem, Frame, HAlignment, Length, OuterVAlignment, Ratio, Rel,
    Sides, SpecificAlignment,
};
use crate::model::{DocumentInfo, Numbering};
use crate::text::LocalName;
use crate::visualize::{Color, Paint};

/// Layouts its child onto one or multiple pages.
///
/// Although this function is primarily used in set rules to affect page
/// properties, it can also be used to explicitly render its argument onto
/// a set of pages of its own.
///
/// Pages can be set to use `{auto}` as their width or height. In this case, the
/// pages will grow to fit their content on the respective axis.
///
/// The [Guide for Page Setup]($guides/page-setup-guide) explains how to use
/// this and related functions to set up a document with many examples.
///
/// # Example
/// ```example
/// >>> #set page(margin: auto)
/// #set page("us-letter")
///
/// There you go, US friends!
/// ```
#[elem(Construct)]
pub struct PageElem {
    /// A standard paper size to set width and height.
    ///
    /// This is just a shorthand for setting `width` and `height` and, as such,
    /// cannot be retrieved in a context expression.
    #[external]
    #[default(Paper::A4)]
    pub paper: Paper,

    /// The width of the page.
    ///
    /// ```example
    /// #set page(
    ///   width: 3cm,
    ///   margin: (x: 0cm),
    /// )
    ///
    /// #for i in range(3) {
    ///   box(square(width: 1cm))
    /// }
    /// ```
    #[parse(
        let paper = args.named_or_find::<Paper>("paper")?;
        args.named("width")?
            .or_else(|| paper.map(|paper| Smart::Custom(paper.width().into())))
    )]
    #[default(Smart::Custom(Paper::A4.width().into()))]
    #[ghost]
    pub width: Smart<Length>,

    /// The height of the page.
    ///
    /// If this is set to `{auto}`, page breaks can only be triggered manually
    /// by inserting a [page break]($pagebreak) or by adding another non-empty
    /// page set rule. Most examples throughout this documentation use `{auto}`
    /// for the height of the page to dynamically grow and shrink to fit their
    /// content.
    #[parse(
        args.named("height")?
            .or_else(|| paper.map(|paper| Smart::Custom(paper.height().into())))
    )]
    #[default(Smart::Custom(Paper::A4.height().into()))]
    #[ghost]
    pub height: Smart<Length>,

    /// Whether the page is flipped into landscape orientation.
    ///
    /// ```example
    /// #set page(
    ///   "us-business-card",
    ///   flipped: true,
    ///   fill: rgb("f2e5dd"),
    /// )
    ///
    /// #set align(bottom + end)
    /// #text(14pt)[*Sam H. Richards*] \
    /// _Procurement Manager_
    ///
    /// #set text(10pt)
    /// 17 Main Street \
    /// New York, NY 10001 \
    /// +1 555 555 5555
    /// ```
    #[default(false)]
    #[ghost]
    pub flipped: bool,

    /// The page's margins.
    ///
    /// - `{auto}`: The margins are set automatically to 2.5/21 times the smaller
    ///   dimension of the page. This results in 2.5 cm margins for an A4 page.
    /// - A single length: The same margin on all sides.
    /// - A dictionary: With a dictionary, the margins can be set individually.
    ///   The dictionary can contain the following keys in order of precedence:
    ///   - `top`: The top margin.
    ///   - `right`: The right margin.
    ///   - `bottom`: The bottom margin.
    ///   - `left`: The left margin.
    ///   - `inside`: The margin at the inner side of the page (where the
    ///     [binding]($page.binding) is).
    ///   - `outside`: The margin at the outer side of the page (opposite to the
    ///     [binding]($page.binding)).
    ///   - `x`: The horizontal margins.
    ///   - `y`: The vertical margins.
    ///   - `rest`: The margins on all sides except those for which the
    ///     dictionary explicitly sets a size.
    ///
    /// All keys are optional; omitted keys will use their previously set value,
    /// or the default margin if never set. In addition, the values for `left`
    /// and `right` are mutually exclusive with the values for `inside` and
    /// `outside`.
    ///
    /// ```example
    /// #set page(
    ///  width: 3cm,
    ///  height: 4cm,
    ///  margin: (x: 8pt, y: 4pt),
    /// )
    ///
    /// #rect(
    ///   width: 100%,
    ///   height: 100%,
    ///   fill: aqua,
    /// )
    /// ```
    #[fold]
    #[ghost]
    pub margin: Margin,

    /// On which side the pages will be bound.
    ///
    /// - `{auto}`: Equivalent to `left` if the [text direction]($text.dir)
    ///   is left-to-right and `right` if it is right-to-left.
    /// - `left`: Bound on the left side.
    /// - `right`: Bound on the right side.
    ///
    /// This affects the meaning of the `inside` and `outside` options for
    /// margins.
    #[ghost]
    pub binding: Smart<Binding>,

    /// How many columns the page has.
    ///
    /// If you need to insert columns into a page or other container, you can
    /// also use the [`columns` function]($columns).
    ///
    /// ```example:single
    /// #set page(columns: 2, height: 4.8cm)
    /// Climate change is one of the most
    /// pressing issues of our time, with
    /// the potential to devastate
    /// communities, ecosystems, and
    /// economies around the world. It's
    /// clear that we need to take urgent
    /// action to reduce our carbon
    /// emissions and mitigate the impacts
    /// of a rapidly changing climate.
    /// ```
    #[default(NonZeroUsize::ONE)]
    #[ghost]
    pub columns: NonZeroUsize,

    /// The page's background fill.
    ///
    /// Setting this to something non-transparent instructs the printer to color
    /// the complete page. If you are considering larger production runs, it may
    /// be more environmentally friendly and cost-effective to source pre-dyed
    /// pages and not set this property.
    ///
    /// When set to `{none}`, the background becomes transparent. Note that PDF
    /// pages will still appear with a (usually white) background in viewers,
    /// but they are actually transparent. (If you print them, no color is used
    /// for the background.)
    ///
    /// The default of `{auto}` results in `{none}` for PDF output, and
    /// `{white}` for PNG and SVG.
    ///
    /// ```example
    /// #set page(fill: rgb("444352"))
    /// #set text(fill: rgb("fdfdfd"))
    /// *Dark mode enabled.*
    /// ```
    #[ghost]
    pub fill: Smart<Option<Paint>>,

    /// How to number the pages. You can refer to the Page Setup Guide for
    /// [customizing page numbers]($guides/page-setup-guide/#page-numbers).
    ///
    /// Accepts a [numbering pattern or function]($numbering) taking one or two
    /// numbers:
    /// 1. The first number is the current page number.
    /// 2. The second number is the total number of pages. In a numbering
    ///    pattern, the second number can be omitted. If a function is passed,
    ///    it will always receive both numbers.
    ///
    /// These are logical numbers controlled by the page counter, and may thus
    /// not match the physical numbers. Specifically, they are the
    /// [current]($counter.get) and the [final]($counter.final) value of
    /// `{counter(page)}`. See the [`counter`]($counter/#page-counter)
    /// documentation for more details.
    ///
    /// If an explicit [`footer`]($page.footer) (or [`header`]($page.header) for
    /// [top-aligned]($page.number-align) numbering) is given, the numbering is
    /// ignored.
    ///
    /// ```example
    /// #set page(
    ///   height: 100pt,
    ///   margin: (top: 16pt, bottom: 24pt),
    ///   numbering: "1 / 1",
    /// )
    ///
    /// #lorem(48)
    /// ```
    #[ghost]
    pub numbering: Option<Numbering>,

    /// A supplement for the pages.
    ///
    /// For page references, this is added before the page number.
    ///
    /// ```example
    /// #set page(numbering: "1.", supplement: [p.])
    ///
    /// = Introduction <intro>
    /// We are on #ref(<intro>, form: "page")!
    /// ```
    #[ghost]
    pub supplement: Smart<Option<Content>>,

    /// The alignment of the page numbering.
    ///
    /// If the vertical component is `top`, the numbering is placed into the
    /// header and if it is `bottom`, it is placed in the footer. Horizon
    /// alignment is forbidden. If an explicit matching `header` or `footer` is
    /// given, the numbering is ignored.
    ///
    /// ```example
    /// #set page(
    ///   margin: (top: 16pt, bottom: 24pt),
    ///   numbering: "1",
    ///   number-align: right,
    /// )
    ///
    /// #lorem(30)
    /// ```
    #[default(SpecificAlignment::Both(HAlignment::Center, OuterVAlignment::Bottom))]
    #[ghost]
    pub number_align: SpecificAlignment<HAlignment, OuterVAlignment>,

    /// The page's header. Fills the top margin of each page.
    ///
    /// - Content: Shows the content as the header.
    /// - `{auto}`: Shows the page number if a [`numbering`]($page.numbering) is
    ///   set and [`number-align`]($page.number-align) is `top`.
    /// - `{none}`: Suppresses the header.
    ///
    /// ```example
    /// #set par(justify: true)
    /// #set page(
    ///   margin: (top: 32pt, bottom: 20pt),
    ///   header: [
    ///     #set text(8pt)
    ///     #smallcaps[Typst Academy]
    ///     #h(1fr) _Exercise Sheet 3_
    ///   ],
    /// )
    ///
    /// #lorem(19)
    /// ```
    #[ghost]
    pub header: Smart<Option<Content>>,

    /// The amount the header is raised into the top margin.
    #[default(Ratio::new(0.3).into())]
    #[ghost]
    pub header_ascent: Rel<Length>,

    /// The page's footer. Fills the bottom margin of each page.
    ///
    /// - Content: Shows the content as the footer.
    /// - `{auto}`: Shows the page number if a [`numbering`]($page.numbering) is
    ///   set and [`number-align`]($page.number-align) is `bottom`.
    /// - `{none}`: Suppresses the footer.
    ///
    /// For just a page number, the `numbering` property typically suffices. If
    /// you want to create a custom footer but still display the page number,
    /// you can directly access the [page counter]($counter).
    ///
    /// ```example
    /// #set par(justify: true)
    /// #set page(
    ///   height: 100pt,
    ///   margin: 20pt,
    ///   footer: context [
    ///     #set align(right)
    ///     #set text(8pt)
    ///     #counter(page).display(
    ///       "1 of I",
    ///       both: true,
    ///     )
    ///   ]
    /// )
    ///
    /// #lorem(48)
    /// ```
    #[ghost]
    pub footer: Smart<Option<Content>>,

    /// The amount the footer is lowered into the bottom margin.
    #[default(Ratio::new(0.3).into())]
    #[ghost]
    pub footer_descent: Rel<Length>,

    /// Content in the page's background.
    ///
    /// This content will be placed behind the page's body. It can be
    /// used to place a background image or a watermark.
    ///
    /// ```example
    /// #set page(background: rotate(24deg,
    ///   text(18pt, fill: rgb("FFCBC4"))[
    ///     *CONFIDENTIAL*
    ///   ]
    /// ))
    ///
    /// = Typst's secret plans
    /// In the year 2023, we plan to take
    /// over the world (of typesetting).
    /// ```
    #[ghost]
    pub background: Option<Content>,

    /// Content in the page's foreground.
    ///
    /// This content will overlay the page's body.
    ///
    /// ```example
    /// #set page(foreground: text(24pt)[🤓])
    ///
    /// Reviewer 2 has marked our paper
    /// "Weak Reject" because they did
    /// not understand our approach...
    /// ```
    #[ghost]
    pub foreground: Option<Content>,

    /// The contents of the page(s).
    ///
    /// Multiple pages will be created if the content does not fit on a single
    /// page. A new page with the page properties prior to the function invocation
    /// will be created after the body has been typeset.
    #[external]
    #[required]
    pub body: Content,
}

impl Construct for PageElem {
    fn construct(engine: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        // The page constructor is special: It doesn't create a page element.
        // Instead, it just ensures that the passed content lives in a separate
        // page and styles it.
        let styles = Self::set(engine, args)?;
        let body = args.expect::<Content>("body")?;
        Ok(Content::sequence([
            PagebreakElem::shared_weak().clone(),
            // We put an effectless, invisible non-tag element on the page.
            // This has two desirable consequences:
            // - The page is kept even if the body is empty
            // - The page doesn't inherit shared styles from the body
            FlushElem::new().pack(),
            body,
            PagebreakElem::shared_boundary().clone(),
        ])
        .styled_with_map(styles))
    }
}

impl LocalName for PageElem {
    const KEY: &'static str = "page";
}

/// A manual page break.
///
/// Must not be used inside any containers.
///
/// # Example
/// ```example
/// The next page contains
/// more details on compound theory.
/// #pagebreak()
///
/// == Compound Theory
/// In 1984, the first ...
/// ```
///
/// Even without manual page breaks, content will be automatically paginated
/// based on the configured page size. You can set [the page height]($page.height)
/// to `{auto}` to let the page grow dynamically until a manual page break
/// occurs.
///
/// Pagination tries to avoid single lines of text at the top or bottom of a
/// page (these are called _widows_ and _orphans_). You can adjust the
/// [`text.costs`] parameter to disable this behavior.
#[elem(title = "Page Break")]
pub struct PagebreakElem {
    /// If `{true}`, the page break is skipped if the current page is already
    /// empty.
    #[default(false)]
    pub weak: bool,

    /// If given, ensures that the next page will be an even/odd page, with an
    /// empty page in between if necessary.
    ///
    /// ```example
    /// #set page(height: 30pt)
    ///
    /// First.
    /// #pagebreak(to: "odd")
    /// Third.
    /// ```
    pub to: Option<Parity>,

    /// Whether this pagebreak designates an end boundary of a page run. This is
    /// an even weaker version of pagebreak `weak` because it not only doesn't
    /// force an empty page, but also doesn't force its initial styles onto a
    /// staged empty page.
    #[internal]
    #[parse(None)]
    #[default(false)]
    pub boundary: bool,
}

impl PagebreakElem {
    /// Get the globally shared weak pagebreak element.
    pub fn shared_weak() -> &'static Content {
        singleton!(Content, PagebreakElem::new().with_weak(true).pack())
    }

    /// Get the globally shared boundary pagebreak element.
    pub fn shared_boundary() -> &'static Content {
        singleton!(
            Content,
            PagebreakElem::new().with_weak(true).with_boundary(true).pack()
        )
    }
}

/// A finished document with metadata and page frames.
#[derive(Debug, Default, Clone)]
pub struct PagedDocument {
    /// The document's finished pages.
    pub pages: Vec<Page>,
    /// Details about the document.
    pub info: DocumentInfo,
    /// Provides the ability to execute queries on the document.
    pub introspector: Introspector,
}

/// A finished page.
#[derive(Debug, Clone, Hash)]
pub struct Page {
    /// The frame that defines the page.
    pub frame: Frame,
    /// How the page is filled.
    ///
    /// - When `None`, the background is transparent.
    /// - When `Auto`, the background is transparent for PDF and white
    ///   for raster and SVG targets.
    ///
    /// Exporters should access the resolved value of this property through
    /// `fill_or_transparent()` or `fill_or_white()`.
    pub fill: Smart<Option<Paint>>,
    /// The page's numbering.
    pub numbering: Option<Numbering>,
    /// The page's supplement.
    pub supplement: Content,
    /// The logical page number (controlled by `counter(page)` and may thus not
    /// match the physical number).
    pub number: u64,
}

impl Page {
    /// Get the configured background or `None` if it is `Auto`.
    ///
    /// This is used in PDF export.
    pub fn fill_or_transparent(&self) -> Option<Paint> {
        self.fill.clone().unwrap_or(None)
    }

    /// Get the configured background or white if it is `Auto`.
    ///
    /// This is used in raster and SVG export.
    pub fn fill_or_white(&self) -> Option<Paint> {
        self.fill.clone().unwrap_or_else(|| Some(Color::WHITE.into()))
    }
}

/// Specification of the page's margins.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Margin {
    /// The margins for each side.
    pub sides: Sides<Option<Smart<Rel<Length>>>>,
    /// Whether to swap `left` and `right` to make them `inside` and `outside`
    /// (when to swap depends on the binding).
    pub two_sided: Option<bool>,
}

impl Margin {
    /// Create an instance with four equal components.
    pub fn splat(value: Option<Smart<Rel<Length>>>) -> Self {
        Self { sides: Sides::splat(value), two_sided: None }
    }
}

impl Default for Margin {
    fn default() -> Self {
        Self {
            sides: Sides::splat(Some(Smart::Auto)),
            two_sided: None,
        }
    }
}

impl Fold for Margin {
    fn fold(self, outer: Self) -> Self {
        Margin {
            sides: self.sides.fold(outer.sides),
            two_sided: self.two_sided.fold(outer.two_sided),
        }
    }
}

cast! {
    Margin,
    self => {
        let two_sided = self.two_sided.unwrap_or(false);
        if !two_sided && self.sides.is_uniform()
            && let Some(left) = self.sides.left {
                return left.into_value();
            }

        let mut dict = Dict::new();
        let mut handle = |key: &str, component: Option<Smart<Rel<Length>>>| {
            if let Some(c) = component {
                dict.insert(key.into(), c.into_value());
            }
        };

        handle("top", self.sides.top);
        handle("bottom", self.sides.bottom);
        if two_sided {
            handle("inside", self.sides.left);
            handle("outside", self.sides.right);
        } else {
            handle("left", self.sides.left);
            handle("right", self.sides.right);
        }

        Value::Dict(dict)
    },
    _: AutoValue => Self::splat(Some(Smart::Auto)),
    v: Rel<Length> => Self::splat(Some(Smart::Custom(v))),
    mut dict: Dict => {
        let mut take = |key| dict.take(key).ok().map(Value::cast).transpose();

        let rest = take("rest")?;
        let x = take("x")?.or(rest);
        let y = take("y")?.or(rest);
        let top = take("top")?.or(y);
        let bottom = take("bottom")?.or(y);
        let outside = take("outside")?;
        let inside = take("inside")?;
        let left = take("left")?;
        let right = take("right")?;

        let implicitly_two_sided = outside.is_some() || inside.is_some();
        let implicitly_not_two_sided = left.is_some() || right.is_some();
        if implicitly_two_sided && implicitly_not_two_sided {
            bail!("`inside` and `outside` are mutually exclusive with `left` and `right`");
        }

        // - If 'implicitly_two_sided' is false here, then
        //   'implicitly_not_two_sided' will be guaranteed to be true
        //    due to the previous two 'if' conditions.
        // - If both are false, this means that this margin change does not
        //   affect lateral margins, and thus shouldn't make a difference on
        //   the 'two_sided' attribute of this margin.
        let two_sided = (implicitly_two_sided || implicitly_not_two_sided)
            .then_some(implicitly_two_sided);

        dict.finish(&[
            "left", "top", "right", "bottom", "outside", "inside", "x", "y", "rest",
        ])?;

        Margin {
            sides: Sides {
                left: inside.or(left).or(x),
                top,
                right: outside.or(right).or(x),
                bottom,
            },
            two_sided,
        }
    }
}

/// Specification of the page's binding.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Binding {
    /// Bound on the left, as customary in LTR languages.
    Left,
    /// Bound on the right, as customary in RTL languages.
    Right,
}

impl Binding {
    /// Whether to swap left and right margin for the page with this number.
    pub fn swap(self, number: NonZeroUsize) -> bool {
        match self {
            // Left-bound must swap on even pages
            // (because it is correct on the first page).
            Self::Left => number.get() % 2 == 0,
            // Right-bound must swap on odd pages
            // (because it is wrong on the first page).
            Self::Right => number.get() % 2 == 1,
        }
    }
}

cast! {
    Binding,
    self => match self {
        Self::Left => Alignment::LEFT.into_value(),
        Self::Right => Alignment::RIGHT.into_value(),
    },
    v: Alignment => match v {
        Alignment::LEFT => Self::Left,
        Alignment::RIGHT => Self::Right,
        _ => bail!("must be `left` or `right`"),
    },
}

/// A list of page ranges to be exported.
#[derive(Debug, Clone)]
pub struct PageRanges(Vec<PageRange>);

/// A range of pages to export.
///
/// The range is one-indexed. For example, `1..=3` indicates the first, second
/// and third pages should be exported.
pub type PageRange = RangeInclusive<Option<NonZeroUsize>>;

impl PageRanges {
    /// Create new page ranges.
    pub fn new(ranges: Vec<PageRange>) -> Self {
        Self(ranges)
    }

    /// Check if a page, given its number, should be included when exporting the
    /// document while restricting the exported pages to these page ranges.
    /// This is the one-indexed version of 'includes_page_index'.
    pub fn includes_page(&self, page: NonZeroUsize) -> bool {
        self.includes_page_index(page.get() - 1)
    }

    /// Check if a page, given its index, should be included when exporting the
    /// document while restricting the exported pages to these page ranges.
    /// This is the zero-indexed version of 'includes_page'.
    pub fn includes_page_index(&self, page: usize) -> bool {
        let page = NonZeroUsize::try_from(page + 1).unwrap();
        self.0.iter().any(|range| match (range.start(), range.end()) {
            (Some(start), Some(end)) => (start..=end).contains(&&page),
            (Some(start), None) => (start..).contains(&&page),
            (None, Some(end)) => (..=end).contains(&&page),
            (None, None) => true,
        })
    }
}

/// Whether something should be even or odd.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Parity {
    /// Next page will be an even page.
    Even,
    /// Next page will be an odd page.
    Odd,
}

impl Parity {
    /// Whether the given number matches the parity.
    pub fn matches(self, number: usize) -> bool {
        match self {
            Self::Even => number % 2 == 0,
            Self::Odd => number % 2 == 1,
        }
    }
}

/// Specification of a paper.
#[derive(Debug, Copy, Clone, Hash)]
pub struct Paper {
    /// The name of the paper.
    name: &'static str,
    /// The width of the paper in millimeters.
    width: Scalar,
    /// The height of the paper in millimeters.
    height: Scalar,
}

impl Paper {
    /// The width of the paper.
    pub fn width(self) -> Abs {
        Abs::mm(self.width.get())
    }

    /// The height of the paper.
    pub fn height(self) -> Abs {
        Abs::mm(self.height.get())
    }
}

/// Defines paper constants and a paper parsing implementation.
macro_rules! papers {
    ($(($var:ident: $width:expr, $height: expr, $name:literal))*) => {
        /// Predefined papers.
        ///
        /// Each paper is parsable from its name in kebab-case.
        impl Paper {
            $(pub const $var: Self = Self {
                name: $name,
                width: Scalar::new($width),
                height: Scalar::new($height),
            };)*
        }

        impl FromStr for Paper {
            type Err = &'static str;

            fn from_str(name: &str) -> Result<Self, Self::Err> {
                match name.to_lowercase().as_str() {
                    $($name => Ok(Self::$var),)*
                    _ => Err("unknown paper size"),
                }
            }
        }

        cast! {
            Paper,
            self => self.name.into_value(),
            $(
                /// Produces a paper of the respective size.
                $name => Self::$var,
            )*
        }
    };
}

// All paper sizes in mm.
//
// Resources:
// - https://papersizes.io/
// - https://en.wikipedia.org/wiki/Paper_size
// - https://www.theedkins.co.uk/jo/units/oldunits/print.htm
// - https://vintagepaper.co/blogs/news/traditional-paper-sizes
papers! {
    // ---------------------------------------------------------------------- //
    // ISO 216 A Series
    (A0:  841.0, 1189.0, "a0")
    (A1:  594.0,  841.0, "a1")
    (A2:  420.0,  594.0, "a2")
    (A3:  297.0,  420.0, "a3")
    (A4:  210.0,  297.0, "a4")
    (A5:  148.0,  210.0, "a5")
    (A6:  105.0,  148.0, "a6")
    (A7:   74.0,  105.0, "a7")
    (A8:   52.0,   74.0, "a8")
    (A9:   37.0,   52.0, "a9")
    (A10:  26.0,   37.0, "a10")
    (A11:  18.0,   26.0, "a11")

    // ISO 216 B Series
    (ISO_B1: 707.0, 1000.0, "iso-b1")
    (ISO_B2: 500.0,  707.0,  "iso-b2")
    (ISO_B3: 353.0,  500.0,  "iso-b3")
    (ISO_B4: 250.0,  353.0,  "iso-b4")
    (ISO_B5: 176.0,  250.0,  "iso-b5")
    (ISO_B6: 125.0,  176.0,  "iso-b6")
    (ISO_B7:  88.0,  125.0,  "iso-b7")
    (ISO_B8:  62.0,   88.0,  "iso-b8")

    // ISO 216 C Series
    (ISO_C3: 324.0, 458.0, "iso-c3")
    (ISO_C4: 229.0, 324.0, "iso-c4")
    (ISO_C5: 162.0, 229.0, "iso-c5")
    (ISO_C6: 114.0, 162.0, "iso-c6")
    (ISO_C7:  81.0, 114.0, "iso-c7")
    (ISO_C8:  57.0,  81.0, "iso-c8")

    // DIN D Series (extension to ISO)
    (DIN_D3: 272.0, 385.0, "din-d3")
    (DIN_D4: 192.0, 272.0, "din-d4")
    (DIN_D5: 136.0, 192.0, "din-d5")
    (DIN_D6:  96.0, 136.0, "din-d6")
    (DIN_D7:  68.0,  96.0, "din-d7")
    (DIN_D8:  48.0,  68.0, "din-d8")

    // SIS (used in academia)
    (SIS_G5: 169.0, 239.0, "sis-g5")
    (SIS_E5: 115.0, 220.0, "sis-e5")

    // ANSI Extensions
    (ANSI_A: 216.0,  279.0, "ansi-a")
    (ANSI_B: 279.0,  432.0, "ansi-b")
    (ANSI_C: 432.0,  559.0, "ansi-c")
    (ANSI_D: 559.0,  864.0, "ansi-d")
    (ANSI_E: 864.0, 1118.0, "ansi-e")

    // ANSI Architectural Paper
    (ARCH_A:  229.0,  305.0, "arch-a")
    (ARCH_B:  305.0,  457.0, "arch-b")
    (ARCH_C:  457.0,  610.0, "arch-c")
    (ARCH_D:  610.0,  914.0, "arch-d")
    (ARCH_E1: 762.0, 1067.0, "arch-e1")
    (ARCH_E:  914.0, 1219.0, "arch-e")

    // JIS B Series
    (JIS_B0:  1030.0, 1456.0, "jis-b0")
    (JIS_B1:   728.0, 1030.0, "jis-b1")
    (JIS_B2:   515.0,  728.0, "jis-b2")
    (JIS_B3:   364.0,  515.0, "jis-b3")
    (JIS_B4:   257.0,  364.0, "jis-b4")
    (JIS_B5:   182.0,  257.0, "jis-b5")
    (JIS_B6:   128.0,  182.0, "jis-b6")
    (JIS_B7:    91.0,  128.0, "jis-b7")
    (JIS_B8:    64.0,   91.0, "jis-b8")
    (JIS_B9:    45.0,   64.0, "jis-b9")
    (JIS_B10:   32.0,   45.0, "jis-b10")
    (JIS_B11:   22.0,   32.0, "jis-b11")

    // SAC D Series
    (SAC_D0: 764.0, 1064.0, "sac-d0")
    (SAC_D1: 532.0,  760.0, "sac-d1")
    (SAC_D2: 380.0,  528.0, "sac-d2")
    (SAC_D3: 264.0,  376.0, "sac-d3")
    (SAC_D4: 188.0,  260.0, "sac-d4")
    (SAC_D5: 130.0,  184.0, "sac-d5")
    (SAC_D6:  92.0,  126.0, "sac-d6")

    // ISO 7810 ID
    (ISO_ID_1: 85.6, 53.98, "iso-id-1")
    (ISO_ID_2: 74.0, 105.0, "iso-id-2")
    (ISO_ID_3: 88.0, 125.0, "iso-id-3")

    // ---------------------------------------------------------------------- //
    // Asia
    (ASIA_F4: 210.0, 330.0, "asia-f4")

    // Japan
    (JP_SHIROKU_BAN_4: 264.0, 379.0, "jp-shiroku-ban-4")
    (JP_SHIROKU_BAN_5: 189.0, 262.0, "jp-shiroku-ban-5")
    (JP_SHIROKU_BAN_6: 127.0, 188.0, "jp-shiroku-ban-6")
    (JP_KIKU_4:        227.0, 306.0, "jp-kiku-4")
    (JP_KIKU_5:        151.0, 227.0, "jp-kiku-5")
    (JP_BUSINESS_CARD:  91.0,  55.0, "jp-business-card")

    // China
    (CN_BUSINESS_CARD: 90.0, 54.0, "cn-business-card")

    // Europe
    (EU_BUSINESS_CARD: 85.0, 55.0, "eu-business-card")

    // French Traditional (AFNOR)
    (FR_TELLIERE:          340.0, 440.0, "fr-tellière")
    (FR_COURONNE_ECRITURE: 360.0, 460.0, "fr-couronne-écriture")
    (FR_COURONNE_EDITION:  370.0, 470.0, "fr-couronne-édition")
    (FR_RAISIN:            500.0, 650.0, "fr-raisin")
    (FR_CARRE:             450.0, 560.0, "fr-carré")
    (FR_JESUS:             560.0, 760.0, "fr-jésus")

    // United Kingdom Imperial
    (UK_BRIEF:    406.4, 342.9, "uk-brief")
    (UK_DRAFT:    254.0, 406.4, "uk-draft")
    (UK_FOOLSCAP: 203.2, 330.2, "uk-foolscap")
    (UK_QUARTO:   203.2, 254.0, "uk-quarto")
    (UK_CROWN:    508.0, 381.0, "uk-crown")
    (UK_BOOK_A:   111.0, 178.0, "uk-book-a")
    (UK_BOOK_B:   129.0, 198.0, "uk-book-b")

    // Unites States
    (US_LETTER:         215.9,  279.4, "us-letter")
    (US_LEGAL:          215.9,  355.6, "us-legal")
    (US_TABLOID:        279.4,  431.8, "us-tabloid")
    (US_EXECUTIVE:      84.15,  266.7, "us-executive")
    (US_FOOLSCAP_FOLIO: 215.9,  342.9, "us-foolscap-folio")
    (US_STATEMENT:      139.7,  215.9, "us-statement")
    (US_LEDGER:         431.8,  279.4, "us-ledger")
    (US_OFICIO:         215.9, 340.36, "us-oficio")
    (US_GOV_LETTER:     203.2,  266.7, "us-gov-letter")
    (US_GOV_LEGAL:      215.9,  330.2, "us-gov-legal")
    (US_BUSINESS_CARD:   88.9,   50.8, "us-business-card")
    (US_DIGEST:         139.7,  215.9, "us-digest")
    (US_TRADE:          152.4,  228.6, "us-trade")

    // ---------------------------------------------------------------------- //
    // Other
    (NEWSPAPER_COMPACT:    280.0,    430.0, "newspaper-compact")
    (NEWSPAPER_BERLINER:   315.0,    470.0, "newspaper-berliner")
    (NEWSPAPER_BROADSHEET: 381.0,    578.0, "newspaper-broadsheet")
    (PRESENTATION_16_9:    297.0, 167.0625, "presentation-16-9")
    (PRESENTATION_4_3:     280.0,    210.0, "presentation-4-3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paged_document_is_send_and_sync() {
        fn ensure_send_and_sync<T: Send + Sync>() {}
        ensure_send_and_sync::<PagedDocument>();
    }
}
