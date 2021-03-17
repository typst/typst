//! Predefined papers.

use crate::geom::{Length, Linear, Relative, Sides, Size};

/// Specification of a paper.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Paper {
    /// The broad class this paper belongs to.
    pub class: PaperClass,
    /// The width of the paper in millimeters.
    pub width: f64,
    /// The height of the paper in millimeters.
    pub height: f64,
}

impl Paper {
    /// The paper with the given name.
    pub fn from_name(name: &str) -> Option<Self> {
        parse_paper(name)
    }

    /// The size of the paper.
    pub fn size(self) -> Size {
        Size::new(Length::mm(self.width), Length::mm(self.height))
    }
}

/// Defines default margins for a class of related papers.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PaperClass {
    Custom,
    Base,
    US,
    Newspaper,
    Book,
}

impl PaperClass {
    /// The default margins for this page class.
    pub fn default_margins(self) -> Sides<Linear> {
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

macro_rules! papers {
    ($(($var:ident: $class:ident, $width:expr, $height: expr, $($pats:tt)*))*) => {
        $(papers!(@$var, stringify!($($pats)*), $class, $width, $height);)*

        fn parse_paper(paper: &str) -> Option<Paper> {
            match paper.to_lowercase().as_str() {
                $($($pats)* => Some($var),)*
                _ => None,
            }
        }
    };

    (@$var:ident, $names:expr, $class:ident, $width:expr, $height:expr) => {
        #[doc = "Paper with name `"]
        #[doc = $names]
        #[doc = "`."]
        pub const $var: Paper = Paper {
            class: PaperClass::$class,
            width: $width,
            height: $height,
        };
    };
}

// All paper sizes in mm.
papers! {
    // ---------------------------------------------------------------------- //
    // ISO 216

    // A Series
    (PAPER_A0:  Base, 841.0, 1189.0, "a0" | "poster")
    (PAPER_A1:  Base, 594.0, 841.0,  "a1")
    (PAPER_A2:  Base, 420.0, 594.0,  "a2")
    (PAPER_A3:  Base, 297.0, 420.0,  "a3")
    (PAPER_A4:  Base, 210.0, 297.0,  "a4")
    (PAPER_A5:  Base, 148.0, 210.0,  "a5")
    (PAPER_A6:  Book, 105.0, 148.0,  "a6")
    (PAPER_A7:  Base, 74.0,  105.0,  "a7" | "iso-7810-id-2" | "id-2" | "visa" | "flyer")
    (PAPER_A8:  Base, 52.0,  74.0,   "a8")
    (PAPER_A9:  Base, 37.0,  52.0,   "a9")
    (PAPER_A10: Base, 26.0,  37.0,   "a10")
    (PAPER_A11: Base, 18.0,  26.0,   "a11")

    // B Series
    (PAPER_B1: Base, 707.0, 1000.0, "b1" | "flipchart")
    (PAPER_B2: Base, 500.0, 707.0,  "b2")
    (PAPER_B3: Base, 353.0, 500.0,  "b3")
    (PAPER_B4: Base, 250.0, 353.0,  "b4" | "sheet-music")
    (PAPER_B5: Book, 176.0, 250.0,  "b5")
    (PAPER_B6: Book, 125.0, 176.0,  "b6" | "book")
    (PAPER_B7: Base, 88.0,  125.0,  "b7" | "passport" | "iso-7810-id-3" | "id-3")
    (PAPER_B8: Base, 62.0,  88.0,   "b8")

    // C Series
    (PAPER_C3: Base, 324.0, 458.0, "c3")
    (PAPER_C4: Base, 229.0, 324.0, "c4")
    (PAPER_C5: Base, 162.0, 229.0, "c5")
    (PAPER_C6: Base, 114.0, 162.0, "c6")
    (PAPER_C7: Base, 81.0, 114.0,  "c7")
    (PAPER_C8: Base, 57.0, 81.0,   "c8")

    // D Series (DIN extension to ISO)
    (PAPER_D3: Base, 272.0, 385.0, "din-d3")
    (PAPER_D4: Base, 192.0, 272.0, "din-d4")
    (PAPER_D5: Base, 136.0, 192.0, "din-d5" | "dvd")
    (PAPER_D6: Base, 96.0, 136.0,  "din-d6")
    (PAPER_D7: Base, 68.0,  96.0,  "din-d7")
    (PAPER_D8: Base, 48.0,  68.0,  "din-d8")

    // Academically relevant SIS extensions
    (PAPER_G5: Base, 169.0, 239.0, "sis-g5")
    (PAPER_E5: Base, 115.0, 220.0, "sis-e5")

    // ---------------------------------------------------------------------- //
    // Unites States

    // Customary
    (PAPER_FOLIO:             US,        210.0, 330.0, "folio" | "us-folio" | "us-f4")
    (PAPER_LETTER:            US,        216.0, 279.0, "letter" | "ansi-a" |
                                                       "american-quarto" | "carta")
    (PAPER_LEGAL:             US,        216.0, 356.0, "legal")
    (PAPER_TABLOID:           Newspaper, 279.0, 432.0, "tabloid" | "ansi-b")
    (PAPER_LEDGER:            Base,      432.0, 279.0, "ledger")
    (PAPER_JUNIOR_LEGAL:      US,        127.0, 203.0, "junior-legal" | "index-card")
    (PAPER_HALF_LETTER:       Base,      140.0, 216.0, "half-letter")
    (PAPER_GOVERNMENT_LETTER: US,        203.0, 267.0, "government-letter")
    (PAPER_GOVERNMENT_LEGAL:  US,        216.0, 330.0, "government-legal" | "officio")

    // ANSI Extensions
    (PAPER_ANSI_C:        Base, 432.0, 559.0,  "ansi-c")
    (PAPER_ANSI_D:        Base, 559.0, 864.0,  "ansi-d")
    (PAPER_ANSI_E:        Base, 864.0, 1118.0, "ansi-e")
    (PAPER_ENGINEERING_F: Base, 711.0, 1016.0, "engineering-f" | "engineering" |
                                               "navfac" | "aerospace")

    // Architectural Paper
    (PAPER_ARCH_A:  Base, 229.0, 305.0,  "arch-a" | "arch-1")
    (PAPER_ARCH_B:  Base, 305.0, 457.0,  "arch-b" | "arch-2" | "extra-tabloide")
    (PAPER_ARCH_C:  Base, 457.0, 610.0,  "arch-c" | "arch-3")
    (PAPER_ARCH_D:  Base, 610.0, 914.0,  "arch-d" | "arch-4")
    (PAPER_ARCH_E1: Base, 762.0, 1067.0, "arch-e1" | "arch-5")
    (PAPER_ARCH_E:  Base, 914.0, 1219.0, "arch-e" | "arch-6")

    // ---------------------------------------------------------------------- //
    // Japan

    // JIS B Series
    (PAPER_JIS_B0:  Base, 1030.0, 1456.0, "jis-b0" | "jb0")
    (PAPER_JIS_B1:  Base, 728.0, 1030.0,  "jis-b1" | "jb1")
    (PAPER_JIS_B2:  Base, 515.0, 728.0,   "jis-b2" | "jb2")
    (PAPER_JIS_B3:  Base, 364.0, 515.0,   "jis-b3" | "jb3")
    (PAPER_JIS_B4:  Base, 257.0, 364.0,   "jis-b4" | "jb4")
    (PAPER_JIS_B5:  Base, 182.0, 257.0,   "jis-b5" | "jb5")
    (PAPER_JIS_B6:  Base, 128.0, 182.0,   "jis-b6" | "jb6")
    (PAPER_JIS_B7:  Base, 91.0,  128.0,   "jis-b7" | "jb7")
    (PAPER_JIS_B8:  Base, 64.0,  91.0,    "jis-b8" | "jb8")
    (PAPER_JIS_B9:  Base, 45.0,  64.0,    "jis-b9" | "jb9")
    (PAPER_JIS_B10: Base, 32.0,  45.0,    "jis-b10" | "jb10")
    (PAPER_JIS_B11: Base, 22.0,  32.0,    "jis-b11" | "jb11")

    // Traditional
    (PAPER_SHIROKU_BAN_4: Base, 264.0, 379.0, "shiroku-ban-4")
    (PAPER_SHIROKU_BAN_5: Base, 189.0, 262.0, "shiroku-ban-5")
    (PAPER_SHIROKU_BAN_6: Base, 127.0, 188.0, "shiroku-ban-6")
    (PAPER_KIKU_4:        Base, 227.0, 306.0, "kiku-4")
    (PAPER_KIKU_5:        Base, 151.0, 227.0, "kiku-5")

    // ---------------------------------------------------------------------- //
    // China

    // Chinese D Series
    (PAPER_SAC_D0: Base, 764.0, 1064.0, "sac-d0" | "cn-d0")
    (PAPER_SAC_D1: Base, 532.0, 760.0,  "sac-d1" | "cn-d1")
    (PAPER_SAC_D2: Base, 380.0, 528.0,  "sac-d2" | "cn-d2")
    (PAPER_SAC_D3: Base, 264.0, 376.0,  "sac-d3" | "cn-d3")
    (PAPER_SAC_D4: Base, 188.0, 260.0,  "sac-d4" | "cn-d4")
    (PAPER_SAC_D5: Base, 130.0, 184.0,  "sac-d5" | "cn-d5")
    (PAPER_SAC_D6: Base, 92.0, 126.0,   "sac-d6" | "cn-d6")

    // ---------------------------------------------------------------------- //
    // United Kingdom Imperial (Assortment)

    (PAPER_MONARCH:      Base, 184.0, 267.0,  "monarch")
    (PAPER_QUARTO:       Base, 229.0, 279.0,  "quarto" | "us-quarto")
    (PAPER_UK_QUARTO:    Base, 203.0, 254.0,  "uk-quarto" | "imperial-quarto")
    (PAPER_UK_FOOLSCAP:  Base, 343.0, 432.0,  "foolscap" | "us-foolscap")
    (PAPER_FOOLSCAP:     Base, 203.0, 330.0,  "imperial-foolscap" | "uk-foolscap")
    (PAPER_POTT:         Base, 318.0, 381.0,  "pott")
    (PAPER_CROWN:        Base, 318.0, 508.0,  "crown")
    (PAPER_PINCHED_POST: Base, 375.0, 470.0,  "pinched-post")
    (PAPER_POST:         Base, 394.0, 489.0,  "post")
    (PAPER_LARGE_POST:   Base, 419.0, 533.0,  "large-post")
    (PAPER_DEMY:         Base, 445.0, 572.0,  "demy")
    (PAPER_ROYAL:        Base, 508.0, 635.0,  "royal")
    (PAPER_DOUBLE_CROWN: Base, 508.0, 762.0,  "double-crown" | "theatre")
    (PAPER_ELEPHANT:     Base, 584.0, 711.0,  "elephant")
    (PAPER_DOUBLE_ROYAL: Base, 635.0, 1016.0, "double-royal" | "rail")
    (PAPER_QUAD_CROWN:   Base, 762.0, 1016.0, "quad-crown" | "cinema")

    // ---------------------------------------------------------------------- //
    // French Traditional (AFNOR)

    (PAPER_CLOCHE:               Base, 300.0, 400.0,   "cloche")
    (PAPER_POT:                  Base, 310.0, 400.0,   "pot" | "ecolier" | "écolier")
    (PAPER_TELLIERE:             Base, 340.0, 440.0,   "telliere" | "tellière")
    (PAPER_COURONNE_ECRITURE:    Base, 360.0, 460.0,   "couronne-ecriture" |
                                                       "couronne" | "couronne-écriture")
    (PAPER_COURONNE_EDITION:     Base, 370.0, 470.0,   "couronne-edition" |
                                                       "couronne-édition")
    (PAPER_ROBERTO:              Base, 390.0, 500.0,   "roberto")
    (PAPER_ECU:                  Base, 400.0, 520.0,   "ecu" | "écu")
    (PAPER_COQUILLE:             Base, 440.0, 560.0,   "coquille")
    (PAPER_CARRE:                Base, 450.0, 560.0,   "carre" | "carré")
    (PAPER_CAVALIER:             Base, 460.0, 620.0,   "cavalier")
    (PAPER_DEMI_RAISIN:          Base, 325.0, 500.0,   "demi-raisin")
    (PAPER_RAISIN:               Base, 500.0, 650.0,   "raisin" | "dessin")
    (PAPER_DOUBLE_RAISIN:        Base, 650.0, 1000.0,  "double-raisin")
    (PAPER_JESUS:                Base, 560.0, 760.0,   "jesus" | "jésus")
    (PAPER_SOLEIL:               Base, 600.0, 800.0,   "soleil")
    (PAPER_COLOMBIER_AFFICHE:    Base, 600.0, 800.0,   "colombier-affiche" | "affiche")
    (PAPER_COLOMBIER_COMMERCIAL: Base, 630.0, 900.0,   "colombier-commercial")
    (PAPER_PETIT_AIGLE:          Base, 700.0, 940.0,   "petit-aigle")
    (PAPER_GRAND_AIGLE:          Base, 750.0, 1060.0,  "grand-aigle" | "napoleon")
    (PAPER_GRAND_MONDE:          Base, 900.0, 1260.0,  "grand-monde")
    (PAPER_UNIVERS:              Base, 1000.0, 1300.0, "univers" | "universe")

    // ---------------------------------------------------------------------- //
    // Newspaper

    (PAPER_COMPACT:        Newspaper, 280.0, 430.0, "compact")
    (PAPER_BERLINER:       Newspaper, 315.0, 470.0, "berliner" | "midi")
    (PAPER_RHENISH:        Newspaper, 350.0, 520.0, "rhenish")
    (PAPER_BROADSHEET:     Newspaper, 381.0, 578.0, "broadsheet" | "newspaper")
    (PAPER_NEW_YORK_TIMES: Newspaper, 305.0, 559.0, "new-york-times" | "times")

    // ---------------------------------------------------------------------- //
    // Books

    (PAPER_FOLIO_BOOK:  Book, 304.8, 482.6,  "book-folio")
    (PAPER_QUARTO_BOOK: Book, 241.3, 304.8,  "book-quarto")
    (PAPER_OCTAVO_BOOK: Book, 152.4, 228.6,  "book-octavo")
    (PAPER_16_MO_BOOK:  Book, 101.6, 171.45, "book-16mo")
    (PAPER_32_MO_BOOK:  Book, 88.9, 139.7,   "book-32mo")

    // ---------------------------------------------------------------------- //
    // Various

    (PAPER_ID_1:             Base, 85.6,  53.98,    "id-card" | "id-1" | "iso-7810-id-1" |
                                                    "eu-business-card" | "business-card")
    (PAPER_US_BUSINESS_CARD: Base, 88.9,  50.8,     "us-business-card")
    (PAPER_JP_BUSINESS_CARD: Base, 91.0,  55.0,     "jp-business-card")
    (PAPER_CN_BUSINESS_CARD: Base, 90.0,  54.0,     "cn-business-card")
    (PAPER_A4_16_9:          Base, 297.0, 167.0625, "presentation-16-9" | "presentation")
    (PAPER_A4_4_3:           Base, 280.0, 210.0,    "presentation-4-3")
    (PAPER_POSTCARD:         Base, 152.4, 101.6,    "postcard")
}
