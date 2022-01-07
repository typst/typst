//! Pages of paper.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use super::prelude::*;
use super::ColumnsNode;

/// Layouts its child onto one or multiple pages.
#[derive(Clone, PartialEq, Hash)]
pub struct PageNode(pub PackedNode);

#[properties]
impl PageNode {
    /// The unflipped width of the page.
    pub const WIDTH: Smart<Length> = Smart::Custom(Paper::default().width());
    /// The unflipped height of the page.
    pub const HEIGHT: Smart<Length> = Smart::Custom(Paper::default().height());
    /// The class of paper. Defines the default margins.
    pub const CLASS: PaperClass = Paper::default().class();
    /// Whether the page is flipped into landscape orientation.
    pub const FLIPPED: bool = false;
    /// The left margin.
    pub const LEFT: Smart<Linear> = Smart::Auto;
    /// The right margin.
    pub const RIGHT: Smart<Linear> = Smart::Auto;
    /// The top margin.
    pub const TOP: Smart<Linear> = Smart::Auto;
    /// The bottom margin.
    pub const BOTTOM: Smart<Linear> = Smart::Auto;
    /// The page's background color.
    pub const FILL: Option<Paint> = None;
    /// How many columns the page has.
    pub const COLUMNS: NonZeroUsize = NonZeroUsize::new(1).unwrap();
    /// How much space is between the page's columns.
    pub const COLUMN_GUTTER: Linear = Relative::new(0.04).into();
}

impl Construct for PageNode {
    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        Ok(Node::Page(Self(args.expect("body")?)))
    }
}

impl Set for PageNode {
    fn set(args: &mut Args, styles: &mut StyleMap) -> TypResult<()> {
        if let Some(paper) = args.named::<Paper>("paper")?.or_else(|| args.find()) {
            styles.set(Self::CLASS, paper.class());
            styles.set(Self::WIDTH, Smart::Custom(paper.width()));
            styles.set(Self::HEIGHT, Smart::Custom(paper.height()));
        }

        if let Some(width) = args.named("width")? {
            styles.set(Self::CLASS, PaperClass::Custom);
            styles.set(Self::WIDTH, width);
        }

        if let Some(height) = args.named("height")? {
            styles.set(Self::CLASS, PaperClass::Custom);
            styles.set(Self::HEIGHT, height);
        }

        let margins = args.named("margins")?;
        styles.set_opt(Self::LEFT, args.named("left")?.or(margins));
        styles.set_opt(Self::TOP, args.named("top")?.or(margins));
        styles.set_opt(Self::RIGHT, args.named("right")?.or(margins));
        styles.set_opt(Self::BOTTOM, args.named("bottom")?.or(margins));

        styles.set_opt(Self::FLIPPED, args.named("flipped")?);
        styles.set_opt(Self::FILL, args.named("fill")?);
        styles.set_opt(Self::COLUMNS, args.named("columns")?);
        styles.set_opt(Self::COLUMN_GUTTER, args.named("column-gutter")?);

        Ok(())
    }
}

impl PageNode {
    /// Layout the page run into a sequence of frames, one per page.
    pub fn layout(&self, ctx: &mut LayoutContext, styles: StyleChain) -> Vec<Rc<Frame>> {
        // When one of the lengths is infinite the page fits its content along
        // that axis.
        let width = styles.get(Self::WIDTH).unwrap_or(Length::inf());
        let height = styles.get(Self::HEIGHT).unwrap_or(Length::inf());
        let mut size = Size::new(width, height);
        if styles.get(Self::FLIPPED) {
            std::mem::swap(&mut size.x, &mut size.y);
        }

        // Determine the margins.
        let class = styles.get(Self::CLASS);
        let default = class.default_margins();
        let padding = Sides {
            left: styles.get(Self::LEFT).unwrap_or(default.left),
            right: styles.get(Self::RIGHT).unwrap_or(default.right),
            top: styles.get(Self::TOP).unwrap_or(default.top),
            bottom: styles.get(Self::BOTTOM).unwrap_or(default.bottom),
        };

        let mut child = self.0.clone();

        // Realize columns with columns node.
        let columns = styles.get(Self::COLUMNS);
        if columns.get() > 1 {
            child = ColumnsNode {
                columns,
                gutter: styles.get(Self::COLUMN_GUTTER),
                child: self.0.clone(),
            }
            .pack();
        }

        // Realize margins with padding node.
        child = child.padded(padding);

        // Layout the child.
        let expand = size.map(Length::is_finite);
        let regions = Regions::repeat(size, size, expand);
        let mut frames: Vec<_> = child
            .layout(ctx, &regions, styles)
            .into_iter()
            .map(|c| c.item)
            .collect();

        // Add background fill if requested.
        if let Some(fill) = styles.get(Self::FILL) {
            for frame in &mut frames {
                let shape = Shape::filled(Geometry::Rect(frame.size), fill);
                Rc::make_mut(frame).prepend(Point::zero(), Element::Shape(shape));
            }
        }

        frames
    }
}

impl Debug for PageNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Page(")?;
        self.0.fmt(f)?;
        f.write_str(")")
    }
}

/// `pagebreak`: Start a new page.
pub fn pagebreak(_: &mut EvalContext, _: &mut Args) -> TypResult<Value> {
    Ok(Value::Node(Node::Pagebreak))
}

/// Specification of a paper.
#[derive(Debug, Copy, Clone)]
pub struct Paper {
    /// The broad class this paper belongs to.
    class: PaperClass,
    /// The width of the paper in millimeters.
    width: f64,
    /// The height of the paper in millimeters.
    height: f64,
}

impl Paper {
    /// The class of the paper.
    pub fn class(self) -> PaperClass {
        self.class
    }

    /// The width of the paper.
    pub fn width(self) -> Length {
        Length::mm(self.width)
    }

    /// The height of the paper.
    pub fn height(self) -> Length {
        Length::mm(self.height)
    }
}

impl Default for Paper {
    fn default() -> Self {
        Paper::A4
    }
}

/// Defines paper constants and a paper parsing implementation.
macro_rules! papers {
    ($(($var:ident: $class:ident, $width:expr, $height: expr, $($pats:tt)*))*) => {
        /// Predefined papers.
        ///
        /// Each paper is parsable from its name in kebab-case.
        impl Paper {
            $(pub const $var: Self = Self {
                class: PaperClass::$class,
                width: $width,
                height: $height,
            };)*
        }

        impl FromStr for Paper {
            type Err = PaperError;

            fn from_str(name: &str) -> Result<Self, Self::Err> {
                match name.to_lowercase().as_str() {
                    $($($pats)* => Ok(Self::$var),)*
                    _ => Err(PaperError),
                }
            }
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
    (A0:  Base, 841.0, 1189.0, "a0")
    (A1:  Base, 594.0,  841.0, "a1")
    (A2:  Base, 420.0,  594.0, "a2")
    (A3:  Base, 297.0,  420.0, "a3")
    (A4:  Base, 210.0,  297.0, "a4")
    (A5:  Base, 148.0,  210.0, "a5")
    (A6:  Book, 105.0,  148.0, "a6")
    (A7:  Base,  74.0,  105.0, "a7")
    (A8:  Base,  52.0,   74.0, "a8")
    (A9:  Base,  37.0,   52.0, "a9")
    (A10: Base,  26.0,   37.0, "a10")
    (A11: Base,  18.0,   26.0, "a11")

    // ISO 216 B Series
    (ISO_B1: Base, 707.0, 1000.0, "iso-b1")
    (ISO_B2: Base, 500.0, 707.0,  "iso-b2")
    (ISO_B3: Base, 353.0, 500.0,  "iso-b3")
    (ISO_B4: Base, 250.0, 353.0,  "iso-b4")
    (ISO_B5: Book, 176.0, 250.0,  "iso-b5")
    (ISO_B6: Book, 125.0, 176.0,  "iso-b6")
    (ISO_B7: Base,  88.0, 125.0,  "iso-b7")
    (ISO_B8: Base,  62.0,  88.0,  "iso-b8")

    // ISO 216 C Series
    (ISO_C3: Base, 324.0, 458.0, "iso-c3")
    (ISO_C4: Base, 229.0, 324.0, "iso-c4")
    (ISO_C5: Base, 162.0, 229.0, "iso-c5")
    (ISO_C6: Base, 114.0, 162.0, "iso-c6")
    (ISO_C7: Base,  81.0, 114.0, "iso-c7")
    (ISO_C8: Base,  57.0,  81.0, "iso-c8")

    // DIN D Series (extension to ISO)
    (DIN_D3: Base, 272.0, 385.0, "din-d3")
    (DIN_D4: Base, 192.0, 272.0, "din-d4")
    (DIN_D5: Base, 136.0, 192.0, "din-d5")
    (DIN_D6: Base,  96.0, 136.0, "din-d6")
    (DIN_D7: Base,  68.0,  96.0, "din-d7")
    (DIN_D8: Base,  48.0,  68.0, "din-d8")

    // SIS (used in academia)
    (SIS_G5: Base, 169.0, 239.0, "sis-g5")
    (SIS_E5: Base, 115.0, 220.0, "sis-e5")

    // ANSI Extensions
    (ANSI_A: Base, 216.0,  279.0, "ansi-a")
    (ANSI_B: Base, 279.0,  432.0, "ansi-b")
    (ANSI_C: Base, 432.0,  559.0, "ansi-c")
    (ANSI_D: Base, 559.0,  864.0, "ansi-d")
    (ANSI_E: Base, 864.0, 1118.0, "ansi-e")

    // ANSI Architectural Paper
    (ARCH_A:  Base, 229.0,  305.0, "arch-a")
    (ARCH_B:  Base, 305.0,  457.0, "arch-b")
    (ARCH_C:  Base, 457.0,  610.0, "arch-c")
    (ARCH_D:  Base, 610.0,  914.0, "arch-d")
    (ARCH_E1: Base, 762.0, 1067.0, "arch-e1")
    (ARCH_E:  Base, 914.0, 1219.0, "arch-e")

    // JIS B Series
    (JIS_B0:  Base, 1030.0, 1456.0, "jis-b0")
    (JIS_B1:  Base,  728.0, 1030.0, "jis-b1")
    (JIS_B2:  Base,  515.0,  728.0, "jis-b2")
    (JIS_B3:  Base,  364.0,  515.0, "jis-b3")
    (JIS_B4:  Base,  257.0,  364.0, "jis-b4")
    (JIS_B5:  Base,  182.0,  257.0, "jis-b5")
    (JIS_B6:  Base,  128.0,  182.0, "jis-b6")
    (JIS_B7:  Base,   91.0,  128.0, "jis-b7")
    (JIS_B8:  Base,   64.0,   91.0, "jis-b8")
    (JIS_B9:  Base,   45.0,   64.0, "jis-b9")
    (JIS_B10: Base,   32.0,   45.0, "jis-b10")
    (JIS_B11: Base,   22.0,   32.0, "jis-b11")

    // SAC D Series
    (SAC_D0: Base, 764.0, 1064.0, "sac-d0")
    (SAC_D1: Base, 532.0,  760.0, "sac-d1")
    (SAC_D2: Base, 380.0,  528.0, "sac-d2")
    (SAC_D3: Base, 264.0,  376.0, "sac-d3")
    (SAC_D4: Base, 188.0,  260.0, "sac-d4")
    (SAC_D5: Base, 130.0,  184.0, "sac-d5")
    (SAC_D6: Base,  92.0,  126.0, "sac-d6")

    // ISO 7810 ID
    (ISO_ID_1: Base, 85.6, 53.98, "iso-id-1")
    (ISO_ID_2: Base, 74.0, 105.0, "iso-id-2")
    (ISO_ID_3: Base, 88.0, 125.0, "iso-id-3")

    // ---------------------------------------------------------------------- //
    // Asia
    (ASIA_F4: Base, 210.0, 330.0, "asia-f4")

    // Japan
    (JP_SHIROKU_BAN_4: Base, 264.0, 379.0, "jp-shiroku-ban-4")
    (JP_SHIROKU_BAN_5: Base, 189.0, 262.0, "jp-shiroku-ban-5")
    (JP_SHIROKU_BAN_6: Base, 127.0, 188.0, "jp-shiroku-ban-6")
    (JP_KIKU_4:        Base, 227.0, 306.0, "jp-kiku-4")
    (JP_KIKU_5:        Base, 151.0, 227.0, "jp-kiku-5")
    (JP_BUSINESS_CARD: Base,  91.0,  55.0, "jp-business-card")

    // China
    (CN_BUSINESS_CARD: Base, 90.0, 54.0, "cn-business-card")

    // Europe
    (EU_BUSINESS_CARD: Base, 85.0, 55.0, "eu-business-card")

    // French Traditional (AFNOR)
    (FR_TELLIERE:          Base, 340.0, 440.0, "fr-tellière")
    (FR_COURONNE_ECRITURE: Base, 360.0, 460.0, "fr-couronne-écriture")
    (FR_COURONNE_EDITION:  Base, 370.0, 470.0, "fr-couronne-édition")
    (FR_RAISIN:            Base, 500.0, 650.0, "fr-raisin")
    (FR_CARRE:             Base, 450.0, 560.0, "fr-carré")
    (FR_JESUS:             Base, 560.0, 760.0, "fr-jésus")

    // United Kingdom Imperial
    (UK_BRIEF:    Base, 406.4, 342.9, "uk-brief")
    (UK_DRAFT:    Base, 254.0, 406.4, "uk-draft")
    (UK_FOOLSCAP: Base, 203.2, 330.2, "uk-foolscap")
    (UK_QUARTO:   Base, 203.2, 254.0, "uk-quarto")
    (UK_CROWN:    Base, 508.0, 381.0, "uk-crown")
    (UK_BOOK_A:   Book, 111.0, 178.0, "uk-book-a")
    (UK_BOOK_B:   Book, 129.0, 198.0, "uk-book-b")

    // Unites States
    (US_LETTER:         US,   215.9,  279.4, "us-letter")
    (US_LEGAL:          US,   215.9,  355.6, "us-legal")
    (US_TABLOID:        US,   279.4,  431.8, "us-tabloid")
    (US_EXECUTIVE:      US,  184.15,  266.7, "us-executive")
    (US_FOOLSCAP_FOLIO: US,   215.9,  342.9, "us-foolscap-folio")
    (US_STATEMENT:      US,   139.7,  215.9, "us-statement")
    (US_LEDGER:         US,   431.8,  279.4, "us-ledger")
    (US_OFICIO:         US,   215.9, 340.36, "us-oficio")
    (US_GOV_LETTER:     US,   203.2,  266.7, "us-gov-letter")
    (US_GOV_LEGAL:      US,   215.9,  330.2, "us-gov-legal")
    (US_BUSINESS_CARD:  Base,  88.9,   50.8, "us-business-card")
    (US_DIGEST:         Book, 139.7,  215.9, "us-digest")
    (US_TRADE:          Book, 152.4,  228.6, "us-trade")

    // ---------------------------------------------------------------------- //
    // Other
    (NEWSPAPER_COMPACT:    Newspaper, 280.0, 430.0,    "newspaper-compact")
    (NEWSPAPER_BERLINER:   Newspaper, 315.0, 470.0,    "newspaper-berliner")
    (NEWSPAPER_BROADSHEET: Newspaper, 381.0, 578.0,    "newspaper-broadsheet")
    (PRESENTATION_16_9:    Base,      297.0, 167.0625, "presentation-16-9")
    (PRESENTATION_4_3:     Base,      280.0, 210.0,    "presentation-4-3")
}

castable! {
    Paper,
    Expected: "string",
    Value::Str(string) => Paper::from_str(&string).map_err(|e| e.to_string())?,
}

/// Defines default margins for a class of related papers.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PaperClass {
    Custom,
    Base,
    US,
    Newspaper,
    Book,
}

impl PaperClass {
    /// The default margins for this page class.
    fn default_margins(self) -> Sides<Linear> {
        let f = |r| Relative::new(r).into();
        let s = |l, t, r, b| Sides::new(f(l), f(t), f(r), f(b));
        match self {
            Self::Custom => s(0.1190, 0.0842, 0.1190, 0.0842),
            Self::Base => s(0.1190, 0.0842, 0.1190, 0.0842),
            Self::US => s(0.1760, 0.1092, 0.1760, 0.0910),
            Self::Newspaper => s(0.0455, 0.0587, 0.0455, 0.0294),
            Self::Book => s(0.1200, 0.0852, 0.1500, 0.0965),
        }
    }
}

/// The error when parsing a [`Paper`] from a string fails.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PaperError;

impl Display for PaperError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("invalid paper name")
    }
}

impl std::error::Error for PaperError {}
