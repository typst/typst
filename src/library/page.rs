//! Pages of paper.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use super::prelude::*;
use super::ColumnsNode;

/// Layouts its child onto one or multiple pages.
#[derive(Clone, PartialEq, Hash)]
pub struct PageNode(pub LayoutNode);

#[class]
impl PageNode {
    /// The unflipped width of the page.
    pub const WIDTH: Smart<Length> = Smart::Custom(Paper::A4.width());
    /// The unflipped height of the page.
    pub const HEIGHT: Smart<Length> = Smart::Custom(Paper::A4.height());
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
    /// The page's header.
    pub const HEADER: Marginal = Marginal::None;
    /// The page's footer.
    pub const FOOTER: Marginal = Marginal::None;

    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Template> {
        Ok(Template::Page(Self(args.expect("body")?)))
    }

    fn set(args: &mut Args, styles: &mut StyleMap) -> TypResult<()> {
        if let Some(paper) = args.named_or_find::<Paper>("paper")? {
            styles.set(Self::WIDTH, Smart::Custom(paper.width()));
            styles.set(Self::HEIGHT, Smart::Custom(paper.height()));
        }

        styles.set_opt(Self::WIDTH, args.named("width")?);
        styles.set_opt(Self::HEIGHT, args.named("height")?);

        let all = args.named("margins")?;
        let hor = args.named("horizontal")?;
        let ver = args.named("vertical")?;
        styles.set_opt(Self::LEFT, args.named("left")?.or(hor).or(all));
        styles.set_opt(Self::TOP, args.named("top")?.or(ver).or(all));
        styles.set_opt(Self::RIGHT, args.named("right")?.or(hor).or(all));
        styles.set_opt(Self::BOTTOM, args.named("bottom")?.or(ver).or(all));

        styles.set_opt(Self::FLIPPED, args.named("flipped")?);
        styles.set_opt(Self::FILL, args.named("fill")?);
        styles.set_opt(Self::COLUMNS, args.named("columns")?);
        styles.set_opt(Self::HEADER, args.named("header")?);
        styles.set_opt(Self::FOOTER, args.named("footer")?);

        Ok(())
    }
}

impl PageNode {
    /// Layout the page run into a sequence of frames, one per page.
    pub fn layout(
        &self,
        vm: &mut Vm,
        mut page: usize,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        // When one of the lengths is infinite the page fits its content along
        // that axis.
        let width = styles.get(Self::WIDTH).unwrap_or(Length::inf());
        let height = styles.get(Self::HEIGHT).unwrap_or(Length::inf());
        let mut size = Size::new(width, height);
        if styles.get(Self::FLIPPED) {
            std::mem::swap(&mut size.x, &mut size.y);
        }

        let mut min = width.min(height);
        if min.is_infinite() {
            min = Paper::A4.width();
        }

        // Determine the margins.
        let default = Linear::from(0.1190 * min);
        let padding = Sides {
            left: styles.get(Self::LEFT).unwrap_or(default),
            right: styles.get(Self::RIGHT).unwrap_or(default),
            top: styles.get(Self::TOP).unwrap_or(default),
            bottom: styles.get(Self::BOTTOM).unwrap_or(default),
        };

        let mut child = self.0.clone();

        // Realize columns with columns node.
        let columns = styles.get(Self::COLUMNS);
        if columns.get() > 1 {
            child = ColumnsNode { columns, child: self.0.clone() }.pack();
        }

        // Realize margins.
        child = child.padded(padding);

        // Realize background fill.
        if let Some(fill) = styles.get(Self::FILL) {
            child = child.filled(fill);
        }

        // Layout the child.
        let regions = Regions::repeat(size, size, size.map(Length::is_finite));
        let mut frames = child.layout(vm, &regions, styles)?;

        let header = styles.get_ref(Self::HEADER);
        let footer = styles.get_ref(Self::FOOTER);

        // Realize header and footer.
        for frame in &mut frames {
            let size = frame.size;
            let padding = padding.resolve(size);
            for (y, h, marginal) in [
                (Length::zero(), padding.top, header),
                (size.y - padding.bottom, padding.bottom, footer),
            ] {
                if let Some(template) = marginal.resolve(vm, page)? {
                    let pos = Point::new(padding.left, y);
                    let w = size.x - padding.left - padding.right;
                    let area = Size::new(w, h);
                    let pod = Regions::one(area, area, area.map(Length::is_finite));
                    let sub = template.layout(vm, &pod, styles)?.remove(0);
                    Arc::make_mut(frame).push_frame(pos, sub);
                }
            }

            page += 1;
        }

        Ok(frames)
    }
}

impl Debug for PageNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Page(")?;
        self.0.fmt(f)?;
        f.write_str(")")
    }
}

/// A page break.
pub struct PagebreakNode;

#[class]
impl PagebreakNode {
    fn construct(_: &mut Vm, _: &mut Args) -> TypResult<Template> {
        Ok(Template::Pagebreak)
    }
}

/// A header or footer definition.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Marginal {
    /// Nothing,
    None,
    /// A bare template.
    Template(Template),
    /// A closure mapping from a page number to a template.
    Func(Func, Span),
}

impl Marginal {
    /// Resolve the marginal based on the page number.
    pub fn resolve(&self, vm: &mut Vm, page: usize) -> TypResult<Option<Template>> {
        Ok(match self {
            Self::None => None,
            Self::Template(template) => Some(template.clone()),
            Self::Func(func, span) => {
                let args = Args::from_values(*span, [Value::Int(page as i64)]);
                func.call(vm, args)?.cast().at(*span)?
            }
        })
    }
}

impl Cast<Spanned<Value>> for Marginal {
    fn is(value: &Spanned<Value>) -> bool {
        matches!(&value.v, Value::Template(_) | Value::Func(_))
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        match value.v {
            Value::None => Ok(Self::None),
            Value::Template(v) => Ok(Self::Template(v)),
            Value::Func(v) => Ok(Self::Func(v, value.span)),
            _ => Err("expected none, template or function")?,
        }
    }
}

/// Specification of a paper.
#[derive(Debug, Copy, Clone)]
pub struct Paper {
    /// The width of the paper in millimeters.
    width: f64,
    /// The height of the paper in millimeters.
    height: f64,
}

impl Paper {
    /// The width of the paper.
    pub fn width(self) -> Length {
        Length::mm(self.width)
    }

    /// The height of the paper.
    pub fn height(self) -> Length {
        Length::mm(self.height)
    }
}

/// Defines paper constants and a paper parsing implementation.
macro_rules! papers {
    ($(($var:ident: $width:expr, $height: expr, $($pats:tt)*))*) => {
        /// Predefined papers.
        ///
        /// Each paper is parsable from its name in kebab-case.
        impl Paper {
            $(pub const $var: Self = Self { width: $width, height: $height };)*
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
    (NEWSPAPER_COMPACT:    280.0,    430.0,    "newspaper-compact")
    (NEWSPAPER_BERLINER:   315.0,    470.0,    "newspaper-berliner")
    (NEWSPAPER_BROADSHEET: 381.0,    578.0,    "newspaper-broadsheet")
    (PRESENTATION_16_9:    297.0, 167.0625, "presentation-16-9")
    (PRESENTATION_4_3:     280.0,    210.0,    "presentation-4-3")
}

castable! {
    Paper,
    Expected: "string",
    Value::Str(string) => Paper::from_str(&string).map_err(|e| e.to_string())?,
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
