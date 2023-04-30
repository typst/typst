use std::ptr;
use std::str::FromStr;

use super::{AlignElem, ColumnsElem};
use crate::meta::{Counter, CounterKey, Numbering};
use crate::prelude::*;

/// Layouts its child onto one or multiple pages.
///
/// Although this function is primarily used in set rules to affect page
/// properties, it can also be used to explicitly render its argument onto
/// a set of pages of its own.
///
/// Pages can be set to use `{auto}` as their width or height. In this case,
/// the pages will grow to fit their content on the respective axis.
///
/// ## Example
/// ```example
/// >>> #set page(margin: auto)
/// #set page("us-letter")
///
/// There you go, US friends!
/// ```
///
/// Display: Page
/// Category: layout
#[element]
pub struct PageElem {
    /// A standard paper size to set width and height. When this is not
    /// specified, Typst defaults to `{"a4"}` paper.
    #[external]
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
    #[resolve]
    #[parse(
        let paper = args.named_or_find::<Paper>("paper")?;
        args.named("width")?
            .or_else(|| paper.map(|paper| Smart::Custom(paper.width().into())))
    )]
    #[default(Smart::Custom(Paper::A4.width().into()))]
    pub width: Smart<Length>,

    /// The height of the page.
    ///
    /// If this is set to `{auto}`, page breaks can only be triggered manually
    /// by inserting a [page break]($func/pagebreak). Most examples throughout
    /// this documentation use `{auto}` for the height of the page to
    /// dynamically grow and shrink to fit their content.
    #[resolve]
    #[parse(
        args.named("height")?
            .or_else(|| paper.map(|paper| Smart::Custom(paper.height().into())))
    )]
    #[default(Smart::Custom(Paper::A4.height().into()))]
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
    pub flipped: bool,

    /// When `{true}`, the left and right margins will be swapped on even pages
    /// to achieve inside and outside margins. This is commonly used when
    /// writing books, for example. Defaults to `{false}`.
    ///
    /// ```example
    /// #set page(two-sided: true, margin: (left: 1in, right: 1.25in))
    /// ```
    #[default(false)]
    pub two_sided: bool,

    /// The page's margins.
    ///
    /// - A single length: The same margin on all sides.
    /// - `{auto}`: The margin is set to the default value for the page's size.
    /// - A dictionary: With a dictionary, the margins can be set individually.
    ///   The dictionary can contain the following keys in order of precedence:
    ///   - `top`: The top margin.
    ///   - `right`: The right margin.
    ///   - `bottom`: The bottom margin.
    ///   - `left`: The left margin.
    ///   - `x`: The horizontal margins.
    ///   - `y`: The vertical margins.
    ///   - `rest`: The margins on all sides except those for which the
    ///     dictionary explicitly sets a size.
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
    pub margin: Sides<Option<Smart<Rel<Length>>>>,

    /// How many columns the page has.
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
    pub columns: NonZeroUsize,

    /// The page's background color.
    ///
    /// This instructs the printer to color the complete page with the given
    /// color. If you are considering larger production runs, it may be more
    /// environmentally friendly and cost-effective to source pre-dyed pages and
    /// not set this property.
    ///
    /// ```example
    /// #set page(fill: rgb("444352"))
    /// #set text(fill: rgb("fdfdfd"))
    /// *Dark mode enabled.*
    /// ```
    pub fill: Option<Paint>,

    /// How to [number]($func/numbering) the pages.
    ///
    /// If an explicit `footer` is given, the numbering is ignored.
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
    pub numbering: Option<Numbering>,

    /// The alignment of the page numbering.
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
    #[default(Align::Center.into())]
    pub number_align: Axes<Option<GenAlign>>,

    /// The page's header. Fills the top margin of each page.
    ///
    /// ```example
    /// #set par(justify: true)
    /// #set page(
    ///   margin: (top: 32pt, bottom: 20pt),
    ///   header: [
    ///     #set text(8pt)
    ///     #smallcaps[Typst Academcy]
    ///     #h(1fr) _Exercise Sheet 3_
    ///   ],
    /// )
    ///
    /// #lorem(19)
    /// ```
    pub header: Option<Content>,

    /// The amount the header is raised into the top margin.
    #[resolve]
    #[default(Ratio::new(0.3).into())]
    pub header_ascent: Rel<Length>,

    /// The page's footer. Fills the bottom margin of each page.
    ///
    /// For just a page number, the `numbering` property, typically suffices. If
    /// you want to create a custom footer, but still display the page number,
    /// you can directly access the [page counter]($func/counter).
    ///
    /// ```example
    /// #set par(justify: true)
    /// #set page(
    ///   height: 100pt,
    ///   margin: 20pt,
    ///   footer: [
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
    pub footer: Option<Content>,

    /// The amount the footer is lowered into the bottom margin.
    #[resolve]
    #[default(Ratio::new(0.3).into())]
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
    pub background: Option<Content>,

    /// Content in the page's foreground.
    ///
    /// This content will overlay the page's body.
    ///
    /// ```example
    /// #set page(foreground: text(24pt)[🥸])
    ///
    /// Reviewer 2 has marked our paper
    /// "Weak Reject" because they did
    /// not understand our approach...
    /// ```
    pub foreground: Option<Content>,

    /// The contents of the page(s).
    ///
    /// Multiple pages will be created if the content does not fit on a single
    /// page. A new page with the page properties prior to the function invocation
    /// will be created after the body has been typeset.
    #[required]
    pub body: Content,
}

impl PageElem {
    /// A document can consist of multiple `PageElem`s, one per run of pages
    /// with equal properties (not one per actual output page!). The `number` is
    /// the physical page number of the first page of this run. It is mutated
    /// while we post-process the pages in this function. This function returns
    /// a fragment consisting of multiple frames, one per output page of this
    /// page run.
    #[tracing::instrument(skip_all)]
    pub fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        mut number: NonZeroUsize,
    ) -> SourceResult<Fragment> {
        tracing::info!("Page layout");

        // When one of the lengths is infinite the page fits its content along
        // that axis.
        let width = self.width(styles).unwrap_or(Abs::inf());
        let height = self.height(styles).unwrap_or(Abs::inf());
        let mut size = Size::new(width, height);
        if self.flipped(styles) {
            std::mem::swap(&mut size.x, &mut size.y);
        }

        let mut min = width.min(height);
        if !min.is_finite() {
            min = Paper::A4.width();
        }

        // Determine the margins.
        let default = Rel::from(0.1190 * min);
        let margin = self
            .margin(styles)
            .map(|side| side.unwrap_or(default))
            .resolve(styles)
            .relative_to(size);

        // Realize columns.
        let mut child = self.body();
        let columns = self.columns(styles);
        if columns.get() > 1 {
            child = ColumnsElem::new(child).with_count(columns).pack();
        }

        // Layout the child.
        let area = size - margin.sum_by_axis();
        let regions = Regions::repeat(area, area.map(Abs::is_finite));
        let mut fragment = child.layout(vt, styles, regions)?;

        let fill = self.fill(styles);
        let foreground = self.foreground(styles);
        let background = self.background(styles);
        let header = self.header(styles);
        let header_ascent = self.header_ascent(styles);
        let footer = self.footer(styles).or_else(|| {
            self.numbering(styles).map(|numbering| {
                let both = match &numbering {
                    Numbering::Pattern(pattern) => pattern.pieces() >= 2,
                    Numbering::Func(_) => true,
                };
                Counter::new(CounterKey::Page)
                    .display(Some(numbering), both)
                    .aligned(self.number_align(styles))
            })
        });
        let footer_descent = self.footer_descent(styles);

        let numbering_meta = FrameItem::Meta(
            Meta::PageNumbering(self.numbering(styles).into()),
            Size::zero(),
        );

        // Post-process pages.
        for frame in fragment.iter_mut() {
            tracing::info!("Layouting page #{number}");

            // The padded width of the page's content without margins.
            let pw = frame.width();

            // Make a clone of the original margin in case you need to modify
            // the margins for a two-sided document.
            let mut margin = margin;

            // Swap right and left margins if the document is two-sided and the
            // page number is even.
            if self.two_sided(styles) && number.get() % 2 == 0 {
                std::mem::swap(&mut margin.left, &mut margin.right);
            }

            // Realize margins.
            frame.set_size(frame.size() + margin.sum_by_axis());
            frame.translate(Point::new(margin.left, margin.top));
            frame.push(Point::zero(), numbering_meta.clone());

            // The page size with margins.
            let size = frame.size();

            // Realize overlays.
            for (name, marginal) in [
                ("header", &header),
                ("footer", &footer),
                ("background", &background),
                ("foreground", &foreground),
            ] {
                tracing::info!("Layouting {name}");

                let Some(content) = marginal else { continue };

                let (pos, area, align);
                if ptr::eq(marginal, &header) {
                    let ascent = header_ascent.relative_to(margin.top);
                    pos = Point::with_x(margin.left);
                    area = Size::new(pw, margin.top - ascent);
                    align = Align::Bottom.into();
                } else if ptr::eq(marginal, &footer) {
                    let descent = footer_descent.relative_to(margin.bottom);
                    pos = Point::new(margin.left, size.y - margin.bottom + descent);
                    area = Size::new(pw, margin.bottom - descent);
                    align = Align::Top.into();
                } else {
                    pos = Point::zero();
                    area = size;
                    align = Align::CENTER_HORIZON.into();
                };

                let pod = Regions::one(area, Axes::splat(true));
                let sub = content
                    .clone()
                    .styled(AlignElem::set_alignment(align))
                    .layout(vt, styles, pod)?
                    .into_frame();

                if ptr::eq(marginal, &header) || ptr::eq(marginal, &background) {
                    frame.prepend_frame(pos, sub);
                } else {
                    frame.push_frame(pos, sub);
                }
            }

            if let Some(fill) = &fill {
                frame.fill(fill.clone());
            }

            number = number.saturating_add(1);
        }

        Ok(fragment)
    }
}

/// A manual page break.
///
/// Must not be used inside any containers.
///
/// ## Example
/// ```example
/// The next page contains
/// more details on compound theory.
/// #pagebreak()
///
/// == Compound Theory
/// In 1984, the first ...
/// ```
///
/// Display: Page Break
/// Category: layout
#[element]
pub struct PagebreakElem {
    /// If `{true}`, the page break is skipped if the current page is already
    /// empty.
    #[default(false)]
    pub weak: bool,
}

/// A header, footer, foreground or background definition.
#[derive(Debug, Clone, Hash)]
pub enum Marginal {
    /// Bare content.
    Content(Content),
    /// A closure mapping from a page number to content.
    Func(Func),
}

impl Marginal {
    /// Resolve the marginal based on the page number.
    pub fn resolve(&self, vt: &mut Vt, page: usize) -> SourceResult<Content> {
        Ok(match self {
            Self::Content(content) => content.clone(),
            Self::Func(func) => func.call_vt(vt, [Value::Int(page as i64)])?.display(),
        })
    }
}

cast_from_value! {
    Marginal,
    v: Content => Self::Content(v),
    v: Func => Self::Func(v),
}

cast_to_value! {
    v: Marginal => match v {
        Marginal::Content(v) => v.into(),
        Marginal::Func(v) => v.into(),
    }
}

/// Specification of a paper.
#[derive(Debug, Copy, Clone, Hash)]
pub struct Paper {
    /// The width of the paper in millimeters.
    width: Scalar,
    /// The height of the paper in millimeters.
    height: Scalar,
}

impl Paper {
    /// The width of the paper.
    pub fn width(self) -> Abs {
        Abs::mm(self.width.0)
    }

    /// The height of the paper.
    pub fn height(self) -> Abs {
        Abs::mm(self.height.0)
    }
}

/// Defines paper constants and a paper parsing implementation.
macro_rules! papers {
    ($(($var:ident: $width:expr, $height: expr, $pat:literal))*) => {
        /// Predefined papers.
        ///
        /// Each paper is parsable from its name in kebab-case.
        impl Paper {
            $(pub const $var: Self = Self {
                width: Scalar($width),
                height: Scalar($height),
            };)*
        }

        impl FromStr for Paper {
            type Err = &'static str;

            fn from_str(name: &str) -> Result<Self, Self::Err> {
                match name.to_lowercase().as_str() {
                    $($pat => Ok(Self::$var),)*
                    _ => Err("unknown paper size"),
                }
            }
        }

        cast_from_value! {
            Paper,
            $(
                /// Produces a paper of the respective size.
                $pat => Self::$var,
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
